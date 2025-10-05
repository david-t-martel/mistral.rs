//! Path format detection and classification.

use crate::constants::*;

/// Supported path formats for Windows path normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PathFormat {
    /// Standard DOS path: `C:\Users\David`
    Dos,
    /// DOS path with forward slashes: `C:/Users/David`
    DosForward,
    /// WSL mount point: `/mnt/c/users/david`
    Wsl,
    /// Cygwin drive: `/cygdrive/c/users/david`
    Cygwin,
    /// UNC long path: `\\?\C:\Users\David`
    Unc,
    /// Unix-like with multiple slashes: `//c//users//david//`
    UnixLike,
    /// Git Bash mangled path: `C:\Program Files\Git\mnt\c\users\david`
    GitBashMangled,
    /// Relative path (no drive/mount info)
    Relative,
    /// Mixed format (combination of separators)
    Mixed,
    /// Unknown or unsupported format
    Unknown,
}

impl PathFormat {
    /// Returns true if this format represents an absolute Windows path.
    pub const fn is_absolute(self) -> bool {
        matches!(
            self,
            Self::Dos | Self::DosForward | Self::Wsl | Self::Cygwin | Self::Unc | Self::UnixLike | Self::GitBashMangled
        )
    }

    /// Returns true if this format uses Unix-style separators.
    pub const fn uses_unix_separators(self) -> bool {
        matches!(self, Self::Wsl | Self::Cygwin | Self::UnixLike)
    }

    /// Returns true if this format requires special handling for long paths.
    pub const fn requires_long_path_prefix(self) -> bool {
        matches!(self, Self::Dos | Self::DosForward | Self::Mixed)
    }

    /// Returns the canonical separator for this format.
    pub const fn canonical_separator(self) -> char {
        match self {
            Self::Dos | Self::DosForward | Self::Unc | Self::Mixed | Self::GitBashMangled => BACKSLASH,
            Self::Wsl | Self::Cygwin | Self::UnixLike => FORWARD_SLASH,
            Self::Relative | Self::Unknown => BACKSLASH, // Default to Windows
        }
    }
}

/// Detects the format of a path string.
///
/// This function analyzes the structure and patterns in a path to determine
/// its format. It uses fast pattern matching optimized for common cases.
///
/// # Examples
///
/// ```rust
/// use winpath::{detect_path_format, PathFormat};
///
/// assert_eq!(detect_path_format(r"C:\Users"), PathFormat::Dos);
/// assert_eq!(detect_path_format("C:/Users"), PathFormat::DosForward);
/// assert_eq!(detect_path_format("/mnt/c/users"), PathFormat::Wsl);
/// assert_eq!(detect_path_format("/cygdrive/c/users"), PathFormat::Cygwin);
/// assert_eq!(detect_path_format(r"\\?\C:\Users"), PathFormat::Unc);
/// ```
pub fn detect_path_format(path: &str) -> PathFormat {
    if path.is_empty() {
        return PathFormat::Unknown;
    }

    // Fast path: check for common prefixes first
    if let Some(format) = detect_by_prefix(path) {
        return format;
    }

    // Check for mixed separators first (before DOS detection)
    if has_mixed_separators(path) {
        return PathFormat::Mixed;
    }

    // Check for DOS paths (drive letter + colon)
    if let Some(format) = detect_dos_format(path) {
        return format;
    }

    // Check for Unix-like patterns
    if let Some(format) = detect_unix_like_format(path) {
        return format;
    }

    // Default to relative if no absolute format detected
    if path.contains(BACKSLASH) || path.contains(FORWARD_SLASH) {
        PathFormat::Relative
    } else {
        PathFormat::Unknown
    }
}

/// Fast prefix-based detection for common formats.
#[inline]
fn detect_by_prefix(path: &str) -> Option<PathFormat> {
    let bytes = path.as_bytes();

    // UNC prefix: \\?\
    if bytes.len() >= 4 && bytes.starts_with(b"\\\\?\\") {
        return Some(PathFormat::Unc);
    }

    // Check for Git Bash mangled paths
    // These contain Git installation path followed by \mnt\ or /mnt/
    for prefix in GIT_BASH_PREFIXES {
        if path.starts_with(prefix) {
            let after_prefix = &path[prefix.len()..];
            if after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN) ||
               after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN_ALT) {
                return Some(PathFormat::GitBashMangled);
            }
        }
    }

    // WSL prefix: /mnt/
    if bytes.len() >= 5 && bytes.starts_with(b"/mnt/") {
        return Some(PathFormat::Wsl);
    }

    // Cygwin prefix: /cygdrive/
    if bytes.len() >= 10 && bytes.starts_with(b"/cygdrive/") {
        return Some(PathFormat::Cygwin);
    }

    // Unix-like with double slashes: //
    if bytes.len() >= 2 && bytes.starts_with(b"//") {
        return Some(PathFormat::UnixLike);
    }

    None
}

/// Detects DOS format variations (C:\ or C:/).
#[inline]
fn detect_dos_format(path: &str) -> Option<PathFormat> {
    let bytes = path.as_bytes();

    // Need at least "C:" (2 chars)
    if bytes.len() < 2 {
        return None;
    }

    // Check for drive letter pattern: [A-Z]:
    if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        // Check separator type if present
        if bytes.len() > 2 {
            match bytes[2] {
                b'\\' => return Some(PathFormat::Dos),
                b'/' => return Some(PathFormat::DosForward),
                _ => {} // Could be relative like "C:file.txt"
            }
        }
        // Drive letter with colon but no separator
        return Some(PathFormat::Dos);
    }

    None
}

/// Detects Unix-like formats that aren't WSL or Cygwin.
#[inline]
fn detect_unix_like_format(path: &str) -> Option<PathFormat> {
    let bytes = path.as_bytes();

    // Must start with /
    if !bytes.starts_with(b"/") {
        return None;
    }

    // Skip if already detected as WSL or Cygwin
    if bytes.starts_with(b"/mnt/") || bytes.starts_with(b"/cygdrive/") {
        return None;
    }

    // Check for Unix-like pattern with potential drive letter
    // Examples: //c/users, /c/users (less common)
    if bytes.len() >= 3 {
        let start_idx = if bytes.starts_with(b"//") { 2 } else { 1 };

        if bytes.len() > start_idx + 1
            && bytes[start_idx].is_ascii_alphabetic()
            && (bytes[start_idx + 1] == b'/' || start_idx + 1 == bytes.len())
        {
            return Some(PathFormat::UnixLike);
        }
    }

    // Generic Unix path without drive info
    Some(PathFormat::Relative)
}

/// Checks if a path contains mixed separators.
#[inline]
fn has_mixed_separators(path: &str) -> bool {
    let has_backslash = memchr::memchr(b'\\', path.as_bytes()).is_some();
    let has_forward_slash = memchr::memchr(b'/', path.as_bytes()).is_some();
    has_backslash && has_forward_slash
}

/// Extracts drive letter from various path formats.
///
/// Returns the drive letter in uppercase if found.
pub fn extract_drive_letter(path: &str, format: PathFormat) -> Option<char> {
    match format {
        PathFormat::Dos | PathFormat::DosForward | PathFormat::Mixed => {
            let bytes = path.as_bytes();
            if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
                Some((bytes[0] as char).to_ascii_uppercase())
            } else {
                None
            }
        }
        PathFormat::Unc => {
            // Extract from \\?\C:\...
            let path = path.strip_prefix(UNC_PREFIX)?;
            if let Some(first_char) = path.chars().next() {
                if first_char.is_ascii_alphabetic() {
                    return Some(first_char.to_ascii_uppercase());
                }
            }
            None
        }
        PathFormat::Wsl => {
            // Extract from /mnt/c/...
            let path = path.strip_prefix(WSL_PREFIX)?;
            if let Some(first_char) = path.chars().next() {
                if first_char.is_ascii_alphabetic() {
                    return Some(first_char.to_ascii_uppercase());
                }
            }
            None
        }
        PathFormat::Cygwin => {
            // Extract from /cygdrive/c/...
            let path = path.strip_prefix(CYGWIN_PREFIX)?;
            if let Some(first_char) = path.chars().next() {
                if first_char.is_ascii_alphabetic() {
                    return Some(first_char.to_ascii_uppercase());
                }
            }
            None
        }
        PathFormat::UnixLike => {
            // Extract from //c/... or similar
            let path = path.trim_start_matches('/');
            if let Some(first_char) = path.chars().next() {
                if first_char.is_ascii_alphabetic() {
                    return Some(first_char.to_ascii_uppercase());
                }
            }
            None
        }
        PathFormat::GitBashMangled => {
            // Extract from paths like C:\Program Files\Git\mnt\c\users
            // Find the Git prefix and strip it
            for prefix in GIT_BASH_PREFIXES {
                if path.starts_with(prefix) {
                    let after_prefix = &path[prefix.len()..];
                    // Look for \mnt\X\ or /mnt/X/
                    if after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN) {
                        let after_mnt = &after_prefix[GIT_BASH_MANGLE_PATTERN.len()..];
                        if let Some(drive_char) = after_mnt.chars().next() {
                            if drive_char.is_ascii_alphabetic() {
                                return Some(drive_char.to_ascii_uppercase());
                            }
                        }
                    } else if after_prefix.starts_with(GIT_BASH_MANGLE_PATTERN_ALT) {
                        let after_mnt = &after_prefix[GIT_BASH_MANGLE_PATTERN_ALT.len()..];
                        if let Some(drive_char) = after_mnt.chars().next() {
                            if drive_char.is_ascii_alphabetic() {
                                return Some(drive_char.to_ascii_uppercase());
                            }
                        }
                    }
                }
            }
            None
        }
        PathFormat::Relative | PathFormat::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        // DOS paths
        assert_eq!(detect_path_format(r"C:\Users"), PathFormat::Dos);
        assert_eq!(detect_path_format(r"D:\Program Files"), PathFormat::Dos);

        // DOS forward slash
        assert_eq!(detect_path_format("C:/Users"), PathFormat::DosForward);
        assert_eq!(detect_path_format("E:/temp"), PathFormat::DosForward);

        // WSL paths
        assert_eq!(detect_path_format("/mnt/c/users"), PathFormat::Wsl);
        assert_eq!(detect_path_format("/mnt/d/temp"), PathFormat::Wsl);

        // Cygwin paths
        assert_eq!(detect_path_format("/cygdrive/c/users"), PathFormat::Cygwin);
        assert_eq!(detect_path_format("/cygdrive/f/data"), PathFormat::Cygwin);

        // UNC paths
        assert_eq!(detect_path_format(r"\\?\C:\Users"), PathFormat::Unc);
        assert_eq!(detect_path_format(r"\\?\UNC\server\share"), PathFormat::Unc);

        // Unix-like
        assert_eq!(detect_path_format("//c/users"), PathFormat::UnixLike);
        assert_eq!(detect_path_format("//d/temp//file"), PathFormat::UnixLike);

        // Mixed separators
        assert_eq!(detect_path_format(r"C:\Users/David"), PathFormat::Mixed);
        assert_eq!(detect_path_format("C:/Users\\Documents"), PathFormat::Mixed);

        // Relative paths
        assert_eq!(detect_path_format("Documents\\file.txt"), PathFormat::Relative);
        assert_eq!(detect_path_format("../temp"), PathFormat::Relative);

        // Edge cases
        assert_eq!(detect_path_format(""), PathFormat::Unknown);
        assert_eq!(detect_path_format("C:"), PathFormat::Dos);
        assert_eq!(detect_path_format("/"), PathFormat::Relative);
    }

    #[test]
    fn test_drive_letter_extraction() {
        assert_eq!(extract_drive_letter(r"C:\Users", PathFormat::Dos), Some('C'));
        assert_eq!(extract_drive_letter("c:/temp", PathFormat::DosForward), Some('C'));
        assert_eq!(extract_drive_letter("/mnt/d/data", PathFormat::Wsl), Some('D'));
        assert_eq!(extract_drive_letter("/cygdrive/f/temp", PathFormat::Cygwin), Some('F'));
        assert_eq!(extract_drive_letter(r"\\?\G:\data", PathFormat::Unc), Some('G'));
        assert_eq!(extract_drive_letter("//h/users", PathFormat::UnixLike), Some('H'));
        assert_eq!(extract_drive_letter("relative/path", PathFormat::Relative), None);
    }

    #[test]
    fn test_format_properties() {
        assert!(PathFormat::Dos.is_absolute());
        assert!(PathFormat::Wsl.uses_unix_separators());
        assert!(PathFormat::Dos.requires_long_path_prefix());
        assert_eq!(PathFormat::Dos.canonical_separator(), '\\');
        assert_eq!(PathFormat::Wsl.canonical_separator(), '/');
    }

    #[test]
    fn test_mixed_separator_detection() {
        assert!(has_mixed_separators(r"C:\Users/David"));
        assert!(has_mixed_separators("C:/Users\\Documents"));
        assert!(!has_mixed_separators(r"C:\Users\David"));
        assert!(!has_mixed_separators("C:/Users/David"));
    }

    #[test]
    fn test_prefix_detection() {
        assert_eq!(detect_by_prefix(r"\\?\C:\"), Some(PathFormat::Unc));
        assert_eq!(detect_by_prefix("/mnt/c/"), Some(PathFormat::Wsl));
        assert_eq!(detect_by_prefix("/cygdrive/c/"), Some(PathFormat::Cygwin));
        assert_eq!(detect_by_prefix("//c/"), Some(PathFormat::UnixLike));
        assert_eq!(detect_by_prefix("regular/path"), None);
    }
}
