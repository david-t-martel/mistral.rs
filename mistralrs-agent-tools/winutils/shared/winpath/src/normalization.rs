//! Core path normalization algorithms.

use crate::{
    constants::*,
    detection::{detect_path_format, extract_drive_letter, PathFormat},
    error::{validate_component, validate_drive_letter, PathError, Result},
    utils::{normalize_separators, remove_redundant_separators, resolve_dot_components},
};
use alloc::{borrow::Cow, string::String, vec::Vec};

/// Result of path normalization containing the normalized path and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationResult<'a> {
    /// The normalized path
    path: Cow<'a, str>,
    /// Original format detected
    original_format: PathFormat,
    /// Whether long path prefix was added
    has_long_prefix: bool,
    /// Whether the path was modified during normalization
    was_modified: bool,
}

impl<'a> NormalizationResult<'a> {
    /// Creates a new normalization result.
    pub fn new(
        path: Cow<'a, str>,
        original_format: PathFormat,
        has_long_prefix: bool,
        was_modified: bool,
    ) -> Self {
        Self {
            path,
            original_format,
            has_long_prefix,
            was_modified,
        }
    }

    /// Returns the normalized path as a string slice.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the normalized path, consuming the result.
    pub fn into_path(self) -> Cow<'a, str> {
        self.path
    }

    /// Returns the original path format that was detected.
    pub fn original_format(&self) -> PathFormat {
        self.original_format
    }

    /// Returns true if a long path prefix was added during normalization.
    pub fn has_long_path_prefix(&self) -> bool {
        self.has_long_prefix
    }

    /// Returns true if the path was modified during normalization.
    pub fn was_modified(&self) -> bool {
        self.was_modified
    }

    /// Returns the length of the normalized path.
    pub fn len(&self) -> usize {
        self.path.len()
    }

    /// Returns true if the normalized path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
}

/// Normalizes a path to Windows format, returning a borrowed string when possible.
///
/// This is the main normalization function that handles all supported path formats
/// and returns a `Cow<str>` for zero-copy optimization when the input is already
/// in the correct format.
///
/// # Examples
///
/// ```rust
/// use winpath::normalize_path_cow;
///
/// // No allocation needed - input is already normalized
/// let result = normalize_path_cow(r"C:\Users\David")?;
/// assert!(!result.was_modified());
///
/// // Allocation needed - input requires normalization
/// let result = normalize_path_cow("/mnt/c/users/david")?;
/// assert!(result.was_modified());
/// assert_eq!(result.path(), r"C:\users\david");
/// ```
pub fn normalize_path_cow(input: &str) -> Result<NormalizationResult<'_>> {
    if input.is_empty() {
        return Err(PathError::EmptyPath);
    }

    let format = detect_path_format(input);

    match format {
        PathFormat::Dos => normalize_dos_cow(input, format),
        PathFormat::DosForward => normalize_dos_forward_cow(input),
        PathFormat::Wsl => normalize_wsl_cow(input),
        PathFormat::Cygwin => normalize_cygwin_cow(input),
        PathFormat::Unc => normalize_unc_cow(input),
        PathFormat::UnixLike => normalize_unix_like_cow(input),
        PathFormat::GitBashMangled => normalize_git_bash_mangled_cow(input),
        PathFormat::Mixed => normalize_mixed_cow(input),
        PathFormat::Relative => normalize_relative_cow(input),
        PathFormat::Unknown => Err(PathError::UnsupportedFormat),
    }
}

/// Normalizes a path to Windows format, always returning an owned string.
///
/// This is a convenience function that always returns a `String`. Use
/// `normalize_path_cow` for better performance when you want to avoid
/// allocations for already-normalized paths.
///
/// # Examples
///
/// ```rust
/// use winpath::normalize_path;
///
/// let result = normalize_path("/mnt/c/users/david")?;
/// assert_eq!(result, r"C:\users\david");
/// ```
pub fn normalize_path(input: &str) -> Result<String> {
    normalize_path_cow(input).map(|result| result.into_path().into_owned())
}

/// Normalizes a DOS path, potentially avoiding allocation.
fn normalize_dos_cow(input: &str, format: PathFormat) -> Result<NormalizationResult<'_>> {
    // Validate drive letter
    if let Some(drive) = extract_drive_letter(input, format) {
        validate_drive_letter(drive)?;
    }

    // Check if already properly normalized
    if is_normalized_dos_path(input) {
        // Still need to validate components even if path is normalized
        validate_path_components(input)?;

        let needs_long_prefix = input.len() > MAX_PATH;
        if needs_long_prefix {
            let long_path = format!("{}{}", UNC_PREFIX, input);
            return Ok(NormalizationResult::new(
                Cow::Owned(long_path),
                format,
                true,
                true,
            ));
        }
        return Ok(NormalizationResult::new(
            Cow::Borrowed(input),
            format,
            false,
            false,
        ));
    }

    // Normalize the path
    let normalized = normalize_dos_path_owned(input)?;
    let needs_long_prefix = normalized.len() > MAX_PATH;

    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, normalized)
    } else {
        normalized
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        format,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes a DOS path with forward slashes.
fn normalize_dos_forward_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let drive = extract_drive_letter(input, PathFormat::DosForward)
        .ok_or(PathError::MalformedUncPath)?;
    validate_drive_letter(drive)?;

    let normalized = normalize_separators(input, BACKSLASH);
    let normalized = resolve_dot_components(&normalized)?;
    let normalized = remove_redundant_separators(&normalized, BACKSLASH);

    validate_path_components(&normalized)?;

    let needs_long_prefix = normalized.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, normalized)
    } else {
        normalized
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::DosForward,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes a WSL mount path.
fn normalize_wsl_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let path_without_prefix = input
        .strip_prefix(WSL_PREFIX)
        .ok_or(PathError::MalformedWslPath)?;

    if path_without_prefix.is_empty() {
        return Err(PathError::MalformedWslPath);
    }

    let mut parts: Vec<&str> = path_without_prefix.split('/').collect();

    // First part should be the drive letter
    if parts.is_empty() {
        return Err(PathError::MalformedWslPath);
    }

    let drive_str = parts.remove(0);
    if drive_str.len() != 1 {
        return Err(PathError::MalformedWslPath);
    }

    let drive = drive_str
        .chars()
        .next()
        .ok_or(PathError::MalformedWslPath)?;
    let drive = validate_drive_letter(drive)?;

    // Build Windows path
    let mut result = String::with_capacity(input.len());
    result.push(drive);
    result.push(':');

    if !parts.is_empty() {
        result.push(BACKSLASH);
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                validate_component(part)?;
                result.push_str(part);
                if i < parts.len() - 1 {
                    result.push(BACKSLASH);
                }
            }
        }
    }

    let needs_long_prefix = result.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, result)
    } else {
        result
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::Wsl,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes a Cygwin drive path.
fn normalize_cygwin_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let path_without_prefix = input
        .strip_prefix(CYGWIN_PREFIX)
        .ok_or(PathError::MalformedCygwinPath)?;

    if path_without_prefix.is_empty() {
        return Err(PathError::MalformedCygwinPath);
    }

    let mut parts: Vec<&str> = path_without_prefix.split('/').collect();

    // First part should be the drive letter
    if parts.is_empty() {
        return Err(PathError::MalformedCygwinPath);
    }

    let drive_str = parts.remove(0);
    if drive_str.len() != 1 {
        return Err(PathError::MalformedCygwinPath);
    }

    let drive = drive_str
        .chars()
        .next()
        .ok_or(PathError::MalformedCygwinPath)?;
    let drive = validate_drive_letter(drive)?;

    // Build Windows path
    let mut result = String::with_capacity(input.len());
    result.push(drive);
    result.push(':');

    if !parts.is_empty() {
        result.push(BACKSLASH);
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                validate_component(part)?;
                result.push_str(part);
                if i < parts.len() - 1 {
                    result.push(BACKSLASH);
                }
            }
        }
    }

    let needs_long_prefix = result.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, result)
    } else {
        result
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::Cygwin,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes a UNC path.
fn normalize_unc_cow(input: &str) -> Result<NormalizationResult<'_>> {
    // UNC paths are already in long format, just validate and clean up
    if !input.starts_with(UNC_PREFIX) {
        return Err(PathError::MalformedUncPath);
    }

    let inner_path = &input[UNC_PREFIX.len()..];
    if inner_path.is_empty() {
        return Err(PathError::MalformedUncPath);
    }

    // If it's a UNC network path (\\?\UNC\), handle differently
    if inner_path.starts_with("UNC\\") {
        // Network UNC path - validate but don't change much
        validate_unc_network_path(inner_path)?;
        return Ok(NormalizationResult::new(
            Cow::Borrowed(input),
            PathFormat::Unc,
            true,
            false,
        ));
    }

    // Regular long path - normalize the inner path
    let normalized_inner = normalize_dos_path_owned(inner_path)?;
    let final_path = format!("{}{}", UNC_PREFIX, normalized_inner);

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::Unc,
        true,
        true,
    ))
}

/// Normalizes Unix-like paths with double slashes.
fn normalize_unix_like_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let trimmed = input.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(PathError::UnsupportedFormat);
    }

    let parts: Vec<&str> = trimmed.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(PathError::UnsupportedFormat);
    }

    // First part should be drive letter
    let drive_str = parts[0];
    if drive_str.len() != 1 {
        return Err(PathError::UnsupportedFormat);
    }

    let drive = drive_str
        .chars()
        .next()
        .ok_or(PathError::UnsupportedFormat)?;
    let drive = validate_drive_letter(drive)?;

    // Build Windows path
    let mut result = String::with_capacity(input.len());
    result.push(drive);
    result.push(':');

    if parts.len() > 1 {
        result.push(BACKSLASH);
        for (i, part) in parts.iter().skip(1).enumerate() {
            validate_component(part)?;
            result.push_str(part);
            if i < parts.len() - 2 {
                result.push(BACKSLASH);
            }
        }
    }

    let needs_long_prefix = result.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, result)
    } else {
        result
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::UnixLike,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes a Git Bash mangled path.
/// Converts paths like `C:\Program Files\Git\mnt\c\users\david` to `C:\users\david`
fn normalize_git_bash_mangled_cow(input: &str) -> Result<NormalizationResult<'_>> {
    // Find which Git Bash prefix matches
    let mut stripped_path = None;
    for prefix in GIT_BASH_PREFIXES {
        if input.starts_with(prefix) {
            let after_prefix = &input[prefix.len()..];
            if after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN) {
                // Remove the Git prefix and the \mnt\ part
                stripped_path = Some(&after_prefix[GIT_BASH_MANGLE_PATTERN.len()..]);
                break;
            } else if after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN_ALT) {
                // Remove the Git prefix and the /mnt/ part
                stripped_path = Some(&after_prefix[GIT_BASH_MANGLE_PATTERN_ALT.len()..]);
                break;
            }
        }
    }

    let clean_path = stripped_path.ok_or(PathError::UnsupportedFormat)?;

    // Now we should have something like "c\users\david" or "c/users/david"
    // Extract the drive letter
    if clean_path.is_empty() {
        return Err(PathError::EmptyPath);
    }

    let drive_char = clean_path.chars().next()
        .ok_or(PathError::MalformedWslPath)?;
    let drive = validate_drive_letter(drive_char)?;

    // Skip the drive letter and separator
    let after_drive = if clean_path.len() > 1 {
        let second_char = clean_path.chars().nth(1);
        if second_char == Some('\\') || second_char == Some('/') {
            &clean_path[2..]
        } else {
            &clean_path[1..]
        }
    } else {
        ""
    };

    // Build the normalized Windows path
    let mut result = String::with_capacity(clean_path.len() + 3);
    result.push(drive);
    result.push(':');
    result.push(BACKSLASH);

    // Process the remaining path parts
    if !after_drive.is_empty() {
        let normalized = normalize_separators(after_drive, BACKSLASH);
        let parts: Vec<&str> = normalized.split(BACKSLASH).filter(|s| !s.is_empty()).collect();

        for (i, part) in parts.iter().enumerate() {
            validate_component(part)?;
            result.push_str(part);
            if i < parts.len() - 1 {
                result.push(BACKSLASH);
            }
        }
    }

    let needs_long_prefix = result.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, result)
    } else {
        result
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::GitBashMangled,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes paths with mixed separators.
fn normalize_mixed_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let normalized = normalize_separators(input, BACKSLASH);
    let normalized = resolve_dot_components(&normalized)?;
    let normalized = remove_redundant_separators(&normalized, BACKSLASH);

    validate_path_components(&normalized)?;

    let needs_long_prefix = normalized.len() > MAX_PATH;
    let final_path = if needs_long_prefix {
        format!("{}{}", UNC_PREFIX, normalized)
    } else {
        normalized
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::Mixed,
        needs_long_prefix,
        true,
    ))
}

/// Normalizes relative paths.
fn normalize_relative_cow(input: &str) -> Result<NormalizationResult<'_>> {
    let normalized = normalize_separators(input, BACKSLASH);
    let normalized = resolve_dot_components(&normalized)?;
    let normalized = remove_redundant_separators(&normalized, BACKSLASH);

    validate_path_components(&normalized)?;

    let was_modified = normalized != input;

    Ok(NormalizationResult::new(
        if was_modified {
            Cow::Owned(normalized)
        } else {
            Cow::Borrowed(input)
        },
        PathFormat::Relative,
        false,
        was_modified,
    ))
}

/// Helper functions

/// Checks if a DOS path is already properly normalized.
fn is_normalized_dos_path(path: &str) -> bool {
    // Must have drive letter and colon
    if path.len() < 2 || !path.chars().nth(0).unwrap().is_ascii_alphabetic() || path.chars().nth(1) != Some(':') {
        return false;
    }

    // Check for proper separators (only backslashes)
    if path.contains('/') {
        return false;
    }

    // Check for redundant separators
    if path.contains("\\\\") {
        return false;
    }

    // Check for dot components
    if path.contains("\\..\\") || path.contains("\\.\\") || path.ends_with("\\..") || path.ends_with("\\.") {
        return false;
    }

    true
}

/// Normalizes a DOS path, returning an owned string.
fn normalize_dos_path_owned(input: &str) -> Result<String> {
    let normalized = normalize_separators(input, BACKSLASH);
    let normalized = resolve_dot_components(&normalized)?;
    let normalized = remove_redundant_separators(&normalized, BACKSLASH);
    validate_path_components(&normalized)?;
    Ok(normalized)
}

/// Validates all components in a path.
fn validate_path_components(path: &str) -> Result<()> {
    // Skip drive letter if present
    let start_idx = if path.len() >= 2 && path.chars().nth(1) == Some(':') { 3 } else { 0 };

    if start_idx >= path.len() {
        return Ok(());
    }

    for component in path[start_idx..].split(BACKSLASH) {
        if !component.is_empty() {
            validate_component(component)?;
        }
    }

    Ok(())
}

/// Validates a UNC network path.
fn validate_unc_network_path(path: &str) -> Result<()> {
    // UNC network paths have format: UNC\server\share\path...
    let without_unc = path.strip_prefix("UNC\\").ok_or(PathError::MalformedUncPath)?;
    let parts: Vec<&str> = without_unc.split('\\').collect();

    if parts.len() < 2 {
        return Err(PathError::MalformedUncPath);
    }

    // Validate server and share names
    validate_component(parts[0])?;
    validate_component(parts[1])?;

    // Validate remaining path components
    for component in parts.iter().skip(2) {
        if !component.is_empty() {
            validate_component(component)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_cow_dos() {
        // Already normalized - should not allocate
        let result = normalize_path_cow(r"C:\Users\David").unwrap();
        assert!(!result.was_modified());
        assert_eq!(result.path(), r"C:\Users\David");

        // Needs normalization
        let result = normalize_path_cow(r"C:\Users\.\David\..\David").unwrap();
        assert!(result.was_modified());
        assert_eq!(result.path(), r"C:\Users\David");
    }

    #[test]
    fn test_normalize_wsl_paths() {
        let result = normalize_path_cow("/mnt/c/users/david").unwrap();
        assert_eq!(result.path(), r"C:\users\david");
        assert!(result.was_modified());

        let result = normalize_path_cow("/mnt/d/Program Files/app").unwrap();
        assert_eq!(result.path(), r"D:\Program Files\app");
    }

    #[test]
    fn test_normalize_cygwin_paths() {
        let result = normalize_path_cow("/cygdrive/c/users/david").unwrap();
        assert_eq!(result.path(), r"C:\users\david");

        let result = normalize_path_cow("/cygdrive/f/temp/file.txt").unwrap();
        assert_eq!(result.path(), r"F:\temp\file.txt");
    }

    #[test]
    fn test_long_path_handling() {
        let long_component = "a".repeat(200);
        let long_path = format!(r"C:\{}\file.txt", long_component);

        let result = normalize_path_cow(&long_path).unwrap();
        assert!(result.has_long_path_prefix());
        assert!(result.path().starts_with(UNC_PREFIX));
    }

    #[test]
    fn test_mixed_separators() {
        let result = normalize_path_cow(r"C:\Users/David\Documents/file.txt").unwrap();
        assert_eq!(result.path(), r"C:\Users\David\Documents\file.txt");
        assert!(result.was_modified());
    }

    #[test]
    fn test_error_cases() {
        assert!(normalize_path_cow("").is_err());
        assert!(normalize_path_cow("/mnt/").is_err());
        assert!(normalize_path_cow("/mnt/invalid_drive/test").is_err());
        assert!(normalize_path_cow(r"C:\con").is_err()); // Reserved name
    }

    #[test]
    fn test_unc_path_normalization() {
        let result = normalize_path_cow(r"\\?\C:\Users\David").unwrap();
        assert!(result.has_long_path_prefix());
        assert_eq!(result.original_format(), PathFormat::Unc);

        // UNC network path
        let result = normalize_path_cow(r"\\?\UNC\server\share\file").unwrap();
        assert!(result.has_long_path_prefix());
        assert!(!result.was_modified()); // Should be left as-is
    }

    #[test]
    fn test_unix_like_paths() {
        let result = normalize_path_cow("//c/users/david").unwrap();
        assert_eq!(result.path(), r"C:\users\david");

        let result = normalize_path_cow("//d//temp//file.txt").unwrap();
        assert_eq!(result.path(), r"D:\temp\file.txt");
    }

    #[test]
    fn test_relative_path_normalization() {
        let result = normalize_path_cow(r"Documents\file.txt").unwrap();
        assert_eq!(result.original_format(), PathFormat::Relative);
        assert!(!result.has_long_path_prefix());

        let result = normalize_path_cow(r".\Documents\..\Documents\file.txt").unwrap();
        assert_eq!(result.path(), r"Documents\file.txt");
        assert!(result.was_modified());
    }
}
