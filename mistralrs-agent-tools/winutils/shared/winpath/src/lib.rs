//! # WinPath - Comprehensive Windows Path Normalization Library
//!
//! This library provides robust Windows path normalization supporting all major path formats:
//! - DOS paths: `C:\Users\David`
//! - DOS forward slash: `C:/Users/David`
//! - WSL mount points: `/mnt/c/users/david`
//! - Cygwin paths: `/cygdrive/c/users/david`
//! - UNC long paths: `\\?\C:\Users\David`
//! - Unix-like with double slashes: `//c//users//david//`
//! - Mixed formats and combinations
//!
//! ## Features
//!
//! - **Zero-copy optimization**: Uses `Cow<str>` for efficient memory usage
//! - **Thread-safe caching**: Optional LRU cache for repeated path normalizations
//! - **Unicode support**: Proper handling of Unicode paths with normalization
//! - **Long path support**: Automatic handling of paths >260 characters
//! - **Type safety**: Strong typing with compile-time path format detection
//! - **Performance optimized**: SIMD-accelerated string operations where available
//!
//! ## Example
//!
//! ```rust
//! use winpath::{PathNormalizer, normalize_path};
//!
//! // Quick normalization
//! let normalized = normalize_path("/mnt/c/users/david/documents")?;
//! assert_eq!(normalized, r"C:\users\david\documents");
//!
//! // Using the normalizer with caching
//! let normalizer = PathNormalizer::new();
//! let result = normalizer.normalize("/cygdrive/c/program files/app")?;
//! assert_eq!(result.path(), r"C:\program files\app");
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    unused_qualifications
)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

// Re-export core modules
pub use crate::detection::{PathFormat, detect_path_format};
pub use crate::normalization::{normalize_path, normalize_path_cow, NormalizationResult};
pub use crate::normalizer::{PathNormalizer, NormalizerConfig};
pub use crate::error::{PathError, Result};

// Core modules
mod detection;
mod normalization;
mod normalizer;
mod error;
mod utils;

// Optional feature modules
#[cfg(feature = "cache")]
mod cache;

// Platform-specific modules
#[cfg(windows)]
mod platform;

// Constants
pub(crate) mod constants {
    /// Maximum path length for standard Windows paths
    pub const MAX_PATH: usize = 260;

    /// Prefix for UNC long paths
    pub const UNC_PREFIX: &str = r"\\?\";

    /// WSL mount prefix
    pub const WSL_PREFIX: &str = "/mnt/";

    /// Cygwin drive prefix
    pub const CYGWIN_PREFIX: &str = "/cygdrive/";

    /// Path separators
    pub const BACKSLASH: char = '\\';
    pub const FORWARD_SLASH: char = '/';

    /// Drive letter pattern (A-Z)
    pub const DRIVE_LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// Default cache capacity
    pub const DEFAULT_CACHE_SIZE: usize = 1024;

    /// Common Git Bash installation prefixes
    pub const GIT_BASH_PREFIXES: &[&str] = &[
        r"C:\Program Files\Git",
        r"C:\Program Files (x86)\Git",
        r"C:\Git",
        r"C:\Tools\Git",
    ];

    /// Git Bash mangled path patterns
    pub const GIT_BASH_MANGLE_PATTERN: &str = r"\mnt\";
    pub const GIT_BASH_MANGLE_PATTERN_ALT: &str = "/mnt/";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_normalization() {
        // DOS path should remain unchanged
        assert_eq!(normalize_path(r"C:\Users\David").unwrap(), r"C:\Users\David");

        // Forward slash DOS path
        assert_eq!(normalize_path("C:/Users/David").unwrap(), r"C:\Users\David");

        // WSL path
        assert_eq!(normalize_path("/mnt/c/users/david").unwrap(), r"C:\users\david");

        // Cygwin path
        assert_eq!(normalize_path("/cygdrive/c/users/david").unwrap(), r"C:\users\david");
    }

    #[test]
    fn test_path_format_detection() {
        assert_eq!(detect_path_format(r"C:\Users"), PathFormat::Dos);
        assert_eq!(detect_path_format("C:/Users"), PathFormat::DosForward);
        assert_eq!(detect_path_format("/mnt/c/users"), PathFormat::Wsl);
        assert_eq!(detect_path_format("/cygdrive/c/users"), PathFormat::Cygwin);
        assert_eq!(detect_path_format(r"\\?\C:\Users"), PathFormat::Unc);
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_cached_normalization() {
        let normalizer = PathNormalizer::new();

        let result1 = normalizer.normalize("/mnt/c/test").unwrap();
        let result2 = normalizer.normalize("/mnt/c/test").unwrap();

        assert_eq!(result1.path(), result2.path());
        assert_eq!(result1.path(), r"C:\test");
    }

    #[test]
    fn test_long_path_handling() {
        let long_component = "a".repeat(200);
        let long_path = format!("/mnt/c/very/long/path/with/{}/component", long_component);

        let result = normalize_path(&long_path).unwrap();
        assert!(result.starts_with(r"\\?\C:\very\long\path\with"));
    }

    #[test]
    fn test_mixed_separators() {
        let mixed = r"C:\Users/David\Documents/file.txt";
        let result = normalize_path(mixed).unwrap();
        assert_eq!(result, r"C:\Users\David\Documents\file.txt");
    }

    #[test]
    fn test_case_preservation() {
        let path = "/mnt/c/Program Files/MyApp";
        let result = normalize_path(path).unwrap();
        assert_eq!(result, r"C:\Program Files\MyApp");
    }

    #[test]
    fn test_error_handling() {
        // Invalid drive letter
        assert!(normalize_path("/mnt/z/test").is_err());

        // Empty path
        assert!(normalize_path("").is_err());

        // Invalid WSL format
        assert!(normalize_path("/mnt/").is_err());
    }
}
