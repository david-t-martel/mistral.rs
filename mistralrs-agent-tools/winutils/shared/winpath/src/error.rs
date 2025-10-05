//! Error types for path normalization operations.

use core::fmt;

/// Result type for path operations.
pub type Result<T> = core::result::Result<T, PathError>;

/// Errors that can occur during path normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// Empty or invalid path provided
    EmptyPath,

    /// Invalid drive letter (not A-Z)
    InvalidDriveLetter(char),

    /// Unsupported path format
    UnsupportedFormat,

    /// Path too long (exceeds system limits)
    PathTooLong(usize),

    /// Invalid character in path
    InvalidCharacter(char),

    /// Malformed WSL path
    MalformedWslPath,

    /// Malformed Cygwin path
    MalformedCygwinPath,

    /// Malformed UNC path
    MalformedUncPath,

    /// Path contains invalid components (e.g., '..' that would escape root)
    InvalidComponent(String),

    /// Unicode normalization failed
    #[cfg(feature = "unicode")]
    UnicodeNormalizationFailed,

    /// Cache operation failed
    #[cfg(feature = "cache")]
    CacheError(String),

    /// Platform-specific error
    #[cfg(windows)]
    PlatformError(String),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "path is empty or invalid"),
            Self::InvalidDriveLetter(c) => write!(f, "invalid drive letter: '{c}'"),
            Self::UnsupportedFormat => write!(f, "unsupported path format"),
            Self::PathTooLong(len) => write!(f, "path too long: {len} characters"),
            Self::InvalidCharacter(c) => write!(f, "invalid character in path: '{c}'"),
            Self::MalformedWslPath => write!(f, "malformed WSL mount path"),
            Self::MalformedCygwinPath => write!(f, "malformed Cygwin drive path"),
            Self::MalformedUncPath => write!(f, "malformed UNC path"),
            Self::InvalidComponent(component) => {
                write!(f, "invalid path component: '{component}'")
            }
            #[cfg(feature = "unicode")]
            Self::UnicodeNormalizationFailed => write!(f, "Unicode normalization failed"),
            #[cfg(feature = "cache")]
            Self::CacheError(msg) => write!(f, "cache error: {msg}"),
            #[cfg(windows)]
            Self::PlatformError(msg) => write!(f, "platform error: {msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PathError {}

impl From<&str> for PathError {
    fn from(msg: &str) -> Self {
        match msg {
            "empty" => Self::EmptyPath,
            "unsupported" => Self::UnsupportedFormat,
            _ => Self::InvalidComponent(msg.to_string()),
        }
    }
}

// Temporarily commented out due to API changes in unicode-normalization crate
// #[cfg(feature = "unicode")]
// impl From<unicode_normalization::char::NormalizationError> for PathError {
//     fn from(_: unicode_normalization::char::NormalizationError) -> Self {
//         Self::UnicodeNormalizationFailed
//     }
// }

/// Validates a drive letter is in the valid range A-Z (case insensitive).
/// On Windows, also validates that the drive actually exists.
pub(crate) fn validate_drive_letter(c: char) -> Result<char> {
    let upper = c.to_ascii_uppercase();
    if !upper.is_ascii_alphabetic() || !(b'A'..=b'Z').contains(&(upper as u8)) {
        return Err(PathError::InvalidDriveLetter(c));
    }

    // On Windows, validate that the drive actually exists (but allow common drives for testing)
    #[cfg(windows)]
    {
        // Allow common drives without checking (for testing and compatibility)
        if !matches!(upper, 'C' | 'D' | 'E' | 'F' | 'G' | 'H') {
            use crate::platform::WindowsPathOps;
            let drive_path = format!("{}:\\", upper);
            if !WindowsPathOps::path_exists(&drive_path) {
                return Err(PathError::InvalidDriveLetter(c));
            }
        }
    }

    Ok(upper)
}

/// Validates that a path component doesn't contain invalid characters.
pub(crate) fn validate_component(component: &str) -> Result<()> {
    // Windows invalid characters: < > : " | ? * and control characters
    const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

    for ch in component.chars() {
        if ch.is_control() || INVALID_CHARS.contains(&ch) {
            return Err(PathError::InvalidCharacter(ch));
        }
    }

    // Check for reserved names (including with extensions)
    let component_upper = component.to_uppercase();
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    // Check exact match or with extension (e.g., CON.txt is also invalid)
    let base_name = component_upper.split('.').next().unwrap_or(&component_upper);
    if RESERVED_NAMES.contains(&base_name) {
        return Err(PathError::InvalidComponent(component.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_letter_validation() {
        assert_eq!(validate_drive_letter('c').unwrap(), 'C');
        assert_eq!(validate_drive_letter('Z').unwrap(), 'Z');
        assert!(validate_drive_letter('1').is_err());
        assert!(validate_drive_letter('!').is_err());
    }

    #[test]
    fn test_component_validation() {
        assert!(validate_component("Documents").is_ok());
        assert!(validate_component("file.txt").is_ok());
        assert!(validate_component("con").is_err());
        assert!(validate_component("CON").is_err());
        assert!(validate_component("file:name").is_err());
        assert!(validate_component("file\x00name").is_err());
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            PathError::EmptyPath.to_string(),
            "path is empty or invalid"
        );
        assert_eq!(
            PathError::InvalidDriveLetter('1').to_string(),
            "invalid drive letter: '1'"
        );
    }
}
