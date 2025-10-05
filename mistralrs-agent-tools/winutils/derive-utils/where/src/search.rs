// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! High-performance search engine for the where utility

use crate::args::Args;
use crate::cache::PathCache;
use crate::error::{WhereError, WhereResult};
use crate::output::{OutputFormatter, SearchResult};
use crate::pathext;
use crate::wildcard::MultiPatternMatcher;
use winpath::normalize_path;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// High-performance search engine for finding executables
pub struct SearchEngine {
    /// Command line arguments
    args: Args,
    /// Pattern matchers
    matchers: MultiPatternMatcher,
    /// PATH cache for performance
    cache: PathCache,
    /// Output formatter
    formatter: OutputFormatter,
    /// Whether to stop on first match (for performance)
    stop_on_first: bool,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new(args: Args) -> WhereResult<Self> {
        args.validate().map_err(|e| WhereError::path(e))?;

        // Expand patterns with PATHEXT extensions if needed
        let expanded_patterns = Self::expand_patterns(&args.patterns);

        let matchers = MultiPatternMatcher::new(&expanded_patterns)?;
        let cache = PathCache::default();
        let formatter = OutputFormatter::new(args.clone());

        Ok(Self {
            args,
            matchers,
            cache,
            formatter,
            stop_on_first: true, // Default behavior like Windows where.exe
        })
    }

    /// Perform the search operation
    pub fn search(&mut self) -> WhereResult<Vec<SearchResult>> {
        if self.args.is_recursive() {
            self.search_recursive()
        } else {
            self.search_path()
        }
    }

    /// Search in PATH directories
    fn search_path(&mut self) -> WhereResult<Vec<SearchResult>> {
        let path_dirs = self.cache.get_path_dirs()?;
        let found = Arc::new(AtomicBool::new(false));
        let results = Arc::new(parking_lot::Mutex::new(Vec::new()));

        // Search across PATH directories
        #[cfg(feature = "parallel")]
        let search_result = path_dirs
            .par_iter()
            .try_for_each(|dir| -> WhereResult<()> {
                // Early termination if we found something and stop_on_first is true
                if self.stop_on_first && found.load(Ordering::Relaxed) {
                    return Ok(());
                }

                let dir_results = self.search_directory(dir)?;

                if !dir_results.is_empty() {
                    found.store(true, Ordering::Relaxed);
                    let mut results_guard = results.lock();
                    results_guard.extend(dir_results);
                }

                Ok(())
            });

        #[cfg(not(feature = "parallel"))]
        let search_result = path_dirs
            .iter()
            .try_for_each(|dir| -> WhereResult<()> {
                // Early termination if we found something and stop_on_first is true
                if self.stop_on_first && found.load(Ordering::Relaxed) {
                    return Ok(());
                }

                let dir_results = self.search_directory(dir)?;

                if !dir_results.is_empty() {
                    found.store(true, Ordering::Relaxed);
                    let mut results_guard = results.lock();
                    results_guard.extend(dir_results);
                }

                Ok(())
            });

        search_result?;

        let final_results = Arc::try_unwrap(results)
            .map_err(|_| WhereError::cache("Failed to extract results"))?
            .into_inner();

        // Print results
        self.formatter.print_results(&final_results)?;
        if !self.args.quiet {
            self.formatter.print_summary(&final_results, &self.args.patterns)?;
        }
        self.formatter.flush().map_err(WhereError::Io)?;

        Ok(final_results)
    }

    /// Search recursively from a specified directory
    fn search_recursive(&mut self) -> WhereResult<Vec<SearchResult>> {
        let search_root = self.args.get_search_root()
            .ok_or_else(|| WhereError::path("No search root specified for recursive search"))?;

        // Normalize the search root path to handle Git Bash mangled paths
        let normalized_root = normalize_path(search_root)
            .map_err(|e| WhereError::path(format!("Invalid search root path '{}': {}", search_root, e)))?;

        let root_path = Path::new(&normalized_root);
        if !root_path.exists() {
            return Err(WhereError::path(format!("Directory '{}' does not exist", normalized_root)));
        }

        let all_files = self.cache.get_recursive_files(root_path)?;
        let results = self.filter_files_parallel(&all_files);

        // Print results
        self.formatter.print_results(&results)?;
        if !self.args.quiet {
            self.formatter.print_summary(&results, &self.args.patterns)?;
        }
        self.formatter.flush().map_err(WhereError::Io)?;

        Ok(results)
    }

    /// Search a single directory for matching files
    fn search_directory(&self, dir: &Path) -> WhereResult<Vec<SearchResult>> {
        let files = self.cache.get_directory_files(dir)?;
        Ok(self.filter_files_parallel(&files))
    }

    /// Filter files using parallel processing
    fn filter_files_parallel(&self, files: &[PathBuf]) -> Vec<SearchResult> {
        let collect_metadata = self.args.show_time;

        #[cfg(feature = "parallel")]
        let iter = files.par_iter();
        #[cfg(not(feature = "parallel"))]
        let iter = files.iter();

        iter.filter_map(|file_path| {
                // Check if any pattern matches
                if let Some(filename) = file_path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if self.matchers.matches_any(filename_str) {
                            let matched_patterns = self.matchers.matching_patterns(filename_str);
                            let matched_pattern = matched_patterns
                                .first()
                                .unwrap_or(&"unknown")
                                .to_string();

                            if collect_metadata {
                                if let Ok(metadata) = std::fs::metadata(file_path) {
                                    return Some(SearchResult::with_metadata(
                                        file_path.clone(),
                                        matched_pattern,
                                        metadata,
                                    ));
                                }
                            }

                            return Some(SearchResult::new(file_path.clone(), matched_pattern));
                        }
                    }
                }
                None
            })
            .collect()
    }

    /// Expand patterns with PATHEXT extensions if needed
    fn expand_patterns(patterns: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();

        for pattern in patterns {
            if pathext::pattern_matches_executables(pattern) {
                // Add the original pattern
                expanded.push(pattern.clone());

                // If the pattern doesn't have an extension, add versions with extensions
                if !pattern.contains('.') {
                    for ext in pathext::get_executable_extensions() {
                        expanded.push(format!("{}{}", pattern, ext.to_lowercase()));
                    }
                }
            } else {
                // Non-executable pattern, just add as-is
                expanded.push(pattern.clone());
            }
        }

        expanded
    }

    /// Set whether to stop on first match
    pub fn set_stop_on_first(&mut self, stop: bool) {
        self.stop_on_first = stop;
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_args(patterns: Vec<String>) -> Args {
        Args {
            patterns,
            recursive_dir: None,
            quiet: true, // Suppress output in tests
            full_path: false,
            show_time: false,
        }
    }

    #[test]
    fn test_search_engine_creation() {
        let args = create_test_args(vec!["test.exe".to_string()]);
        let engine = SearchEngine::new(args);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_pattern_expansion() {
        let patterns = vec!["python".to_string(), "test.exe".to_string()];
        let expanded = SearchEngine::expand_patterns(&patterns);

        // Should have original patterns plus expanded versions
        assert!(expanded.len() >= patterns.len());
        assert!(expanded.contains(&"python".to_string()));
        assert!(expanded.contains(&"test.exe".to_string()));

        // Should have python with extensions
        assert!(expanded.iter().any(|p| p.starts_with("python") && p.contains(".exe")));
    }

    #[test]
    fn test_directory_search() {
        let temp = TempDir::new().unwrap();
        let exe_file = temp.path().join("test.exe");
        let txt_file = temp.path().join("test.txt");

        fs::write(&exe_file, "").unwrap();
        fs::write(&txt_file, "").unwrap();

        let args = create_test_args(vec!["*.exe".to_string()]);
        let engine = SearchEngine::new(args).unwrap();

        let results = engine.search_directory(temp.path()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("test.exe"));
    }

    #[test]
    fn test_recursive_search() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let exe_file = subdir.join("program.exe");
        fs::write(&exe_file, "").unwrap();

        let mut args = create_test_args(vec!["*.exe".to_string()]);
        args.recursive_dir = Some(temp.path().to_string_lossy().to_string());

        let mut engine = SearchEngine::new(args).unwrap();
        let results = engine.search().unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("program.exe"));
    }

    #[test]
    fn test_stop_on_first() {
        let args = create_test_args(vec!["test".to_string()]);
        let mut engine = SearchEngine::new(args).unwrap();

        assert!(engine.stop_on_first); // Default should be true

        engine.set_stop_on_first(false);
        assert!(!engine.stop_on_first);
    }

    #[test]
    fn test_cache_operations() {
        let args = create_test_args(vec!["test".to_string()]);
        let mut engine = SearchEngine::new(args).unwrap();

        let stats_before = engine.cache_stats();

        engine.clear_cache();

        let stats_after = engine.cache_stats();
        assert_eq!(stats_after.dir_cache_entries, 0);
        assert_eq!(stats_after.file_cache_entries, 0);
    }

    #[test]
    fn test_filter_files_parallel() {
        let temp = TempDir::new().unwrap();
        let exe1 = temp.path().join("test1.exe");
        let exe2 = temp.path().join("test2.exe");
        let txt_file = temp.path().join("document.txt");

        fs::write(&exe1, "").unwrap();
        fs::write(&exe2, "").unwrap();
        fs::write(&txt_file, "").unwrap();

        let files = vec![exe1, exe2, txt_file];
        let args = create_test_args(vec!["*.exe".to_string()]);
        let engine = SearchEngine::new(args).unwrap();

        let results = engine.filter_files_parallel(&files);
        assert_eq!(results.len(), 2);
    }
}
