// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! High-performance caching for PATH directories and file listings

use crate::error::{WhereError, WhereResult};
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;
use winpath::normalize_path;

/// Cache entry for directory listings
#[derive(Debug, Clone)]
struct DirectoryEntry {
    files: Vec<PathBuf>,
    last_modified: SystemTime,
    expires_at: SystemTime,
}

/// Cache for PATH directories and their contents
pub struct PathCache {
    /// Cache of directory contents
    dir_cache: Arc<RwLock<LruCache<PathBuf, DirectoryEntry>>>,
    /// Cache of individual file existence checks
    file_cache: Arc<DashMap<PathBuf, (bool, SystemTime)>>,
    /// Parsed PATH directories
    path_dirs: Arc<RwLock<Option<Vec<PathBuf>>>>,
    /// Cache timeout in seconds
    cache_timeout: Duration,
}

impl PathCache {
    /// Create a new PATH cache with specified capacity and timeout
    pub fn new(capacity: usize, timeout_secs: u64) -> Self {
        Self {
            dir_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap()
            ))),
            file_cache: Arc::new(DashMap::new()),
            path_dirs: Arc::new(RwLock::new(None)),
            cache_timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Get PATH directories, parsing and caching them
    pub fn get_path_dirs(&self) -> WhereResult<Vec<PathBuf>> {
        {
            let path_dirs = self.path_dirs.read();
            if let Some(ref dirs) = *path_dirs {
                return Ok(dirs.clone());
            }
        }

        // Parse PATH environment variable
        let path_var = std::env::var("PATH")
            .map_err(|_| WhereError::environment("PATH"))?;

        let mut dirs = Vec::new();

        // Add current directory first (Windows behavior)
        dirs.push(PathBuf::from("."));

        // Parse PATH directories with normalization for Git Bash compatibility
        for dir_str in path_var.split(';') {
            if !dir_str.is_empty() {
                // Normalize the path to handle Git Bash mangled paths and other formats
                match normalize_path(dir_str) {
                    Ok(normalized) => {
                        let dir_path = PathBuf::from(normalized);
                        if dir_path.is_dir() {
                            dirs.push(dir_path);
                        }
                    }
                    Err(_) => {
                        // If normalization fails, try the original path as fallback
                        let dir_path = PathBuf::from(dir_str);
                        if dir_path.is_dir() {
                            dirs.push(dir_path);
                        }
                    }
                }
            }
        }

        // Cache the parsed directories
        {
            let mut path_dirs = self.path_dirs.write();
            *path_dirs = Some(dirs.clone());
        }

        Ok(dirs)
    }

    /// Get files in a directory with caching
    pub fn get_directory_files(&self, dir: &Path) -> WhereResult<Vec<PathBuf>> {
        let now = SystemTime::now();

        // Check cache first
        {
            let mut cache = self.dir_cache.write();
            if let Some(entry) = cache.get(dir) {
                if entry.expires_at > now {
                    return Ok(entry.files.clone());
                }
                // Entry expired, remove it
                cache.pop(dir);
            }
        }

        // Scan directory
        let files = self.scan_directory(dir)?;

        // Cache the results
        let entry = DirectoryEntry {
            files: files.clone(),
            last_modified: now,
            expires_at: now + self.cache_timeout,
        };

        {
            let mut cache = self.dir_cache.write();
            cache.put(dir.to_path_buf(), entry);
        }

        Ok(files)
    }

    /// Check if a specific file exists with caching
    pub fn file_exists(&self, path: &Path) -> bool {
        let now = SystemTime::now();

        // Check cache first
        if let Some(entry) = self.file_cache.get(path) {
            let (exists, cached_at) = entry.value();
            if now.duration_since(*cached_at).unwrap_or(Duration::MAX) < self.cache_timeout {
                return *exists;
            }
        }

        // Check filesystem
        let exists = path.exists();

        // Cache the result
        self.file_cache.insert(path.to_path_buf(), (exists, now));

        exists
    }

    /// Recursively get all files in a directory tree
    pub fn get_recursive_files(&self, root: &Path) -> WhereResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    /// Clear all caches
    pub fn clear(&self) {
        {
            let mut dir_cache = self.dir_cache.write();
            dir_cache.clear();
        }

        self.file_cache.clear();

        {
            let mut path_dirs = self.path_dirs.write();
            *path_dirs = None;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let dir_cache = self.dir_cache.read();
        let file_cache_len = self.file_cache.len();

        CacheStats {
            dir_cache_entries: dir_cache.len(),
            file_cache_entries: file_cache_len,
            dir_cache_capacity: dir_cache.cap().get(),
        }
    }

    /// Scan a single directory for files
    fn scan_directory(&self, dir: &Path) -> WhereResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        let read_dir = std::fs::read_dir(dir)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    WhereError::permission_denied(dir.display().to_string())
                }
                _ => WhereError::Io(e),
            })?;

        for entry in read_dir {
            let entry = entry.map_err(WhereError::Io)?;
            let path = entry.path();

            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }
}

impl Default for PathCache {
    fn default() -> Self {
        // Default: 1000 directory entries, 5 minute timeout
        Self::new(1000, 300)
    }
}

/// Cache performance statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub dir_cache_entries: usize,
    pub file_cache_entries: usize,
    pub dir_cache_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_path_cache_creation() {
        let cache = PathCache::new(100, 60);
        let stats = cache.stats();
        assert_eq!(stats.dir_cache_capacity, 100);
        assert_eq!(stats.dir_cache_entries, 0);
    }

    #[test]
    fn test_file_exists_caching() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let cache = PathCache::default();

        // First check should cache the result
        assert!(cache.file_exists(&file_path));

        // Second check should use cache
        assert!(cache.file_exists(&file_path));
    }

    #[test]
    fn test_directory_scanning() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.exe");
        let file2 = temp.path().join("file2.bat");
        fs::write(&file1, "").unwrap();
        fs::write(&file2, "").unwrap();

        let cache = PathCache::default();
        let files = cache.get_directory_files(temp.path()).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
    }

    #[test]
    fn test_cache_clearing() {
        let cache = PathCache::default();

        // Add some entries
        cache.file_exists(&PathBuf::from("nonexistent"));

        let stats_before = cache.stats();
        assert!(stats_before.file_cache_entries > 0);

        cache.clear();

        let stats_after = cache.stats();
        assert_eq!(stats_after.file_cache_entries, 0);
        assert_eq!(stats_after.dir_cache_entries, 0);
    }
}
