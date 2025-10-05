// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Windows PATHEXT environment variable handling

use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::env;
use ahash::AHashSet;

/// Default Windows executable extensions
const DEFAULT_PATHEXT: &[&str] = &[
    ".COM", ".EXE", ".BAT", ".CMD", ".VBS", ".VBE", ".JS", ".JSE", ".WSF", ".WSH", ".MSC", ".PY"
];

/// Cached PATHEXT extensions (uppercase for case-insensitive comparison)
static PATHEXT_CACHE: Lazy<AHashSet<String>> = Lazy::new(|| {
    let pathext = env::var("PATHEXT").unwrap_or_else(|_| {
        DEFAULT_PATHEXT.join(";")
    });

    pathext
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_uppercase())
        .collect()
});

/// Check if a file extension is executable according to PATHEXT
pub fn is_executable_extension(ext: &str) -> bool {
    let ext_upper = ext.to_uppercase();
    PATHEXT_CACHE.contains(&ext_upper)
}

/// Get all executable extensions from PATHEXT
pub fn get_executable_extensions() -> Vec<String> {
    PATHEXT_CACHE.iter().cloned().collect()
}

/// Check if a filename has an executable extension
pub fn has_executable_extension(filename: &str) -> bool {
    if let Some(ext_pos) = filename.rfind('.') {
        let ext = &filename[ext_pos..];
        is_executable_extension(ext)
    } else {
        false
    }
}

/// Get the file extension (including the dot)
pub fn get_extension(filename: &str) -> Option<&str> {
    filename.rfind('.').map(|pos| &filename[pos..])
}

/// Add extensions to a base filename if it doesn't already have an executable extension
pub fn expand_with_extensions(filename: &str) -> Vec<String> {
    let mut results = Vec::new();

    // Always include the original filename
    results.push(filename.to_string());

    // If the filename doesn't have an executable extension, add all possible extensions
    if !has_executable_extension(filename) {
        for ext in get_executable_extensions() {
            results.push(format!("{}{}", filename, ext.to_lowercase()));
        }
    }

    results
}

/// Check if a pattern might match executable files
pub fn pattern_matches_executables(pattern: &str) -> bool {
    // If pattern has no extension, it could match any executable
    if !pattern.contains('.') {
        return true;
    }

    // If pattern has wildcard extension, it could match executables
    if pattern.ends_with(".*") {
        return true;
    }

    // Check if the pattern's extension is executable
    if let Some(ext) = get_extension(pattern) {
        // Handle wildcard patterns like "*.exe"
        if ext.starts_with(".") && ext.len() > 1 {
            let clean_ext = if ext.contains('*') || ext.contains('?') {
                // For patterns like "*.exe", extract the ".exe" part
                if let Some(star_pos) = ext.find('*') {
                    &ext[star_pos + 1..]
                } else {
                    ext
                }
            } else {
                ext
            };

            return is_executable_extension(clean_ext);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_executable_extension() {
        assert!(is_executable_extension(".exe"));
        assert!(is_executable_extension(".EXE"));
        assert!(is_executable_extension(".com"));
        assert!(is_executable_extension(".bat"));
        assert!(!is_executable_extension(".txt"));
        assert!(!is_executable_extension(".rs"));
    }

    #[test]
    fn test_has_executable_extension() {
        assert!(has_executable_extension("test.exe"));
        assert!(has_executable_extension("program.COM"));
        assert!(has_executable_extension("script.bat"));
        assert!(!has_executable_extension("document.txt"));
        assert!(!has_executable_extension("noextension"));
    }

    #[test]
    fn test_expand_with_extensions() {
        let expanded = expand_with_extensions("python");
        assert!(expanded.contains(&"python".to_string()));
        assert!(expanded.len() > 1); // Should have added extensions

        let expanded = expand_with_extensions("test.exe");
        assert_eq!(expanded.len(), 1); // Already has executable extension
        assert_eq!(expanded[0], "test.exe");
    }

    #[test]
    fn test_pattern_matches_executables() {
        assert!(pattern_matches_executables("python"));
        assert!(pattern_matches_executables("*.exe"));
        assert!(pattern_matches_executables("test.*"));
        assert!(pattern_matches_executables("prog*.exe"));
        assert!(!pattern_matches_executables("*.txt"));
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("test.exe"), Some(".exe"));
        assert_eq!(get_extension("file.tar.gz"), Some(".gz"));
        assert_eq!(get_extension("noextension"), None);
        assert_eq!(get_extension(".hidden"), Some(".hidden"));
    }
}
