//! # winpath - Windows Path Normalization Library
//!
//! This library provides comprehensive path normalization for Windows environments,
//! with special support for Git Bash, WSL, Cygwin, and native Windows paths.
//!
//! ## Key Features
//!
//! - Automatic detection of path context (Git Bash, WSL, Cygwin, Windows)
//! - Bidirectional path conversion between formats
//! - Long path support (>260 characters)
//! - UNC path handling
//! - Drive letter normalization
//! - Environment variable expansion
//! - Symlink resolution
//!
//! ## Usage
//!
//! ```rust
//! use winpath::{PathNormalizer, PathContext};
//!
//! let normalizer = PathNormalizer::new();
//! let normalized = normalizer.normalize("/c/Users/user", PathContext::GitBash)?;
//! assert_eq!(normalized, "C:\\Users\\user");
//! ```

use anyhow::{anyhow, Context, Result};
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(windows)]
use widestring::U16CString;
#[cfg(windows)]
use windows::Win32::Foundation::{GetLastError, ERROR_SUCCESS, MAX_PATH};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    GetFullPathNameW, GetLongPathNameW, GetShortPathNameW, FILE_ATTRIBUTE_REPARSE_POINT,
    GetFileAttributesW,
};

/// Errors that can occur during path normalization
#[derive(Error, Debug)]
pub enum PathError {
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),
    #[error("Path too long (>{0} characters): {1}")]
    TooLong(usize, String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Windows API error: {0}")]
    WindowsApi(String),
    #[error("Unsupported operation in context: {0:?}")]
    UnsupportedContext(PathContext),
}

/// Context in which the path normalization is occurring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathContext {
    /// Native Windows (CMD, PowerShell, native applications)
    Windows,
    /// Git Bash (MSYS2-based)
    GitBash,
    /// Windows Subsystem for Linux
    WSL,
    /// Cygwin environment
    Cygwin,
    /// Auto-detect based on environment
    Auto,
}

/// Options for path normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeOptions {
    /// Target context for normalization
    pub target_context: PathContext,
    /// Whether to resolve symlinks
    pub resolve_symlinks: bool,
    /// Whether to expand environment variables
    pub expand_env_vars: bool,
    /// Whether to use long path format (\\?\ prefix)
    pub use_long_paths: bool,
    /// Whether to normalize case (Windows is case-insensitive)
    pub normalize_case: bool,
    /// Whether to clean redundant path components (., .., //)
    pub clean_path: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            target_context: PathContext::Auto,
            resolve_symlinks: true,
            expand_env_vars: true,
            use_long_paths: false,
            normalize_case: true,
            clean_path: true,
        }
    }
}

/// Main path normalization struct
#[derive(Debug)]
pub struct PathNormalizer {
    current_context: PathContext,
    drive_mappings: HashMap<String, String>,
    env_cache: HashMap<String, String>,
}

impl Default for PathNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PathNormalizer {
    /// Create a new path normalizer with auto-detected context
    pub fn new() -> Self {
        let current_context = Self::detect_context();
        let mut normalizer = Self {
            current_context,
            drive_mappings: HashMap::new(),
            env_cache: HashMap::new(),
        };

        normalizer.initialize_drive_mappings();
        normalizer.cache_common_env_vars();
        normalizer
    }

    /// Create a path normalizer with explicit context
    pub fn with_context(context: PathContext) -> Self {
        let mut normalizer = Self {
            current_context: context,
            drive_mappings: HashMap::new(),
            env_cache: HashMap::new(),
        };

        normalizer.initialize_drive_mappings();
        normalizer.cache_common_env_vars();
        normalizer
    }

    /// Detect the current path context based on environment variables
    pub fn detect_context() -> PathContext {
        // Check for WSL
        if env::var("WSL_DISTRO_NAME").is_ok() ||
           env::var("WSL_INTEROP").is_ok() ||
           Path::new("/proc/version").exists() {
            return PathContext::WSL;
        }

        // Check for Git Bash (MSYS2)
        if env::var("MSYSTEM").is_ok() ||
           env::var("MINGW_PREFIX").is_ok() ||
           env::var("TERM").map(|t| t.contains("xterm")).unwrap_or(false) {
            return PathContext::GitBash;
        }

        // Check for Cygwin
        if env::var("CYGWIN").is_ok() ||
           env::var("SHELL").map(|s| s.contains("bash")).unwrap_or(false) &&
           env::var("OSTYPE").map(|o| o.contains("cygwin")).unwrap_or(false) {
            return PathContext::Cygwin;
        }

        // Default to Windows
        PathContext::Windows
    }

    /// Normalize a path with default options
    pub fn normalize(&self, path: &str) -> Result<PathBuf> {
        self.normalize_with_options(path, &NormalizeOptions::default())
    }

    /// Normalize a path to a specific target context
    pub fn normalize_to_context(&self, path: &str, target: PathContext) -> Result<PathBuf> {
        let mut options = NormalizeOptions::default();
        options.target_context = target;
        self.normalize_with_options(path, &options)
    }

    /// Normalize a path with custom options
    pub fn normalize_with_options(&self, path: &str, options: &NormalizeOptions) -> Result<PathBuf> {
        log::debug!("Normalizing path: '{}' with options: {:?}", path, options);

        let mut normalized_path = String::from(path);

        // Expand environment variables if requested
        if options.expand_env_vars {
            normalized_path = self.expand_env_vars(&normalized_path)?;
        }

        // Detect source context if auto
        let source_context = if self.current_context == PathContext::Auto {
            Self::detect_path_context(&normalized_path)
        } else {
            self.current_context
        };

        let target_context = if options.target_context == PathContext::Auto {
            PathContext::Windows
        } else {
            options.target_context
        };

        // Convert between contexts
        normalized_path = self.convert_path_context(&normalized_path, source_context, target_context)?;

        // Create PathBuf and clean if requested
        let mut path_buf = PathBuf::from(normalized_path);

        if options.clean_path {
            path_buf = path_buf.clean();
        }

        // Handle Windows-specific operations
        #[cfg(windows)]
        if target_context == PathContext::Windows {
            path_buf = self.apply_windows_normalization(&path_buf, options)?;
        }

        // Resolve symlinks if requested
        if options.resolve_symlinks {
            path_buf = self.resolve_symlinks(&path_buf)?;
        }

        log::debug!("Normalized result: {:?}", path_buf);
        Ok(path_buf)
    }

    /// Detect the context of a specific path
    fn detect_path_context(path: &str) -> PathContext {
        // Unix-style absolute paths
        if path.starts_with('/') {
            // WSL paths start with /mnt/
            if path.starts_with("/mnt/") {
                return PathContext::WSL;
            }
            // Cygwin paths start with /cygdrive/
            if path.starts_with("/cygdrive/") {
                return PathContext::Cygwin;
            }
            // Git Bash paths like /c/, /d/, etc.
            if path.len() >= 3 && path.chars().nth(1).unwrap().is_ascii_alphabetic() &&
               path.chars().nth(2) == Some('/') {
                return PathContext::GitBash;
            }
            // Other Unix paths (could be WSL or Git Bash internal)
            return PathContext::GitBash;
        }

        // Windows-style paths
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            return PathContext::Windows;
        }

        // UNC paths
        if path.starts_with("\\\\") || path.starts_with("//") {
            return PathContext::Windows;
        }

        // Default to Windows
        PathContext::Windows
    }

    /// Convert between different path contexts
    fn convert_path_context(
        &self,
        path: &str,
        source: PathContext,
        target: PathContext,
    ) -> Result<String> {
        if source == target {
            return Ok(path.to_string());
        }

        match (source, target) {
            // To Windows format
            (PathContext::GitBash, PathContext::Windows) => self.gitbash_to_windows(path),
            (PathContext::WSL, PathContext::Windows) => self.wsl_to_windows(path),
            (PathContext::Cygwin, PathContext::Windows) => self.cygwin_to_windows(path),

            // From Windows format
            (PathContext::Windows, PathContext::GitBash) => self.windows_to_gitbash(path),
            (PathContext::Windows, PathContext::WSL) => self.windows_to_wsl(path),
            (PathContext::Windows, PathContext::Cygwin) => self.windows_to_cygwin(path),

            // Between Unix-like contexts
            (PathContext::GitBash, PathContext::WSL) => {
                let windows_path = self.gitbash_to_windows(path)?;
                self.windows_to_wsl(&windows_path)
            }
            (PathContext::WSL, PathContext::GitBash) => {
                let windows_path = self.wsl_to_windows(path)?;
                self.windows_to_gitbash(&windows_path)
            }
            _ => Ok(path.to_string()),
        }
    }

    /// Convert Git Bash path to Windows path
    fn gitbash_to_windows(&self, path: &str) -> Result<String> {
        // Handle Git Bash drive notation (/c/ -> C:\)
        if path.len() >= 3 && path.starts_with('/') &&
           path.chars().nth(1).unwrap().is_ascii_alphabetic() &&
           path.chars().nth(2) == Some('/') {
            let drive = path.chars().nth(1).unwrap().to_ascii_uppercase();
            let rest = &path[3..];
            let windows_path = format!("{}:\\{}", drive, rest.replace('/', "\\"));
            return Ok(windows_path);
        }

        // Handle absolute Unix paths in Git Bash context
        if path.starts_with('/') {
            // This might be a Git Bash internal path, try to map it
            if let Some(mapped) = self.try_map_gitbash_path(path) {
                return Ok(mapped);
            }
        }

        // Relative paths - just convert separators
        Ok(path.replace('/', "\\"))
    }

    /// Convert WSL path to Windows path
    fn wsl_to_windows(&self, path: &str) -> Result<String> {
        // Handle WSL mount points (/mnt/c/ -> C:\)
        if path.starts_with("/mnt/") && path.len() >= 7 {
            let drive = path.chars().nth(5).unwrap().to_ascii_uppercase();
            let rest = &path[7..];
            let windows_path = format!("{}:\\{}", drive, rest.replace('/', "\\"));
            return Ok(windows_path);
        }

        // Handle other WSL paths (these stay as Unix paths when used in WSL context)
        Ok(path.to_string())
    }

    /// Convert Cygwin path to Windows path
    fn cygwin_to_windows(&self, path: &str) -> Result<String> {
        // Handle Cygwin drive notation (/cygdrive/c/ -> C:\)
        if path.starts_with("/cygdrive/") && path.len() >= 12 {
            let drive = path.chars().nth(10).unwrap().to_ascii_uppercase();
            let rest = &path[12..];
            let windows_path = format!("{}:\\{}", drive, rest.replace('/', "\\"));
            return Ok(windows_path);
        }

        // Relative paths - just convert separators
        Ok(path.replace('/', "\\"))
    }

    /// Convert Windows path to Git Bash path
    fn windows_to_gitbash(&self, path: &str) -> Result<String> {
        // Handle drive letters (C:\ -> /c/)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().nth(0).unwrap().to_ascii_lowercase();
            let rest = if path.len() > 3 { &path[3..] } else { "" };
            let gitbash_path = format!("/{}/{}", drive, rest.replace('\\', "/"));
            return Ok(gitbash_path);
        }

        // UNC paths (\\server\share -> //server/share)
        if path.starts_with("\\\\") {
            return Ok(path.replace('\\', "/"));
        }

        // Relative paths
        Ok(path.replace('\\', "/"))
    }

    /// Convert Windows path to WSL path
    fn windows_to_wsl(&self, path: &str) -> Result<String> {
        // Handle drive letters (C:\ -> /mnt/c/)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().nth(0).unwrap().to_ascii_lowercase();
            let rest = if path.len() > 3 { &path[3..] } else { "" };
            let wsl_path = format!("/mnt/{}/{}", drive, rest.replace('\\', "/"));
            return Ok(wsl_path);
        }

        // UNC paths - not directly supported in WSL, return as-is
        Ok(path.replace('\\', "/"))
    }

    /// Convert Windows path to Cygwin path
    fn windows_to_cygwin(&self, path: &str) -> Result<String> {
        // Handle drive letters (C:\ -> /cygdrive/c/)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().nth(0).unwrap().to_ascii_lowercase();
            let rest = if path.len() > 3 { &path[3..] } else { "" };
            let cygwin_path = format!("/cygdrive/{}/{}", drive, rest.replace('\\', "/"));
            return Ok(cygwin_path);
        }

        // UNC paths
        if path.starts_with("\\\\") {
            return Ok(path.replace('\\', "/"));
        }

        // Relative paths
        Ok(path.replace('\\', "/"))
    }

    /// Try to map Git Bash internal paths to Windows equivalents
    fn try_map_gitbash_path(&self, path: &str) -> Option<String> {
        // Common Git Bash path mappings
        match path {
            "/usr" => Some("C:\\Program Files\\Git\\usr".to_string()),
            "/bin" => Some("C:\\Program Files\\Git\\usr\\bin".to_string()),
            "/etc" => Some("C:\\Program Files\\Git\\etc".to_string()),
            "/tmp" => Some(env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".to_string())),
            _ => {
                // Try common prefixes
                if path.starts_with("/usr/") {
                    Some(format!("C:\\Program Files\\Git\\{}", &path[1..].replace('/', "\\")))
                } else if path.starts_with("/tmp/") {
                    let temp = env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".to_string());
                    Some(format!("{}\\{}", temp, &path[5..].replace('/', "\\")))
                } else {
                    None
                }
            }
        }
    }

    /// Expand environment variables in path
    fn expand_env_vars(&self, path: &str) -> Result<String> {
        let mut result = path.to_string();

        // Handle $VAR and ${VAR} syntax
        let var_pattern = regex::Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)")
            .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

        result = var_pattern.replace_all(&result, |caps: &regex::Captures| {
            let var_name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
            self.env_cache.get(var_name)
                .or_else(|| env::var(var_name).ok().as_ref())
                .unwrap_or(&caps[0])
                .to_string()
        }).to_string();

        // Handle Windows %VAR% syntax
        let win_var_pattern = regex::Regex::new(r"%([A-Za-z_][A-Za-z0-9_]*)%")
            .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

        result = win_var_pattern.replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            self.env_cache.get(var_name)
                .or_else(|| env::var(var_name).ok().as_ref())
                .unwrap_or(&caps[0])
                .to_string()
        }).to_string();

        Ok(result)
    }

    /// Apply Windows-specific path normalization
    #[cfg(windows)]
    fn apply_windows_normalization(&self, path: &PathBuf, options: &NormalizeOptions) -> Result<PathBuf> {
        let path_str = path.to_string_lossy();

        // Convert to wide string for Windows APIs
        let wide_path = U16CString::from_str(&path_str)
            .map_err(|e| PathError::InvalidFormat(format!("Failed to convert to wide string: {}", e)))?;

        // Get full path name
        let mut buffer = vec![0u16; 32768]; // Large buffer for long paths
        let result = unsafe {
            GetFullPathNameW(
                wide_path.as_ptr(),
                buffer.len() as u32,
                buffer.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };

        if result == 0 {
            return Err(PathError::WindowsApi(format!(
                "GetFullPathNameW failed with error: {:?}",
                unsafe { GetLastError() }
            )).into());
        }

        // Convert back to string
        let full_path = unsafe {
            let len = result as usize;
            String::from_utf16_lossy(&buffer[..len])
        };

        let mut result_path = PathBuf::from(full_path);

        // Apply long path format if requested or if path is too long
        if options.use_long_paths || result_path.to_string_lossy().len() > MAX_PATH as usize {
            if !result_path.to_string_lossy().starts_with("\\\\?\\") {
                result_path = PathBuf::from(format!("\\\\?\\{}", result_path.display()));
            }
        }

        Ok(result_path)
    }

    #[cfg(not(windows))]
    fn apply_windows_normalization(&self, path: &PathBuf, _options: &NormalizeOptions) -> Result<PathBuf> {
        Ok(path.clone())
    }

    /// Resolve symlinks in path
    fn resolve_symlinks(&self, path: &PathBuf) -> Result<PathBuf> {
        match std::fs::canonicalize(path) {
            Ok(canonical) => Ok(canonical),
            Err(_) => {
                // If canonicalization fails, return the original path
                // This might happen if the path doesn't exist yet
                log::debug!("Failed to canonicalize path: {:?}, returning original", path);
                Ok(path.clone())
            }
        }
    }

    /// Initialize drive mappings for the current system
    fn initialize_drive_mappings(&mut self) {
        // Add common drive mappings
        #[cfg(windows)]
        {
            // Get available drives
            let drives = std::fs::read_dir("\\")
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|entry| entry.ok())
                        .filter_map(|entry| {
                            entry.file_name().to_str()
                                .and_then(|name| name.chars().next())
                                .filter(|c| c.is_ascii_alphabetic())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for drive in drives {
                let drive_letter = drive.to_ascii_lowercase().to_string();
                self.drive_mappings.insert(drive_letter.clone(), format!("{}:", drive.to_ascii_uppercase()));
            }
        }
    }

    /// Cache common environment variables for faster lookup
    fn cache_common_env_vars(&mut self) {
        let common_vars = [
            "HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "PROGRAMFILES",
            "PROGRAMFILES(X86)", "PROGRAMDATA", "TEMP", "TMP", "WINDIR", "SYSTEMROOT",
        ];

        for var in &common_vars {
            if let Ok(value) = env::var(var) {
                self.env_cache.insert(var.to_string(), value);
            }
        }
    }

    /// Get the current context
    pub fn current_context(&self) -> PathContext {
        self.current_context
    }

    /// Check if a path exists
    pub fn path_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists()
    }

    /// Check if a path is absolute
    pub fn is_absolute<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().is_absolute()
    }

    /// Get the file extension
    pub fn get_extension<P: AsRef<Path>>(&self, path: P) -> Option<&str> {
        path.as_ref().extension()?.to_str()
    }

    /// Join multiple path components
    pub fn join_paths<I, P>(&self, paths: I) -> PathBuf
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut result = PathBuf::new();
        for path in paths {
            result = result.join(path);
        }
        result
    }
}

// Add regex dependency
use regex;

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_context_detection() {
        let context = PathNormalizer::detect_context();
        // This will vary depending on where tests are run
        assert!(matches!(context, PathContext::Windows | PathContext::GitBash | PathContext::WSL | PathContext::Cygwin));
    }

    #[test]
    fn test_gitbash_to_windows() {
        let normalizer = PathNormalizer::new();
        let result = normalizer.gitbash_to_windows("/c/Users/test").unwrap();
        assert_eq!(result, "C:\\Users\\test");
    }

    #[test]
    fn test_windows_to_gitbash() {
        let normalizer = PathNormalizer::new();
        let result = normalizer.windows_to_gitbash("C:\\Users\\test").unwrap();
        assert_eq!(result, "/c/Users/test");
    }

    #[test]
    fn test_wsl_to_windows() {
        let normalizer = PathNormalizer::new();
        let result = normalizer.wsl_to_windows("/mnt/c/Users/test").unwrap();
        assert_eq!(result, "C:\\Users\\test");
    }

    #[test]
    fn test_windows_to_wsl() {
        let normalizer = PathNormalizer::new();
        let result = normalizer.windows_to_wsl("C:\\Users\\test").unwrap();
        assert_eq!(result, "/mnt/c/Users/test");
    }

    #[test]
    fn test_path_context_detection() {
        assert_eq!(PathNormalizer::detect_path_context("/c/Users"), PathContext::GitBash);
        assert_eq!(PathNormalizer::detect_path_context("/mnt/c/Users"), PathContext::WSL);
        assert_eq!(PathNormalizer::detect_path_context("C:\\Users"), PathContext::Windows);
        assert_eq!(PathNormalizer::detect_path_context("/cygdrive/c/Users"), PathContext::Cygwin);
    }

    #[test]
    fn test_env_var_expansion() {
        let normalizer = PathNormalizer::new();

        // Set a test environment variable
        env::set_var("TEST_VAR", "test_value");

        let result = normalizer.expand_env_vars("$TEST_VAR/path").unwrap();
        assert_eq!(result, "test_value/path");

        let result = normalizer.expand_env_vars("${TEST_VAR}/path").unwrap();
        assert_eq!(result, "test_value/path");

        // Clean up
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_path_cleaning() {
        let normalizer = PathNormalizer::new();
        let options = NormalizeOptions {
            clean_path: true,
            ..Default::default()
        };

        let result = normalizer.normalize_with_options("C:\\Users\\..\\Users\\test\\.", &options).unwrap();
        assert_eq!(result.to_string_lossy(), "C:\\Users\\test");
    }

    #[test]
    fn test_relative_path_handling() {
        let normalizer = PathNormalizer::new();

        let result = normalizer.normalize("./test/path").unwrap();
        assert!(result.to_string_lossy().contains("test"));
        assert!(result.to_string_lossy().contains("path"));
    }
}
