//! Optimized path format detection with SIMD and branchless operations.

use crate::constants::*;
use arrayvec::ArrayVec;
use lazy_static::lazy_static;
use smallvec::SmallVec;

#[cfg(feature = "simd")]
use packed_simd_2::{u8x16, u8x32, u8x64};

// Lookup tables for fast path detection
lazy_static! {
    /// Pre-computed drive letter lookup table (0-255 -> bool)
    static ref DRIVE_LETTER_TABLE: [bool; 256] = {
        let mut table = [false; 256];
        for c in b'A'..=b'Z' {
            table[c as usize] = true;
        }
        for c in b'a'..=b'z' {
            table[c as usize] = true;
        }
        table
    };

    /// Pre-computed separator lookup table
    static ref SEPARATOR_TABLE: [bool; 256] = {
        let mut table = [false; 256];
        table[b'/' as usize] = true;
        table[b'\\' as usize] = true;
        table
    };

    /// Pre-computed prefix patterns for fast matching
    static ref PREFIX_PATTERNS: Vec<(&'static [u8], PathFormat)> = vec![
        (b"\\\\?\\", PathFormat::Unc),
        (b"/mnt/", PathFormat::Wsl),
        (b"/cygdrive/", PathFormat::Cygwin),
        (b"//", PathFormat::UnixLike),
    ];
}

/// Supported path formats for Windows path normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]  // Compact representation for cache efficiency
pub enum PathFormat {
    Dos = 0,
    DosForward = 1,
    Wsl = 2,
    Cygwin = 3,
    Unc = 4,
    UnixLike = 5,
    GitBashMangled = 6,
    Relative = 7,
    Mixed = 8,
    Unknown = 9,
}

impl PathFormat {
    /// Returns true if this format represents an absolute Windows path (branchless).
    #[inline(always)]
    pub const fn is_absolute(self) -> bool {
        let mask = (1 << Self::Dos as u8) |
                  (1 << Self::DosForward as u8) |
                  (1 << Self::Wsl as u8) |
                  (1 << Self::Cygwin as u8) |
                  (1 << Self::Unc as u8) |
                  (1 << Self::UnixLike as u8) |
                  (1 << Self::GitBashMangled as u8);
        ((mask >> (self as u8)) & 1) != 0
    }

    /// Returns true if this format uses Unix-style separators (branchless).
    #[inline(always)]
    pub const fn uses_unix_separators(self) -> bool {
        let mask = (1 << Self::Wsl as u8) |
                  (1 << Self::Cygwin as u8) |
                  (1 << Self::UnixLike as u8);
        ((mask >> (self as u8)) & 1) != 0
    }

    /// Returns true if this format requires special handling for long paths (branchless).
    #[inline(always)]
    pub const fn requires_long_path_prefix(self) -> bool {
        let mask = (1 << Self::Dos as u8) |
                  (1 << Self::DosForward as u8) |
                  (1 << Self::Mixed as u8);
        ((mask >> (self as u8)) & 1) != 0
    }

    /// Returns the canonical separator for this format (branchless).
    #[inline(always)]
    pub const fn canonical_separator(self) -> char {
        // Use conditional move instead of branching
        let windows_sep = '\\' as u8;
        let unix_sep = '/' as u8;
        let uses_unix = self.uses_unix_separators() as u8;
        let sep = windows_sep * (1 - uses_unix) + unix_sep * uses_unix;
        sep as char
    }
}

/// SIMD-accelerated path format detection.
#[cfg(feature = "simd")]
#[inline(always)]
pub fn detect_path_format_simd(path: &str) -> PathFormat {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return PathFormat::Unknown;
    }

    // Use SIMD for prefix detection on paths >= 16 bytes
    if bytes.len() >= 16 {
        if let Some(format) = detect_prefix_simd(bytes) {
            return format;
        }
    }

    // Fall back to optimized scalar detection
    detect_path_format_optimized(path)
}

/// SIMD prefix detection for common path formats.
#[cfg(feature = "simd")]
#[inline(always)]
fn detect_prefix_simd(bytes: &[u8]) -> Option<PathFormat> {
    // Load first 16 bytes into SIMD register
    let chunk = u8x16::from_slice_unaligned(&bytes[..16]);

    // Check for UNC prefix "\\?\"
    if bytes.len() >= 4 {
        let unc_pattern = u8x16::splat(b'\\');
        let question = u8x16::splat(b'?');

        // Create masks for positions
        let mask1 = chunk.eq(unc_pattern);
        let mask2 = chunk.eq(question);

        // Check if pattern matches at start
        if mask1.extract(0) && mask1.extract(1) &&
           mask2.extract(2) && mask1.extract(3) {
            return Some(PathFormat::Unc);
        }
    }

    // Check for WSL prefix "/mnt/"
    if bytes.len() >= 5 {
        let slash = u8x16::splat(b'/');
        let m = u8x16::splat(b'm');
        let n = u8x16::splat(b'n');
        let t = u8x16::splat(b't');

        let mask_slash = chunk.eq(slash);
        let mask_m = chunk.eq(m);
        let mask_n = chunk.eq(n);
        let mask_t = chunk.eq(t);

        if mask_slash.extract(0) && mask_m.extract(1) &&
           mask_n.extract(2) && mask_t.extract(3) && mask_slash.extract(4) {
            return Some(PathFormat::Wsl);
        }
    }

    // Check for Unix-like "//"
    if bytes.len() >= 2 {
        let slash = u8x16::splat(b'/');
        let mask = chunk.eq(slash);
        if mask.extract(0) && mask.extract(1) {
            return Some(PathFormat::UnixLike);
        }
    }

    None
}

/// Optimized scalar path format detection with lookup tables.
#[inline(always)]
pub fn detect_path_format_optimized(path: &str) -> PathFormat {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return PathFormat::Unknown;
    }

    // Fast path: check for common prefixes using lookup
    if let Some(format) = detect_by_prefix_optimized(bytes) {
        return format;
    }

    // Check for mixed separators (branchless)
    let has_mixed = has_mixed_separators_branchless(bytes);
    if has_mixed {
        return PathFormat::Mixed;
    }

    // Check for DOS paths using lookup table
    if let Some(format) = detect_dos_format_optimized(bytes) {
        return format;
    }

    // Check for Unix-like patterns
    if let Some(format) = detect_unix_like_optimized(bytes) {
        return format;
    }

    // Determine if relative or unknown
    let has_sep = bytes.iter().any(|&b| SEPARATOR_TABLE[b as usize]);
    if has_sep {
        PathFormat::Relative
    } else {
        PathFormat::Unknown
    }
}

/// Optimized prefix detection using memchr and lookup tables.
#[inline(always)]
fn detect_by_prefix_optimized(bytes: &[u8]) -> Option<PathFormat> {
    // Use early return for most common cases
    if bytes.len() < 2 {
        return None;
    }

    // Check prefixes in order of frequency
    for &(prefix, format) in PREFIX_PATTERNS.iter() {
        if bytes.starts_with(prefix) {
            return Some(format);
        }
    }

    // Check for Git Bash mangled paths
    for prefix in GIT_BASH_PREFIXES {
        let prefix_bytes = prefix.as_bytes();
        if bytes.starts_with(prefix_bytes) {
            let after = &bytes[prefix_bytes.len()..];
            if after.starts_with(b"\\mnt\\") || after.starts_with(b"/mnt/") {
                return Some(PathFormat::GitBashMangled);
            }
        }
    }

    // Special case for Cygwin (longer prefix)
    if bytes.len() >= 10 && bytes.starts_with(b"/cygdrive/") {
        return Some(PathFormat::Cygwin);
    }

    None
}

/// Branchless mixed separator detection.
#[inline(always)]
fn has_mixed_separators_branchless(bytes: &[u8]) -> bool {
    let mut has_backslash = 0u8;
    let mut has_forward = 0u8;

    for &b in bytes {
        // Branchless accumulation
        has_backslash |= ((b == b'\\') as u8);
        has_forward |= ((b == b'/') as u8);
    }

    (has_backslash & has_forward) != 0
}

/// Optimized DOS format detection using lookup tables.
#[inline(always)]
fn detect_dos_format_optimized(bytes: &[u8]) -> Option<PathFormat> {
    if bytes.len() < 2 {
        return None;
    }

    // Use lookup table for drive letter check
    let is_drive = DRIVE_LETTER_TABLE[bytes[0] as usize];
    let has_colon = bytes[1] == b':';

    if is_drive && has_colon {
        if bytes.len() > 2 {
            // Branchless format selection
            let sep = bytes[2];
            let is_backslash = (sep == b'\\') as u8;
            let is_forward = (sep == b'/') as u8;

            if is_backslash != 0 {
                return Some(PathFormat::Dos);
            } else if is_forward != 0 {
                return Some(PathFormat::DosForward);
            }
        }
        return Some(PathFormat::Dos);
    }

    None
}

/// Optimized Unix-like format detection.
#[inline(always)]
fn detect_unix_like_optimized(bytes: &[u8]) -> Option<PathFormat> {
    if !bytes.starts_with(b"/") {
        return None;
    }

    // Already handled WSL and Cygwin, check for other Unix patterns
    if bytes.len() >= 3 {
        let start_idx = if bytes.starts_with(b"//") { 2 } else { 1 };

        if bytes.len() > start_idx + 1 {
            let potential_drive = bytes[start_idx];
            if DRIVE_LETTER_TABLE[potential_drive as usize] {
                let next = if start_idx + 1 < bytes.len() {
                    bytes[start_idx + 1]
                } else {
                    0
                };

                if next == b'/' || start_idx + 1 == bytes.len() {
                    return Some(PathFormat::UnixLike);
                }
            }
        }
    }

    Some(PathFormat::Relative)
}

/// SIMD-accelerated drive letter extraction.
#[cfg(feature = "simd")]
#[inline(always)]
pub fn extract_drive_letter_simd(path: &str, format: PathFormat) -> Option<char> {
    let bytes = path.as_bytes();

    match format {
        PathFormat::Dos | PathFormat::DosForward | PathFormat::Mixed => {
            if bytes.len() >= 2 && DRIVE_LETTER_TABLE[bytes[0] as usize] && bytes[1] == b':' {
                Some((bytes[0] as char).to_ascii_uppercase())
            } else {
                None
            }
        }
        PathFormat::Wsl => extract_from_prefix(bytes, b"/mnt/"),
        PathFormat::Cygwin => extract_from_prefix(bytes, b"/cygdrive/"),
        PathFormat::Unc => extract_from_unc(bytes),
        PathFormat::UnixLike => extract_from_unix_like(bytes),
        PathFormat::GitBashMangled => extract_from_git_bash(bytes),
        _ => None,
    }
}

#[inline(always)]
fn extract_from_prefix(bytes: &[u8], prefix: &[u8]) -> Option<char> {
    if bytes.starts_with(prefix) && bytes.len() > prefix.len() {
        let drive = bytes[prefix.len()];
        if DRIVE_LETTER_TABLE[drive as usize] {
            return Some((drive as char).to_ascii_uppercase());
        }
    }
    None
}

#[inline(always)]
fn extract_from_unc(bytes: &[u8]) -> Option<char> {
    if bytes.starts_with(b"\\\\?\\") && bytes.len() > 4 {
        let drive = bytes[4];
        if DRIVE_LETTER_TABLE[drive as usize] {
            return Some((drive as char).to_ascii_uppercase());
        }
    }
    None
}

#[inline(always)]
fn extract_from_unix_like(bytes: &[u8]) -> Option<char> {
    let start = bytes.iter().position(|&b| b != b'/')?;
    if bytes.len() > start && DRIVE_LETTER_TABLE[bytes[start] as usize] {
        Some((bytes[start] as char).to_ascii_uppercase())
    } else {
        None
    }
}

#[inline(always)]
fn extract_from_git_bash(bytes: &[u8]) -> Option<char> {
    for prefix in GIT_BASH_PREFIXES {
        let prefix_bytes = prefix.as_bytes();
        if bytes.starts_with(prefix_bytes) {
            let after = &bytes[prefix_bytes.len()..];
            let mnt_idx = if after.starts_with(b"\\mnt\\") {
                5
            } else if after.starts_with(b"/mnt/") {
                5
            } else {
                continue;
            };

            if after.len() > mnt_idx && DRIVE_LETTER_TABLE[after[mnt_idx] as usize] {
                return Some((after[mnt_idx] as char).to_ascii_uppercase());
            }
        }
    }
    None
}

/// Non-SIMD fallback
#[cfg(not(feature = "simd"))]
#[inline(always)]
pub fn detect_path_format_simd(path: &str) -> PathFormat {
    detect_path_format_optimized(path)
}

#[cfg(not(feature = "simd"))]
#[inline(always)]
pub fn extract_drive_letter_simd(path: &str, format: PathFormat) -> Option<char> {
    crate::detection::extract_drive_letter(path, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_format_detection() {
        assert_eq!(detect_path_format_optimized(r"C:\Users"), PathFormat::Dos);
        assert_eq!(detect_path_format_optimized("C:/Users"), PathFormat::DosForward);
        assert_eq!(detect_path_format_optimized("/mnt/c/users"), PathFormat::Wsl);
        assert_eq!(detect_path_format_optimized("/cygdrive/c/users"), PathFormat::Cygwin);
        assert_eq!(detect_path_format_optimized(r"\\?\C:\Users"), PathFormat::Unc);
        assert_eq!(detect_path_format_optimized("//c/users"), PathFormat::UnixLike);
        assert_eq!(detect_path_format_optimized(r"C:\Users/David"), PathFormat::Mixed);
    }

    #[test]
    fn test_branchless_operations() {
        assert!(PathFormat::Dos.is_absolute());
        assert!(!PathFormat::Relative.is_absolute());
        assert!(PathFormat::Wsl.uses_unix_separators());
        assert!(!PathFormat::Dos.uses_unix_separators());
        assert_eq!(PathFormat::Dos.canonical_separator(), '\\');
        assert_eq!(PathFormat::Wsl.canonical_separator(), '/');
    }

    #[test]
    fn test_mixed_separator_detection() {
        assert!(has_mixed_separators_branchless(r"C:\Users/David".as_bytes()));
        assert!(!has_mixed_separators_branchless(r"C:\Users\David".as_bytes()));
        assert!(!has_mixed_separators_branchless("C:/Users/David".as_bytes()));
    }

    #[cfg(feature = "simd")]
    #[test]
    fn test_simd_detection() {
        assert_eq!(detect_path_format_simd(r"\\?\C:\Users\David"), PathFormat::Unc);
        assert_eq!(detect_path_format_simd("/mnt/c/users"), PathFormat::Wsl);
        assert_eq!(detect_path_format_simd("//c/users"), PathFormat::UnixLike);
    }
}
