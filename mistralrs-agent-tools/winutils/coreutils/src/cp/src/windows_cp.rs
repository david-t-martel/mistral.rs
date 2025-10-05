//! Windows-optimized copy implementation
//!
//! This module provides the main Windows-optimized copy functionality,
//! including option parsing and coordination of copy operations.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use clap::ArgMatches;
use rayon::prelude::*;
use uucore::error::{UResult, UError, USimpleError};
use uucore::show_error;

use crate::copy_engine::CopyEngine;
use crate::file_attributes::WindowsFileAttributes;
use crate::junction_handler::JunctionHandler;
use crate::progress::ProgressReporter;

/// Windows-specific copy options
#[derive(Debug, Clone)]
pub struct WindowsCpOptions {
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub recursive: bool,
    pub preserve_all: bool,
    pub preserve_timestamps: bool,
    pub preserve_ownership: bool,
    pub preserve_permissions: bool,
    pub preserve_links: bool,
    pub preserve_security: bool,
    pub preserve_streams: bool,
    pub follow_junctions: bool,
    pub preserve_junctions: bool,
    pub force: bool,
    pub interactive: bool,
    pub no_clobber: bool,
    pub verbose: bool,
    pub progress: bool,
    pub unbuffered: bool,
    pub parallel_threads: Option<usize>,
    pub target_directory: Option<PathBuf>,
    pub no_target_directory: bool,
    pub update: bool,
    pub backup: Option<String>,
    pub dereference: bool,
    pub no_dereference: bool,
    pub strip_trailing_slashes: bool,
}

impl WindowsCpOptions {
    /// Parse command line arguments into Windows copy options
    pub fn from_matches(matches: &ArgMatches) -> Result<Self, String> {
        let files: Vec<_> = matches
            .get_many::<String>("FILES")
            .unwrap_or_default()
            .collect();

        if files.is_empty() {
            return Err("missing file operand".to_string());
        }

        let (sources, destination) = if let Some(target_dir) = matches.get_one::<String>("target-directory") {
            let target_path = PathBuf::from(target_dir);
            let sources = files.into_iter().map(|s| PathBuf::from(s)).collect();
            (sources, target_path)
        } else if matches.get_flag("no-target-directory") {
            if files.len() != 2 {
                return Err("exactly two arguments required when using -T".to_string());
            }
            let sources = vec![PathBuf::from(files[0])];
            let destination = PathBuf::from(files[1]);
            (sources, destination)
        } else {
            if files.len() < 2 {
                return Err("missing destination file operand".to_string());
            }
            let mut file_paths: Vec<PathBuf> = files.into_iter().map(|s| PathBuf::from(s)).collect();
            let destination = file_paths.pop().unwrap();
            (file_paths, destination)
        };

        // Handle archive mode (-a)
        let archive_mode = matches.get_flag("archive");
        let preserve_spec = matches.get_one::<String>("preserve");

        let (preserve_timestamps, preserve_ownership, preserve_permissions, preserve_links) =
            if archive_mode {
                (true, true, true, true)
            } else if let Some(spec) = preserve_spec {
                parse_preserve_spec(spec)
            } else if matches.get_flag("preserve") {
                (true, true, true, false)
            } else {
                (false, false, false, false)
            };

        let parallel_threads = matches
            .get_one::<String>("parallel")
            .and_then(|s| s.parse().ok())
            .or_else(|| {
                if sources.len() > 1 {
                    Some(num_cpus::get().min(sources.len()))
                } else {
                    None
                }
            });

        Ok(WindowsCpOptions {
            sources,
            destination,
            recursive: matches.get_flag("recursive"),
            preserve_all: archive_mode,
            preserve_timestamps,
            preserve_ownership,
            preserve_permissions,
            preserve_links,
            preserve_security: matches.get_flag("preserve-security"),
            preserve_streams: matches.get_flag("preserve-streams"),
            follow_junctions: matches.get_flag("follow-junctions"),
            preserve_junctions: matches.get_flag("preserve-junctions"),
            force: matches.get_flag("force"),
            interactive: matches.get_flag("interactive"),
            no_clobber: matches.get_flag("no-clobber"),
            verbose: matches.get_flag("verbose"),
            progress: matches.get_flag("progress"),
            unbuffered: matches.get_flag("unbuffered"),
            parallel_threads,
            target_directory: matches.get_one::<String>("target-directory").map(PathBuf::from),
            no_target_directory: matches.get_flag("no-target-directory"),
            update: matches.get_flag("update"),
            backup: matches.get_one::<String>("backup").map(String::from),
            dereference: matches.get_flag("dereference"),
            no_dereference: matches.get_flag("no-dereference"),
            strip_trailing_slashes: matches.get_flag("strip-trailing-slashes"),
        })
    }

    /// Check if Windows optimizations should be used
    pub fn should_use_windows_optimizations(&self) -> bool {
        // Use Windows optimizations when:
        // - Preserving Windows-specific attributes
        // - Using junction handling
        // - Using progress reporting for large files
        // - Using parallel copy
        self.preserve_security
            || self.preserve_streams
            || self.follow_junctions
            || self.preserve_junctions
            || self.progress
            || self.parallel_threads.is_some()
            || self.unbuffered
    }
}

/// Parse preserve specification (e.g., "mode,timestamps,links")
fn parse_preserve_spec(spec: &str) -> (bool, bool, bool, bool) {
    let mut timestamps = false;
    let mut ownership = false;
    let mut permissions = false;
    let mut links = false;

    for attr in spec.split(',') {
        match attr.trim() {
            "mode" | "permissions" => permissions = true,
            "timestamps" | "times" => timestamps = true,
            "ownership" | "owner" => ownership = true,
            "links" => links = true,
            "all" => {
                timestamps = true;
                ownership = true;
                permissions = true;
                links = true;
            }
            _ => {} // Ignore unknown attributes
        }
    }

    (timestamps, ownership, permissions, links)
}

/// Main copy function using Windows optimizations
pub fn copy_files(options: WindowsCpOptions) -> UResult<()> {
    if !options.should_use_windows_optimizations() {
        // Fallback to standard implementation
        return Err(USimpleError::new(1, "Windows optimizations not needed"));
    }

    let copy_engine = Arc::new(CopyEngine::new(&options)?);
    let progress_reporter = Arc::new(ProgressReporter::new(options.progress, options.verbose));
    let junction_handler = Arc::new(JunctionHandler::new(
        options.follow_junctions,
        options.preserve_junctions,
    ));

    // Determine if we're copying to a directory
    let dest_is_dir = options.destination.is_dir() || options.target_directory.is_some();

    if options.sources.len() > 1 && !dest_is_dir {
        return Err(USimpleError::new(1, "target is not a directory"));
    }

    // Use parallel processing when beneficial
    if let Some(threads) = options.parallel_threads {
        if options.sources.len() > 1 && threads > 1 {
            return copy_files_parallel(options, copy_engine, progress_reporter, junction_handler);
        }
    }

    // Sequential copying
    for source in &options.sources {
        let dest_path = if dest_is_dir {
            let filename = source
                .file_name()
                .ok_or_else(|| USimpleError::new(1, "invalid source path"))?;
            options.destination.join(filename)
        } else {
            options.destination.clone()
        };

        copy_single_item(
            source,
            &dest_path,
            &options,
            &copy_engine,
            &progress_reporter,
            &junction_handler,
        )?;
    }

    Ok(())
}

/// Copy files in parallel using Rayon
fn copy_files_parallel(
    options: WindowsCpOptions,
    copy_engine: Arc<CopyEngine>,
    progress_reporter: Arc<ProgressReporter>,
    junction_handler: Arc<JunctionHandler>,
) -> UResult<()> {
    let dest_is_dir = options.destination.is_dir() || options.target_directory.is_some();

    // Configure thread pool
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.parallel_threads.unwrap_or(num_cpus::get()))
        .build()
        .map_err(|e| USimpleError::new(1, format!("failed to create thread pool: {}", e)))?;

    let results: Vec<UResult<()>> = thread_pool.install(|| {
        options.sources
            .par_iter()
            .map(|source| {
                let dest_path = if dest_is_dir {
                    let filename = source
                        .file_name()
                        .ok_or_else(|| USimpleError::new(1, "invalid source path"))?;
                    options.destination.join(filename)
                } else {
                    options.destination.clone()
                };

                copy_single_item(
                    source,
                    &dest_path,
                    &options,
                    &copy_engine,
                    &progress_reporter,
                    &junction_handler,
                )
            })
            .collect()
    });

    // Check for any errors
    for result in results {
        result?;
    }

    Ok(())
}

/// Copy a single file or directory
fn copy_single_item(
    source: &Path,
    destination: &Path,
    options: &WindowsCpOptions,
    copy_engine: &CopyEngine,
    progress_reporter: &ProgressReporter,
    junction_handler: &JunctionHandler,
) -> UResult<()> {
    if options.verbose {
        println!("'{}' -> '{}'", source.display(), destination.display());
    }

    // Check if source exists
    if !source.exists() {
        return Err(USimpleError::new(1, format!("cannot stat '{}': No such file or directory", source.display())));
    }

    // Handle update mode
    if options.update && destination.exists() {
        if let (Ok(src_meta), Ok(dest_meta)) = (source.metadata(), destination.metadata()) {
            if let (Ok(src_time), Ok(dest_time)) = (src_meta.modified(), dest_meta.modified()) {
                if src_time <= dest_time {
                    return Ok(()); // Skip, destination is newer or same age
                }
            }
        }
    }

    // Handle no-clobber mode
    if options.no_clobber && destination.exists() {
        return Ok(()); // Skip existing files
    }

    // Handle interactive mode
    if options.interactive && destination.exists() {
        if !prompt_overwrite(destination)? {
            return Ok(()); // User chose not to overwrite
        }
    }

    // Check for junction points
    if junction_handler.is_junction(source)? {
        return junction_handler.copy_junction(source, destination, options);
    }

    // Handle directories
    if source.is_dir() {
        if !options.recursive {
            return Err(USimpleError::new(1, format!("omitting directory '{}'", source.display())));
        }
        return copy_directory_recursive(source, destination, options, copy_engine, progress_reporter, junction_handler);
    }

    // Copy regular file
    copy_engine.copy_file(source, destination, progress_reporter)
}

/// Copy directory recursively
fn copy_directory_recursive(
    source_dir: &Path,
    dest_dir: &Path,
    options: &WindowsCpOptions,
    copy_engine: &CopyEngine,
    progress_reporter: &ProgressReporter,
    junction_handler: &JunctionHandler,
) -> UResult<()> {
    // Create destination directory if it doesn't exist
    if !dest_dir.exists() {
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| USimpleError::new(1, format!("cannot create directory '{}': {}", dest_dir.display(), e)))?;
    }

    // Copy directory contents
    let entries = std::fs::read_dir(source_dir)
        .map_err(|e| USimpleError::new(1, format!("cannot read directory '{}': {}", source_dir.display(), e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| USimpleError::new(1, format!("error reading directory entry: {}", e)))?;

        let source_path = entry.path();
        let dest_path = dest_dir.join(entry.file_name());

        copy_single_item(
            &source_path,
            &dest_path,
            options,
            copy_engine,
            progress_reporter,
            junction_handler,
        )?;
    }

    // Copy directory attributes
    if options.preserve_timestamps || options.preserve_all {
        copy_engine.copy_directory_attributes(source_dir, dest_dir)?;
    }

    Ok(())
}

/// Prompt user for overwrite confirmation
fn prompt_overwrite(path: &Path) -> UResult<bool> {
    use std::io::{self, Write};

    print!("cp: overwrite '{}'? ", path.display());
    io::stdout().flush().map_err(|e| USimpleError::new(1, format!("failed to flush stdout: {}", e)))?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|e| USimpleError::new(1, format!("failed to read input: {}", e)))?;

    Ok(input.trim().to_lowercase().starts_with('y'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_preserve_spec() {
        let (timestamps, ownership, permissions, links) = parse_preserve_spec("mode,timestamps");
        assert!(timestamps);
        assert!(!ownership);
        assert!(permissions);
        assert!(!links);

        let (timestamps, ownership, permissions, links) = parse_preserve_spec("all");
        assert!(timestamps);
        assert!(ownership);
        assert!(permissions);
        assert!(links);
    }

    #[test]
    fn test_windows_optimizations_detection() {
        let mut options = WindowsCpOptions {
            sources: vec![],
            destination: PathBuf::new(),
            recursive: false,
            preserve_all: false,
            preserve_timestamps: false,
            preserve_ownership: false,
            preserve_permissions: false,
            preserve_links: false,
            preserve_security: true, // This should trigger Windows optimizations
            preserve_streams: false,
            follow_junctions: false,
            preserve_junctions: false,
            force: false,
            interactive: false,
            no_clobber: false,
            verbose: false,
            progress: false,
            unbuffered: false,
            parallel_threads: None,
            target_directory: None,
            no_target_directory: false,
            update: false,
            backup: None,
            dereference: false,
            no_dereference: false,
            strip_trailing_slashes: false,
        };

        assert!(options.should_use_windows_optimizations());

        options.preserve_security = false;
        options.progress = true;
        assert!(options.should_use_windows_optimizations());
    }
}
