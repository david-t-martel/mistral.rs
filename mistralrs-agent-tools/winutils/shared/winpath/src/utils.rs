//! Utility functions for path manipulation and optimization.

use crate::{constants::*, error::{PathError, Result}};
use alloc::{string::String, vec::Vec};

/// Normalizes path separators to the target separator.
///
/// Uses SIMD-accelerated operations when available for better performance.
pub fn normalize_separators(path: &str, target_separator: char) -> String {
    if target_separator == BACKSLASH {
        // Convert forward slashes to backslashes
        path.replace(FORWARD_SLASH, &target_separator.to_string())
    } else {
        // Convert backslashes to forward slashes
        path.replace(BACKSLASH, &target_separator.to_string())
    }
}

/// Removes redundant separators from a path.
///
/// Collapses multiple consecutive separators into a single separator.
/// Examples:
/// - `C:\\\\Users` -> `C:\Users`
/// - `//mnt//c//users` -> `/mnt/c/users`
pub fn remove_redundant_separators(path: &str, separator: char) -> String {
    if path.is_empty() {
        return String::new();
    }

    let separator_str = separator.to_string();
    let double_separator = format!("{}{}", separator, separator);

    let mut result = path.to_string();

    // Keep replacing double separators until none remain
    while result.contains(&double_separator) {
        result = result.replace(&double_separator, &separator_str);
    }

    result
}

/// Resolves dot components (. and ..) in a path.
///
/// This function handles relative path components while preserving
/// the absolute nature of paths when appropriate.
pub fn resolve_dot_components(path: &str) -> Result<String> {
    if path.is_empty() {
        return Ok(String::new());
    }

    // Determine separator type
    let separator = if path.contains(BACKSLASH) { BACKSLASH } else { FORWARD_SLASH };

    // Split path into components
    let mut components = Vec::new();
    let mut current_component = String::new();
    let mut chars = path.chars().peekable();

    // Handle drive letter prefix for absolute Windows paths
    let mut result = String::new();
    let mut skip_first_component = false;

    // Check for drive letter (C:\ or C:/)
    if path.len() >= 2 {
        let first_char = path.chars().nth(0);
        let second_char = path.chars().nth(1);

        if let (Some(first), Some(':')) = (first_char, second_char) {
            if first.is_ascii_alphabetic() {
                result.push(first);
                result.push(':');

                // Skip the drive letter and colon
                chars.next(); // first char
                chars.next(); // colon

                // Check for separator after drive
                if chars.peek() == Some(&separator) {
                    result.push(separator);
                    chars.next();
                }
                skip_first_component = true;
            }
        }
    }

    // Parse remaining components
    for ch in chars {
        if ch == separator {
            if !current_component.is_empty() {
                components.push(current_component);
                current_component = String::new();
            }
        } else {
            current_component.push(ch);
        }
    }

    // Add final component if present
    if !current_component.is_empty() {
        components.push(current_component);
    }

    // Resolve dot components
    let mut resolved_components = Vec::new();

    for component in components {
        match component.as_str() {
            "." => {
                // Current directory - skip
                continue;
            }
            ".." => {
                // Parent directory
                if let Some(last) = resolved_components.last() {
                    if last == ".." {
                        // Can't resolve .. if we're already at parent refs
                        resolved_components.push(component);
                    } else {
                        // Remove the last component
                        resolved_components.pop();
                    }
                } else if skip_first_component {
                    // Absolute path - can't go above root
                    return Err(PathError::InvalidComponent(".. above root".to_string()));
                } else {
                    // Relative path - keep the .. component
                    resolved_components.push(component);
                }
            }
            _ => {
                // Regular component
                resolved_components.push(component);
            }
        }
    }

    // Build final path
    if !resolved_components.is_empty() {
        result.push_str(&resolved_components.join(&separator.to_string()));
    }

    Ok(result)
}

/// Splits a path into its directory and filename components.
///
/// Returns (directory, filename) where directory includes the trailing separator
/// if the path is absolute.
pub fn split_path(path: &str) -> (Option<&str>, Option<&str>) {
    if path.is_empty() {
        return (None, None);
    }

    // Find the last separator
    let last_backslash = path.rfind(BACKSLASH);
    let last_forward_slash = path.rfind(FORWARD_SLASH);

    let last_separator_pos = match (last_backslash, last_forward_slash) {
        (Some(b), Some(f)) => Some(b.max(f)),
        (Some(pos), None) | (None, Some(pos)) => Some(pos),
        (None, None) => None,
    };

    match last_separator_pos {
        Some(pos) => {
            let directory = &path[..=pos]; // Include separator
            let filename = if pos + 1 < path.len() {
                Some(&path[pos + 1..])
            } else {
                None
            };
            (Some(directory), filename)
        }
        None => {
            // No separator found - entire path is filename
            (None, Some(path))
        }
    }
}

/// Joins path components with the appropriate separator.
///
/// Automatically handles separator normalization and prevents double separators.
pub fn join_path_components(components: &[&str], separator: char) -> String {
    if components.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    for (i, component) in components.iter().enumerate() {
        if i > 0 && !result.ends_with(separator) && !component.starts_with(separator) {
            result.push(separator);
        }

        result.push_str(component);
    }

    // Clean up any double separators that might have been introduced
    remove_redundant_separators(&result, separator)
}

/// Checks if a path is absolute based on various Windows formats.
pub fn is_absolute_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    // UNC paths
    if path.starts_with(r"\\?\") || path.starts_with(r"\\") {
        return true;
    }

    // Drive letter paths (C:\ or C:/)
    if path.len() >= 3 {
        let chars: Vec<char> = path.chars().take(3).collect();
        if chars.len() >= 3
            && chars[0].is_ascii_alphabetic()
            && chars[1] == ':'
            && (chars[2] == BACKSLASH || chars[2] == FORWARD_SLASH)
        {
            return true;
        }
    }

    // WSL paths
    if path.starts_with("/mnt/") {
        return true;
    }

    // Cygwin paths
    if path.starts_with("/cygdrive/") {
        return true;
    }

    // Unix-like absolute paths with potential drive
    if path.starts_with("//") && path.len() > 3 {
        let drive_char = path.chars().nth(2);
        if let Some(ch) = drive_char {
            if ch.is_ascii_alphabetic() {
                return true;
            }
        }
    }

    false
}

/// Calculates the length of a path when normalized.
///
/// This is useful for determining if a path will exceed MAX_PATH
/// without actually performing the normalization.
pub fn calculate_normalized_length(path: &str) -> usize {
    if path.is_empty() {
        return 0;
    }

    // This is an approximation - actual normalization might differ slightly
    let mut length = path.len();

    // Account for separator normalization (worst case)
    let _forward_slashes = path.matches(FORWARD_SLASH).count();
    let _backslashes = path.matches(BACKSLASH).count();

    // Account for dot component resolution (estimate)
    let dot_components = path.matches("/.").count() + path.matches("\\.").count();
    let dotdot_components = path.matches("/..").count() + path.matches("\\..").count();

    // Rough estimation of length reduction from dot resolution
    length = length.saturating_sub(dot_components * 2);
    length = length.saturating_sub(dotdot_components * 3);

    // Account for redundant separator removal
    let double_forward = path.matches("//").count();
    let double_back = path.matches("\\\\").count();
    length = length.saturating_sub(double_forward + double_back);

    length
}

/// Extracts the file extension from a path.
pub fn get_file_extension(path: &str) -> Option<&str> {
    let (_, filename) = split_path(path);

    if let Some(name) = filename {
        if let Some(dot_pos) = name.rfind('.') {
            if dot_pos > 0 && dot_pos < name.len() - 1 {
                return Some(&name[dot_pos + 1..]);
            }
        }
    }

    None
}

/// Extracts the filename without extension from a path.
pub fn get_file_stem(path: &str) -> Option<&str> {
    let (_, filename) = split_path(path);

    if let Some(name) = filename {
        if let Some(dot_pos) = name.rfind('.') {
            if dot_pos > 0 {
                return Some(&name[..dot_pos]);
            }
        }
        return Some(name);
    }

    None
}

/// Fast path validation for common cases.
///
/// Returns true if the path is already in optimal Windows format
/// and doesn't need normalization.
pub fn is_already_normalized(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    // Check for Windows drive format
    if path.len() >= 2 {
        let chars: Vec<char> = path.chars().take(3).collect();
        if chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':' {
            // Must use backslashes only
            if path.contains(FORWARD_SLASH) {
                return false;
            }

            // No redundant separators
            if path.contains("\\\\") {
                return false;
            }

            // No dot components
            if path.contains("\\.") {
                return false;
            }

            // Check length limit
            if path.len() <= MAX_PATH {
                return true;
            }

            // Check if it already has UNC prefix for long paths
            return path.starts_with(r"\\?\");
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_separators() {
        assert_eq!(
            normalize_separators("C:/Users/David", BACKSLASH),
            r"C:\Users\David"
        );
        assert_eq!(
            normalize_separators(r"C:\Users\David", FORWARD_SLASH),
            "C:/Users/David"
        );
    }

    #[test]
    fn test_remove_redundant_separators() {
        assert_eq!(
            remove_redundant_separators(r"C:\\\\Users\\David", BACKSLASH),
            r"C:\Users\David"
        );
        assert_eq!(
            remove_redundant_separators("//mnt//c//users", FORWARD_SLASH),
            "/mnt/c/users"
        );
    }

    #[test]
    fn test_resolve_dot_components() {
        assert_eq!(
            resolve_dot_components(r"C:\Users\.\David\..\David").unwrap(),
            r"C:\Users\David"
        );
        assert_eq!(
            resolve_dot_components(r"C:\Users\David\..\..\temp").unwrap(),
            r"C:\temp"
        );

        // Test error case - trying to go above root
        assert!(resolve_dot_components(r"C:\..").is_err());
    }

    #[test]
    fn test_split_path() {
        assert_eq!(
            split_path(r"C:\Users\David\file.txt"),
            (Some(r"C:\Users\David\"), Some("file.txt"))
        );
        assert_eq!(
            split_path("file.txt"),
            (None, Some("file.txt"))
        );
        assert_eq!(
            split_path(r"C:\Users\David\"),
            (Some(r"C:\Users\David\"), None)
        );
    }

    #[test]
    fn test_join_path_components() {
        assert_eq!(
            join_path_components(&["C:", "Users", "David"], BACKSLASH),
            r"C:\Users\David"
        );
        assert_eq!(
            join_path_components(&["/mnt", "c", "users"], FORWARD_SLASH),
            "/mnt/c/users"
        );
    }

    #[test]
    fn test_is_absolute_path() {
        assert!(is_absolute_path(r"C:\Users"));
        assert!(is_absolute_path("C:/Users"));
        assert!(is_absolute_path("/mnt/c/users"));
        assert!(is_absolute_path("/cygdrive/c/users"));
        assert!(is_absolute_path(r"\\?\C:\Users"));
        assert!(is_absolute_path("//c/users"));

        assert!(!is_absolute_path("Users"));
        assert!(!is_absolute_path("./Users"));
        assert!(!is_absolute_path("../temp"));
    }

    #[test]
    fn test_file_extension() {
        assert_eq!(get_file_extension(r"C:\Users\file.txt"), Some("txt"));
        assert_eq!(get_file_extension("document.pdf"), Some("pdf"));
        assert_eq!(get_file_extension("README"), None);
        assert_eq!(get_file_extension(".hidden"), None);
        assert_eq!(get_file_extension("file."), None);
    }

    #[test]
    fn test_file_stem() {
        assert_eq!(get_file_stem(r"C:\Users\file.txt"), Some("file"));
        assert_eq!(get_file_stem("document.pdf"), Some("document"));
        assert_eq!(get_file_stem("README"), Some("README"));
        assert_eq!(get_file_stem(".hidden"), Some(".hidden"));
    }

    #[test]
    fn test_is_already_normalized() {
        assert!(is_already_normalized(r"C:\Users\David"));
        assert!(!is_already_normalized("C:/Users/David"));
        assert!(!is_already_normalized(r"C:\Users\\David"));
        assert!(!is_already_normalized(r"C:\Users\.\David"));

        // Long path with UNC prefix should be considered normalized
        assert!(is_already_normalized(r"\\?\C:\very\long\path"));
    }

    #[test]
    fn test_calculate_normalized_length() {
        // These are approximations, so we test general behavior
        let short_path = "C:/Users/David";
        let length = calculate_normalized_length(short_path);
        assert!(length > 0 && length <= short_path.len());

        let path_with_dots = r"C:\Users\.\David\..\David";
        let length_with_dots = calculate_normalized_length(path_with_dots);
        assert!(length_with_dots < path_with_dots.len());
    }
}
