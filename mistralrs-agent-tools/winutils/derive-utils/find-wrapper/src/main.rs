//! # Enhanced Find Utility for Windows
//!
//! A high-performance find replacement that combines fd's speed with GNU find compatibility
//! and universal Windows path support via winpath integration.
//!
//! ## Features
//! - fd-like performance with parallel directory traversal
//! - GNU find command-line compatibility
//! - Universal path normalization (DOS, WSL, Cygwin, Git Bash)
//! - Windows-specific file attributes support
//! - NTFS alternate data streams detection
//! - Junction and symbolic link handling
//! - Gitignore-style filtering support

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use rayon::prelude::*;
use walkdir::{DirEntry, WalkDir};
use regex::Regex;
use ignore::{WalkBuilder, WalkState};
use winpath::{normalize_path, PathNormalizer};

mod filters;
mod output;
mod options;
mod windows;

use filters::*;
use output::*;
use options::*;
use windows::*;

/// Main entry point for the enhanced find utility
fn main() -> Result<()> {
    // Initialize error handling
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("find: fatal error: {}", panic_info);
        std::process::exit(2);
    }));

    let app = build_cli();
    let matches = app.get_matches();

    // Parse options from command line arguments
    let options = FindOptions::from_matches(&matches)?;

    // Initialize path normalizer for winpath integration
    let normalizer = Arc::new(PathNormalizer::new());

    // Execute find operation
    execute_find(options, normalizer)
}

/// Build the command-line interface with GNU find compatibility
fn build_cli() -> Command {
    Command::new("find")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Find files and directories with enhanced Windows support")
        .long_about("A high-performance find replacement with GNU find compatibility,\n\
                     universal Windows path support (DOS/WSL/Cygwin/Git Bash), and\n\
                     fd-like parallel directory traversal for optimal performance.")
        .arg(Arg::new("paths")
            .help("Starting paths for search (default: current directory)")
            .action(ArgAction::Append)
            .value_name("PATH"))
        .arg(Arg::new("name")
            .short('n')
            .long("name")
            .help("Find files matching pattern")
            .value_name("PATTERN"))
        .arg(Arg::new("iname")
            .long("iname")
            .help("Find files matching pattern (case-insensitive)")
            .value_name("PATTERN"))
        .arg(Arg::new("type")
            .short('t')
            .long("type")
            .help("File type: f=file, d=directory, l=symlink, p=pipe, s=socket, b=block, c=char")
            .value_name("TYPE"))
        .arg(Arg::new("size")
            .long("size")
            .help("File size filter (+/-/exact size with k/M/G suffix)")
            .value_name("SIZE"))
        .arg(Arg::new("newer")
            .long("newer")
            .help("Files newer than reference file")
            .value_name("FILE"))
        .arg(Arg::new("older")
            .long("older")
            .help("Files older than reference file")
            .value_name("FILE"))
        .arg(Arg::new("maxdepth")
            .long("maxdepth")
            .help("Maximum directory depth")
            .value_name("LEVELS"))
        .arg(Arg::new("mindepth")
            .long("mindepth")
            .help("Minimum directory depth")
            .value_name("LEVELS"))
        .arg(Arg::new("hidden")
            .short('H')
            .long("hidden")
            .help("Include hidden files and directories")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("follow")
            .short('L')
            .long("follow")
            .help("Follow symbolic links")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("ignore-case")
            .short('i')
            .long("ignore-case")
            .help("Case-insensitive matching")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("parallel")
            .short('j')
            .long("parallel")
            .help("Number of parallel threads (default: CPU count)")
            .value_name("THREADS"))
        .arg(Arg::new("color")
            .long("color")
            .help("When to use color: auto, always, never")
            .value_name("WHEN")
            .default_value("auto"))
        .arg(Arg::new("print0")
            .long("print0")
            .help("Separate output with null characters")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("count")
            .short('c')
            .long("count")
            .help("Only show count of matching files")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("exec")
            .long("exec")
            .help("Execute command on each match")
            .value_name("COMMAND")
            .action(ArgAction::Append))
        // Windows-specific options
        .arg(Arg::new("windows-attributes")
            .long("windows-attributes")
            .help("Show Windows file attributes (H=Hidden, S=System, R=ReadOnly, A=Archive)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("ntfs-streams")
            .long("ntfs-streams")
            .help("Include NTFS alternate data streams")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("junctions")
            .long("junctions")
            .help("Include junction points and reparse points")
            .action(ArgAction::SetTrue))
        // Path format options
        .arg(Arg::new("path-format")
            .long("path-format")
            .help("Output path format: windows, unix, native, auto")
            .value_name("FORMAT")
            .default_value("auto"))
}

/// Execute the find operation with the given options
fn execute_find(options: FindOptions, normalizer: Arc<PathNormalizer>) -> Result<()> {
    // Normalize starting paths using winpath
    let starting_paths = if options.paths.is_empty() {
        vec![normalize_current_directory(&normalizer)?]
    } else {
        options.paths.iter()
            .map(|p| normalize_search_path(p, &normalizer))
            .collect::<Result<Vec<_>>>()?
    };

    // Initialize output formatter
    let formatter = OutputFormatter::new(&options);

    // Set up parallel execution
    let thread_count = options.parallel_threads.unwrap_or_else(num_cpus::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()
        .context("Failed to initialize thread pool")?;

    let mut total_matches = 0u64;
    let results = Arc::new(crossbeam_channel::unbounded());
    let (sender, receiver) = (results.0.clone(), results.1.clone());

    // Process each starting path
    for start_path in starting_paths {
        let matches = if options.use_ignore_patterns {
            find_with_ignore(&start_path, &options, normalizer.clone(), sender.clone())?
        } else {
            find_with_walkdir(&start_path, &options, normalizer.clone(), sender.clone())?
        };
        total_matches += matches;
    }

    // Close sender to signal completion
    drop(sender);

    // Output results
    if options.count_only {
        println!("{}", total_matches);
    } else {
        // Collect and sort results if needed
        let mut results: Vec<_> = receiver.iter().collect();
        if !options.disable_sort {
            results.sort_by(|a, b| a.path.cmp(&b.path));
        }

        for result in results {
            formatter.format_entry(&result)?;
        }
    }

    Ok(())
}

/// Find files using walkdir with custom filtering
fn find_with_walkdir(
    start_path: &Path,
    options: &FindOptions,
    normalizer: Arc<PathNormalizer>,
    sender: crossbeam_channel::Sender<FindResult>,
) -> Result<u64> {
    let mut walker = WalkDir::new(start_path);

    // Configure walker based on options
    if let Some(max_depth) = options.max_depth {
        walker = walker.max_depth(max_depth);
    }
    if let Some(min_depth) = options.min_depth {
        walker = walker.min_depth(min_depth);
    }
    if options.follow_symlinks {
        walker = walker.follow_links(true);
    }

    // Create filters
    let name_filter = create_name_filter(options)?;
    let type_filter = create_type_filter(options)?;
    let size_filter = create_size_filter(options)?;
    let time_filter = create_time_filter(options)?;

    let mut count = 0u64;

    // Parallel processing of directory entries
    walker.into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>()
        .into_par_iter()
        .for_each(|entry| {
            if should_process_entry(&entry, options, &normalizer) {
                if let Ok(result) = create_find_result(entry, options, &normalizer) {
                    if apply_filters(&result, &name_filter, &type_filter, &size_filter, &time_filter) {
                        let _ = sender.send(result);
                    }
                }
            }
        });

    Ok(count)
}

/// Find files using ignore crate for gitignore-style filtering
fn find_with_ignore(
    start_path: &Path,
    options: &FindOptions,
    normalizer: Arc<PathNormalizer>,
    sender: crossbeam_channel::Sender<FindResult>,
) -> Result<u64> {
    let mut builder = WalkBuilder::new(start_path);

    // Configure ignore patterns
    builder
        .hidden(!options.include_hidden)
        .git_ignore(!options.ignore_git_ignore)
        .git_global(!options.ignore_git_ignore)
        .git_exclude(!options.ignore_git_ignore);

    if let Some(max_depth) = options.max_depth {
        builder.max_depth(Some(max_depth));
    }

    if options.follow_symlinks {
        builder.follow_links(true);
    }

    // Set thread count
    if let Some(threads) = options.parallel_threads {
        builder.threads(threads);
    }

    // Create filters
    let name_filter = create_name_filter(options)?;
    let type_filter = create_type_filter(options)?;
    let size_filter = create_size_filter(options)?;
    let time_filter = create_time_filter(options)?;

    let mut count = 0u64;

    builder.build_parallel().run(|| {
        let sender = sender.clone();
        let name_filter = name_filter.clone();
        let type_filter = type_filter.clone();
        let size_filter = size_filter.clone();
        let time_filter = time_filter.clone();
        let normalizer = normalizer.clone();
        let options = options.clone();

        Box::new(move |result| {
            match result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file() || ft.is_dir()) {
                        if let Ok(find_result) = create_find_result_from_dir_entry(entry.path(), &options, &normalizer) {
                            if apply_filters(&find_result, &name_filter, &type_filter, &size_filter, &time_filter) {
                                let _ = sender.send(find_result);
                            }
                        }
                    }
                    WalkState::Continue
                }
                Err(_) => WalkState::Continue,
            }
        })
    });

    Ok(count)
}

/// Normalize the current directory using winpath
fn normalize_current_directory(normalizer: &PathNormalizer) -> Result<PathBuf> {
    let current = std::env::current_dir()
        .context("Failed to get current directory")?;

    let normalized = normalizer
        .normalize(current.to_string_lossy().as_ref())
        .context("Failed to normalize current directory")?;

    Ok(PathBuf::from(normalized.path()))
}

/// Normalize a search path using winpath
fn normalize_search_path(path: &str, normalizer: &PathNormalizer) -> Result<PathBuf> {
    let normalized = normalizer
        .normalize(path)
        .context("Failed to normalize search path")?;

    Ok(PathBuf::from(normalized.path()))
}

/// Check if a directory entry should be processed
fn should_process_entry(entry: &DirEntry, options: &FindOptions, normalizer: &PathNormalizer) -> bool {
    // Skip hidden files unless explicitly requested
    if !options.include_hidden {
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with('.') {
                return false;
            }
        }
    }

    // Check minimum depth
    if let Some(min_depth) = options.min_depth {
        if entry.depth() < min_depth {
            return false;
        }
    }

    // Windows-specific filtering
    #[cfg(windows)]
    {
        if !options.include_hidden {
            if is_windows_hidden(entry.path()) {
                return false;
            }
        }

        if !options.include_junctions {
            if is_junction_point(entry.path()) {
                return false;
            }
        }
    }

    true
}

/// Create a FindResult from a walkdir DirEntry
fn create_find_result(entry: DirEntry, options: &FindOptions, normalizer: &PathNormalizer) -> Result<FindResult> {
    let path = entry.path();
    let metadata = entry.metadata()?;

    create_find_result_from_path(path, &metadata, options, normalizer)
}

/// Create a FindResult from a path and metadata
fn create_find_result_from_dir_entry(path: &Path, options: &FindOptions, normalizer: &PathNormalizer) -> Result<FindResult> {
    let metadata = std::fs::metadata(path)?;
    create_find_result_from_path(path, &metadata, options, normalizer)
}

/// Create a FindResult from a path and metadata
fn create_find_result_from_path(
    path: &Path,
    metadata: &std::fs::Metadata,
    options: &FindOptions,
    normalizer: &PathNormalizer,
) -> Result<FindResult> {
    // Normalize path for output
    let normalized_path = match &options.path_format {
        PathFormat::Windows => {
            normalizer.normalize(path.to_string_lossy().as_ref())?.into_owned()
        }
        PathFormat::Unix => {
            path.to_string_lossy().replace('\\', "/")
        }
        PathFormat::Native => {
            path.to_string_lossy().into_owned()
        }
        PathFormat::Auto => {
            // Auto-detect based on current environment
            if cfg!(windows) {
                normalizer.normalize(path.to_string_lossy().as_ref())?.into_owned()
            } else {
                path.to_string_lossy().into_owned()
            }
        }
    };

    let file_type = if metadata.is_file() {
        FileType::File
    } else if metadata.is_dir() {
        FileType::Directory
    } else {
        FileType::Symlink
    };

    let mut result = FindResult {
        path: normalized_path,
        file_type,
        size: metadata.len(),
        modified: metadata.modified().ok(),
        #[cfg(windows)]
        windows_attributes: if options.show_windows_attributes {
            Some(get_windows_attributes(path)?)
        } else {
            None
        },
        #[cfg(windows)]
        ntfs_streams: if options.include_ntfs_streams {
            Some(get_ntfs_streams(path)?)
        } else {
            None
        },
    };

    Ok(result)
}

/// Apply all filters to a FindResult
fn apply_filters(
    result: &FindResult,
    name_filter: &Option<NameFilter>,
    type_filter: &Option<TypeFilter>,
    size_filter: &Option<SizeFilter>,
    time_filter: &Option<TimeFilter>,
) -> bool {
    if let Some(filter) = name_filter {
        if !filter.matches(&result.path) {
            return false;
        }
    }

    if let Some(filter) = type_filter {
        if !filter.matches(result.file_type) {
            return false;
        }
    }

    if let Some(filter) = size_filter {
        if !filter.matches(result.size) {
            return false;
        }
    }

    if let Some(filter) = time_filter {
        if let Some(modified) = result.modified {
            if !filter.matches(modified) {
                return false;
            }
        }
    }

    true
}
