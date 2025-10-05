//! Search implementation for grep wrapper

use anyhow::{Context, Result};
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader, Read, Stdin, StdinLock};
use walkdir::{WalkDir, DirEntry};
use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use grep_searcher::{Searcher, SearcherBuilder};
use grep_matcher::Matcher;
use winpath::PathNormalizer;
use std::sync::Arc;

use crate::config::GrepConfig;
use crate::output::GrepPrinter;
use crate::InputFile;

/// Search a single file
pub fn search_file<M: Matcher, P: GrepPrinter>(
    input_file: &InputFile,
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    match input_file {
        InputFile::Stdin => search_stdin(config, matcher, searcher, printer),
        InputFile::Path(path) => search_path(path, config, matcher, searcher, printer),
    }
}

/// Search standard input
fn search_stdin<M: Matcher, P: GrepPrinter>(
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    let mut match_count = 0u64;
    let mut line_count = 0u64;

    searcher.search_reader(
        matcher,
        &mut stdin_lock,
        printer.sink(None, &mut match_count, &mut line_count, config)?,
    ).context("Failed to search stdin")?;

    if config.count_only {
        println!("{}", match_count);
    }

    Ok(match_count)
}

/// Search a file path
fn search_path<M: Matcher, P: GrepPrinter>(
    path: &Path,
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    // Check if path exists
    if !path.exists() {
        if !config.quiet {
            eprintln!("grep: {}: No such file or directory", path.display());
        }
        return Ok(0);
    }

    // Handle directory
    if path.is_dir() {
        if config.recursive {
            return search_directory_recursive(path, config, matcher, searcher, printer);
        } else {
            if !config.quiet {
                eprintln!("grep: {}: Is a directory", path.display());
            }
            return Ok(0);
        }
    }

    // Search regular file
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let mut match_count = 0u64;
    let mut line_count = 0u64;

    searcher.search_file(
        matcher,
        path,
        printer.sink(Some(path), &mut match_count, &mut line_count, config)?,
    ).with_context(|| format!("Failed to search file: {}", path.display()))?;

    // Handle count-only mode for individual files
    if config.count_only && config.should_show_filename() {
        if config.null_separator {
            print!("{}:{}\0", path.display(), match_count);
        } else {
            println!("{}:{}", path.display(), match_count);
        }
    } else if config.count_only {
        println!("{}", match_count);
    }

    Ok(match_count)
}

/// Search directories recursively
pub fn search_recursive<M: Matcher + Sync, P: GrepPrinter + Sync>(
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
    normalizer: &Arc<PathNormalizer>,
) -> Result<u64> {
    let starting_paths: Vec<_> = if config.files.is_empty() {
        vec![".".to_string()]
    } else {
        config.files.clone()
    };

    let mut total_matches = 0u64;

    for start_path in starting_paths {
        // Normalize the starting path
        let normalized = normalizer
            .normalize(&start_path)
            .context("Failed to normalize search path")?;
        let path = Path::new(normalized.path());

        if !path.exists() {
            if !config.quiet {
                eprintln!("grep: {}: No such file or directory", path.display());
            }
            continue;
        }

        let matches = if config.use_gitignore_filtering() {
            search_with_ignore(path, config, matcher, searcher, printer)?
        } else {
            search_with_walkdir(path, config, matcher, searcher, printer)?
        };

        total_matches += matches;
    }

    Ok(total_matches)
}

/// Search directory recursively using walkdir
fn search_directory_recursive<M: Matcher, P: GrepPrinter>(
    dir_path: &Path,
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    let mut total_matches = 0u64;

    let walker = WalkDir::new(dir_path)
        .follow_links(config.follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| should_search_entry(entry, config))
        .collect::<Vec<_>>();

    // Process files in parallel if enabled
    if config.parallel_threads.unwrap_or(1) > 1 {
        let results: Vec<_> = walker
            .into_par_iter()
            .map(|entry| {
                if entry.file_type().is_file() {
                    search_path(entry.path(), config, matcher, searcher, printer)
                        .unwrap_or(0)
                } else {
                    0
                }
            })
            .collect();

        total_matches = results.into_iter().sum();
    } else {
        // Sequential processing
        for entry in walker {
            if entry.file_type().is_file() {
                total_matches += search_path(entry.path(), config, matcher, searcher, printer)?;
            }
        }
    }

    Ok(total_matches)
}

/// Search using ignore crate (gitignore-style filtering)
fn search_with_ignore<M: Matcher + Sync, P: GrepPrinter + Sync>(
    start_path: &Path,
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    let mut builder = WalkBuilder::new(start_path);

    // Configure ignore patterns
    builder
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .hidden(false); // We'll handle hidden files manually

    if config.follow_symlinks {
        builder.follow_links(true);
    }

    // Set thread count
    if let Some(threads) = config.parallel_threads {
        builder.threads(threads);
    }

    // Add custom ignore patterns
    for pattern in &config.exclude_patterns {
        builder.add_custom_ignore_filename(pattern);
    }

    let total_matches = Arc::new(std::sync::atomic::AtomicU64::new(0));

    builder.build_parallel().run(|| {
        let total_matches = total_matches.clone();

        Box::new(move |result| {
            match result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if should_search_path(entry.path(), config) {
                            if let Ok(matches) = search_path(entry.path(), config, matcher, searcher, printer) {
                                total_matches.fetch_add(matches, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                    WalkState::Continue
                }
                Err(_) => WalkState::Continue,
            }
        })
    });

    Ok(total_matches.load(std::sync::atomic::Ordering::Relaxed))
}

/// Search using walkdir
fn search_with_walkdir<M: Matcher, P: GrepPrinter>(
    start_path: &Path,
    config: &GrepConfig,
    matcher: &M,
    searcher: &Searcher,
    printer: &P,
) -> Result<u64> {
    let mut total_matches = 0u64;

    let walker = WalkDir::new(start_path)
        .follow_links(config.follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| should_search_entry(entry, config));

    for entry in walker {
        if entry.file_type().is_file() {
            total_matches += search_path(entry.path(), config, matcher, searcher, printer)?;
        }
    }

    Ok(total_matches)
}

/// Check if a directory entry should be searched
fn should_search_entry(entry: &DirEntry, config: &GrepConfig) -> bool {
    let path = entry.path();

    // Skip directories that match exclude patterns
    if entry.file_type().is_dir() {
        for pattern in &config.exclude_dirs {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if glob::Pattern::new(pattern).map_or(false, |p| p.matches(name)) {
                    return false;
                }
            }
        }
    }

    should_search_path(path, config)
}

/// Check if a path should be searched based on include/exclude patterns
fn should_search_path(path: &Path, config: &GrepConfig) -> bool {
    let filename = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    // Check exclude patterns
    for pattern in &config.exclude_patterns {
        if glob::Pattern::new(pattern).map_or(false, |p| p.matches(filename)) {
            return false;
        }
    }

    // Check include patterns (if any)
    if !config.include_patterns.is_empty() {
        let mut included = false;
        for pattern in &config.include_patterns {
            if glob::Pattern::new(pattern).map_or(false, |p| p.matches(filename)) {
                included = true;
                break;
            }
        }
        if !included {
            return false;
        }
    }

    true
}

impl GrepConfig {
    /// Check if gitignore filtering should be used
    fn use_gitignore_filtering(&self) -> bool {
        // Use gitignore filtering if we have exclude patterns or are in a git repository
        !self.exclude_patterns.is_empty() ||
        !self.exclude_dirs.is_empty() ||
        std::env::current_dir()
            .map(|dir| dir.join(".git").exists())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_should_search_path() {
        let config = GrepConfig {
            include_patterns: vec!["*.txt".to_string()],
            exclude_patterns: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        assert!(should_search_path(Path::new("test.txt"), &config));
        assert!(!should_search_path(Path::new("test.tmp"), &config));
        assert!(!should_search_path(Path::new("test.rs"), &config)); // Not in include
    }

    #[test]
    fn test_exclude_patterns() {
        let config = GrepConfig {
            exclude_patterns: vec!["*.log".to_string(), "temp*".to_string()],
            ..Default::default()
        };

        assert!(!should_search_path(Path::new("error.log"), &config));
        assert!(!should_search_path(Path::new("temp_file.txt"), &config));
        assert!(should_search_path(Path::new("readme.txt"), &config));
    }
}
