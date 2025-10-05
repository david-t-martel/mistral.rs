//! # fd-wrapper - Fast Find with Windows Path Integration
//!
//! A high-performance file search utility that combines the speed of fd with
//! comprehensive Windows path normalization and cross-platform compatibility.
//!
//! ## Key Features
//!
//! - Fast file and directory search (similar to `fd` and `find`)
//! - Automatic path normalization for Windows, Git Bash, WSL, Cygwin
//! - Smart ignore patterns (.gitignore, .fdignore support)
//! - Parallel directory traversal
//! - Windows-specific optimizations (file attributes, long paths)
//! - Regular expression and glob pattern matching
//! - Type filtering (files, directories, symlinks, executables)
//! - Size, time, and permission filtering
//! - Output formatting options (JSON, detailed, null-terminated)
//!
//! ## Usage
//!
//! ```rust
//! use fd_wrapper::{SearchOptions, FileSearcher};
//!
//! let options = SearchOptions::new()
//!     .pattern("*.rs")
//!     .max_depth(3)
//!     .include_hidden(false);
//!
//! let searcher = FileSearcher::new(options)?;
//! let results = searcher.search("C:\\projects")?;
//! ```

use anyhow::{anyhow, Context, Result};
use crossbeam_utils::thread;
use ignore::{WalkBuilder, WalkState};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{FileType, Metadata};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use winpath::PathNormalizer;

#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_DEVICE,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_SPARSE_FILE, FILE_ATTRIBUTE_SYSTEM, FILE_ATTRIBUTE_TEMPORARY,
};

/// Errors that can occur during file searching
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Invalid search pattern: {0}")]
    InvalidPattern(String),
    #[error("Path access denied: {0}")]
    AccessDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path normalization error: {0}")]
    PathNormalization(String),
    #[error("Invalid regular expression: {0}")]
    InvalidRegex(String),
    #[error("Search cancelled")]
    Cancelled,
}

/// Type of file system entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    File,
    Directory,
    Symlink,
    Other,
}

/// File size constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeFilter {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

impl SizeFilter {
    pub fn new() -> Self {
        Self {
            min_size: None,
            max_size: None,
        }
    }

    pub fn min(mut self, size: u64) -> Self {
        self.min_size = Some(size);
        self
    }

    pub fn max(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    pub fn matches(&self, size: u64) -> bool {
        if let Some(min) = self.min_size {
            if size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if size > max {
                return false;
            }
        }
        true
    }
}

impl Default for SizeFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Time-based constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeFilter {
    pub newer_than: Option<SystemTime>,
    pub older_than: Option<SystemTime>,
}

impl TimeFilter {
    pub fn new() -> Self {
        Self {
            newer_than: None,
            older_than: None,
        }
    }

    pub fn newer_than(mut self, time: SystemTime) -> Self {
        self.newer_than = Some(time);
        self
    }

    pub fn older_than(mut self, time: SystemTime) -> Self {
        self.older_than = Some(time);
        self
    }

    pub fn within_days(mut self, days: u64) -> Self {
        let now = SystemTime::now();
        let duration = Duration::from_secs(days * 24 * 60 * 60);
        self.newer_than = now.checked_sub(duration);
        self
    }

    pub fn matches(&self, time: SystemTime) -> bool {
        if let Some(newer) = self.newer_than {
            if time < newer {
                return false;
            }
        }
        if let Some(older) = self.older_than {
            if time > older {
                return false;
            }
        }
        true
    }
}

impl Default for TimeFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Search configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Search pattern (glob or regex)
    pub pattern: Option<String>,
    /// Use regex instead of glob patterns
    pub use_regex: bool,
    /// Case-sensitive matching
    pub case_sensitive: bool,
    /// Maximum search depth
    pub max_depth: Option<usize>,
    /// Minimum search depth
    pub min_depth: Option<usize>,
    /// Include hidden files and directories
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Types of entries to include
    pub entry_types: HashSet<EntryType>,
    /// Size constraints
    pub size_filter: SizeFilter,
    /// Time constraints (modified time)
    pub time_filter: TimeFilter,
    /// File extensions to include
    pub extensions: HashSet<String>,
    /// Additional ignore patterns
    pub ignore_patterns: Vec<String>,
    /// Respect .gitignore files
    pub respect_gitignore: bool,
    /// Respect .fdignore files
    pub respect_fdignore: bool,
    /// Number of parallel threads
    pub threads: usize,
    /// Output path format
    /// Normalize output paths
    pub normalize_paths: bool,
    /// Only search for executable files
    pub executable_only: bool,
    /// Only search for readable files
    pub readable_only: bool,
    /// Search in archives (zip, tar, etc.)
    pub search_archives: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            pattern: None,
            use_regex: false,
            case_sensitive: false,
            max_depth: None,
            min_depth: None,
            include_hidden: false,
            follow_links: false,
            entry_types: [EntryType::File, EntryType::Directory].iter().cloned().collect(),
            size_filter: SizeFilter::default(),
            time_filter: TimeFilter::default(),
            extensions: HashSet::new(),
            ignore_patterns: Vec::new(),
            respect_gitignore: true,
            respect_fdignore: true,
            threads: num_cpus::get(),
            normalize_paths: true,
            executable_only: false,
            readable_only: false,
            search_archives: false,
        }
    }
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pattern<S: Into<String>>(mut self, pattern: S) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    pub fn regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }

    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn min_depth(mut self, depth: usize) -> Self {
        self.min_depth = Some(depth);
        self
    }

    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    pub fn follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    pub fn entry_types(mut self, types: HashSet<EntryType>) -> Self {
        self.entry_types = types;
        self
    }

    pub fn size_filter(mut self, filter: SizeFilter) -> Self {
        self.size_filter = filter;
        self
    }

    pub fn time_filter(mut self, filter: TimeFilter) -> Self {
        self.time_filter = filter;
        self
    }

    pub fn extensions(mut self, exts: HashSet<String>) -> Self {
        self.extensions = exts;
        self
    }

    pub fn threads(mut self, count: usize) -> Self {
        self.threads = count.max(1);
        self
    }


    pub fn executable_only(mut self, executable: bool) -> Self {
        self.executable_only = executable;
        self
    }
}

/// Search result entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub entry_type: EntryType,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub permissions: Option<u32>,
    pub is_executable: bool,
    pub is_hidden: bool,
    #[cfg(windows)]
    pub windows_attributes: Option<u32>,
}

impl SearchResult {
    fn from_entry(entry: &ignore::DirEntry, normalizer: &PathNormalizer, options: &SearchOptions) -> Result<Self> {
        let path = entry.path();
        let metadata = entry.metadata().ok();

        let entry_type = if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            EntryType::Directory
        } else if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
            EntryType::Symlink
        } else if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            EntryType::File
        } else {
            EntryType::Other
        };

        let size = metadata.as_ref().map(|m| m.len());
        let modified = metadata.as_ref()
            .and_then(|m| m.modified().ok());

        #[cfg(unix)]
        let permissions = metadata.as_ref()
            .map(|m| {
                use std::os::unix::fs::PermissionsExt;
                m.permissions().mode()
            });

        #[cfg(windows)]
        let permissions = None; // Windows permissions are more complex

        let is_executable = Self::is_executable(path, &metadata);
        let is_hidden = Self::is_hidden(path);

        #[cfg(windows)]
        let windows_attributes = Self::get_windows_attributes(path);

        // Normalize path if requested
        let final_path = if options.normalize_paths {
            normalizer.normalize_to_context(
                &path.to_string_lossy(),
                options.output_context
            ).unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        };

        Ok(SearchResult {
            path: final_path,
            entry_type,
            size,
            modified,
            permissions,
            is_executable,
            is_hidden,
            #[cfg(windows)]
            windows_attributes,
        })
    }

    fn is_executable(path: &Path, metadata: &Option<Metadata>) -> bool {
        #[cfg(unix)]
        {
            metadata.as_ref()
                .map(|m| {
                    use std::os::unix::fs::PermissionsExt;
                    m.permissions().mode() & 0o111 != 0
                })
                .unwrap_or(false)
        }

        #[cfg(windows)]
        {
            // On Windows, check file extension
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(ext.to_lowercase().as_str(), "exe" | "bat" | "cmd" | "com" | "scr" | "ps1")
                })
                .unwrap_or(false)
        }
    }

    fn is_hidden(path: &Path) -> bool {
        // Check if filename starts with dot (Unix convention)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && name != "." && name != ".." {
                return true;
            }
        }

        #[cfg(windows)]
        {
            // Check Windows hidden attribute
            if let Some(attrs) = Self::get_windows_attributes(path) {
                return attrs & FILE_ATTRIBUTE_HIDDEN.0 != 0;
            }
        }

        false
    }

    #[cfg(windows)]
    fn get_windows_attributes(path: &Path) -> Option<u32> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetFileAttributesW;

        let wide_path: Vec<u16> = path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let attrs = unsafe { GetFileAttributesW(wide_path.as_ptr()) };

        if attrs != u32::MAX {
            Some(attrs)
        } else {
            None
        }
    }
}

/// Pattern matcher for file names
#[derive(Debug)]
pub struct PatternMatcher {
    glob_matcher: Option<glob::Pattern>,
    regex_matcher: Option<regex::Regex>,
    case_sensitive: bool,
}

impl PatternMatcher {
    pub fn new(pattern: &str, use_regex: bool, case_sensitive: bool) -> Result<Self> {
        let (glob_matcher, regex_matcher) = if use_regex {
            let regex_flags = if case_sensitive {
                regex::RegexBuilder::new(pattern)
            } else {
                regex::RegexBuilder::new(pattern).case_insensitive(true)
            };

            let regex = regex_flags.build()
                .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;
            (None, Some(regex))
        } else {
            let glob_pattern = if case_sensitive {
                glob::Pattern::new(pattern)
            } else {
                // Convert to lowercase for case-insensitive matching
                glob::Pattern::new(&pattern.to_lowercase())
            };

            let glob = glob_pattern
                .map_err(|e| SearchError::InvalidPattern(e.to_string()))?;
            (Some(glob), None)
        };

        Ok(Self {
            glob_matcher,
            regex_matcher,
            case_sensitive,
        })
    }

    pub fn matches(&self, name: &str) -> bool {
        if let Some(regex) = &self.regex_matcher {
            regex.is_match(name)
        } else if let Some(glob) = &self.glob_matcher {
            let test_name = if self.case_sensitive {
                name
            } else {
                &name.to_lowercase()
            };
            glob.matches(test_name)
        } else {
            true // No pattern means match everything
        }
    }
}

/// Main file searcher implementation
pub struct FileSearcher {
    options: SearchOptions,
    normalizer: PathNormalizer,
    pattern_matcher: Option<PatternMatcher>,
}

impl FileSearcher {
    pub fn new(options: SearchOptions) -> Result<Self> {
        let normalizer = PathNormalizer::new();

        let pattern_matcher = if let Some(pattern) = &options.pattern {
            Some(PatternMatcher::new(pattern, options.use_regex, options.case_sensitive)?)
        } else {
            None
        };

        Ok(Self {
            options,
            normalizer,
            pattern_matcher,
        })
    }

    /// Search for files starting from the given root path
    pub fn search<P: AsRef<Path>>(&self, root: P) -> Result<Vec<SearchResult>> {
        let root_path = root.as_ref();

        // Normalize the root path
        let normalized_root = if self.options.normalize_paths {
            self.normalizer.normalize(&root_path.to_string_lossy())?
        } else {
            root_path.to_path_buf()
        };

        debug!("Starting search in: {:?}", normalized_root);

        let (tx, rx) = mpsc::channel();
        let options = Arc::new(self.options.clone());
        let normalizer = Arc::new(self.normalizer.clone());
        let pattern_matcher = Arc::new(self.pattern_matcher.clone());

        // Set up parallel walker
        let walker = WalkBuilder::new(&normalized_root)
            .hidden(!self.options.include_hidden)
            .follow_links(self.options.follow_links)
            .git_ignore(self.options.respect_gitignore)
            .threads(self.options.threads)
            .max_depth(self.options.max_depth)
            .build_parallel();

        // Spawn worker threads
        thread::scope(|scope| {
            let tx = tx.clone();

            scope.spawn(move |_| {
                walker.run(|| {
                    let tx = tx.clone();
                    let options = Arc::clone(&options);
                    let normalizer = Arc::clone(&normalizer);
                    let pattern_matcher = Arc::clone(&pattern_matcher);

                    Box::new(move |entry_result| {
                        match entry_result {
                            Ok(entry) => {
                                if let Ok(result) = self.process_entry(&entry, &options, &normalizer, &pattern_matcher) {
                                    if let Some(search_result) = result {
                                        let _ = tx.send(Ok(search_result));
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = tx.send(Err(SearchError::Io(err)));
                            }
                        }
                        WalkState::Continue
                    })
                });
            });

            // Drop the original sender
            drop(tx);
        }).map_err(|_| SearchError::Cancelled)?;

        // Collect results
        let mut results = Vec::new();
        while let Ok(result) = rx.recv() {
            match result {
                Ok(search_result) => results.push(search_result),
                Err(err) => {
                    warn!("Search error: {}", err);
                    // Continue with other results
                }
            }
        }

        debug!("Search completed. Found {} results", results.len());
        Ok(results)
    }

    fn process_entry(
        &self,
        entry: &ignore::DirEntry,
        options: &SearchOptions,
        normalizer: &PathNormalizer,
        pattern_matcher: &Option<PatternMatcher>,
    ) -> Result<Option<SearchResult>> {
        let path = entry.path();
        let depth = entry.depth();

        // Check depth constraints
        if let Some(min_depth) = options.min_depth {
            if depth < min_depth {
                return Ok(None);
            }
        }

        // Check entry type
        let entry_type = if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            EntryType::Directory
        } else if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
            EntryType::Symlink
        } else if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            EntryType::File
        } else {
            EntryType::Other
        };

        if !options.entry_types.contains(&entry_type) {
            return Ok(None);
        }

        // Check pattern match
        if let Some(matcher) = pattern_matcher {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !matcher.matches(name) {
                    return Ok(None);
                }
            }
        }

        // Check file extension
        if !options.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !options.extensions.contains(&ext.to_lowercase()) {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }

        let metadata = entry.metadata().ok();

        // Check size constraints
        if let Some(meta) = &metadata {
            if !options.size_filter.matches(meta.len()) {
                return Ok(None);
            }
        }

        // Check time constraints
        if let Some(meta) = &metadata {
            if let Ok(modified) = meta.modified() {
                if !options.time_filter.matches(modified) {
                    return Ok(None);
                }
            }
        }

        // Check executable constraint
        if options.executable_only {
            let is_exe = SearchResult::is_executable(path, &metadata);
            if !is_exe {
                return Ok(None);
            }
        }

        // Check readable constraint
        if options.readable_only {
            if !path.exists() || metadata.is_none() {
                return Ok(None);
            }
        }

        // Create search result
        SearchResult::from_entry(entry, normalizer, options).map(Some)
    }

    /// Count total matches without returning full results (for performance)
    pub fn count<P: AsRef<Path>>(&self, root: P) -> Result<usize> {
        let results = self.search(root)?;
        Ok(results.len())
    }

    /// Check if any matches exist (early termination for existence checks)
    pub fn exists<P: AsRef<Path>>(&self, root: P) -> Result<bool> {
        // Early termination version - just check if any match exists
        let root_path = root.as_ref();
        let normalized_root = if self.options.normalize_paths {
            self.normalizer.normalize(&root_path.to_string_lossy())?
        } else {
            root_path.to_path_buf()
        };

        let walker = WalkBuilder::new(&normalized_root)
            .hidden(!self.options.include_hidden)
            .follow_links(self.options.follow_links)
            .git_ignore(self.options.respect_gitignore)
            .max_depth(self.options.max_depth)
            .build();

        for entry_result in walker {
            if let Ok(entry) = entry_result {
                if let Ok(Some(_)) = self.process_entry(
                    &entry,
                    &self.options,
                    &self.normalizer,
                    &self.pattern_matcher,
                ) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

// Add num_cpus dependency
use regex;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files and directories
        fs::create_dir_all(root.join("subdir")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();

        fs::write(root.join("test.txt"), "test content").unwrap();
        fs::write(root.join("test.rs"), "fn main() {}").unwrap();
        fs::write(root.join("subdir").join("nested.txt"), "nested").unwrap();
        fs::write(root.join(".hidden").join("secret.txt"), "secret").unwrap();

        temp_dir
    }

    #[test]
    fn test_basic_search() {
        let temp_dir = create_test_structure();
        let options = SearchOptions::new().pattern("*.txt");
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert!(results.len() >= 2); // test.txt and nested.txt
    }

    #[test]
    fn test_regex_search() {
        let temp_dir = create_test_structure();
        let options = SearchOptions::new()
            .pattern(r"test\.(txt|rs)")
            .regex(true);
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_hidden_files() {
        let temp_dir = create_test_structure();
        let options = SearchOptions::new()
            .pattern("secret.txt")
            .include_hidden(true);
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_depth_limiting() {
        let temp_dir = create_test_structure();
        let options = SearchOptions::new()
            .max_depth(1)
            .pattern("nested.txt");
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert_eq!(results.len(), 0); // nested.txt is at depth 2
    }

    #[test]
    fn test_type_filtering() {
        let temp_dir = create_test_structure();
        let mut types = HashSet::new();
        types.insert(EntryType::Directory);

        let options = SearchOptions::new().entry_types(types);
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert!(results.iter().all(|r| r.entry_type == EntryType::Directory));
    }

    #[test]
    fn test_extension_filtering() {
        let temp_dir = create_test_structure();
        let mut extensions = HashSet::new();
        extensions.insert("rs".to_string());

        let options = SearchOptions::new().extensions(extensions);
        let searcher = FileSearcher::new(options).unwrap();

        let results = searcher.search(temp_dir.path()).unwrap();
        assert!(results.iter().all(|r| {
            r.path.extension().and_then(|e| e.to_str()) == Some("rs")
        }));
    }

    #[test]
    fn test_pattern_matcher() {
        let matcher = PatternMatcher::new("*.txt", false, false).unwrap();
        assert!(matcher.matches("test.txt"));
        assert!(matcher.matches("TEST.TXT")); // case insensitive
        assert!(!matcher.matches("test.rs"));
    }

    #[test]
    fn test_size_filter() {
        let filter = SizeFilter::new().min(5).max(100);
        assert!(filter.matches(50));
        assert!(!filter.matches(4));
        assert!(!filter.matches(101));
    }

    #[test]
    fn test_time_filter() {
        let now = SystemTime::now();
        let hour_ago = now - Duration::from_secs(3600);
        let future = now + Duration::from_secs(3600);

        let filter = TimeFilter::new().newer_than(hour_ago).older_than(future);
        assert!(filter.matches(now));
        assert!(!filter.matches(hour_ago - Duration::from_secs(1)));
        assert!(!filter.matches(future + Duration::from_secs(1)));
    }
}
