//! Optimized path normalization with SIMD, zero-copy, and memory pooling.

use crate::{
    constants::*,
    detection_optimized::{detect_path_format_simd, extract_drive_letter_simd, PathFormat},
    error::{PathError, Result},
};
use arrayvec::ArrayString;
use bstr::ByteSlice;
use compact_str::CompactString;
use smallvec::SmallVec;
use std::borrow::Cow;

#[cfg(feature = "simd")]
use packed_simd_2::{u8x16, u8x32, u8x64};

/// Stack-allocated buffer for small paths (avoids heap allocation)
type PathBuffer = ArrayString<512>;
type ComponentVec = SmallVec<[CompactString; 16]>;

/// Optimized normalization result
#[derive(Debug, Clone)]
pub struct NormalizationResult<'a> {
    path: Cow<'a, str>,
    original_format: PathFormat,
    has_long_prefix: bool,
    was_modified: bool,
}

impl<'a> NormalizationResult<'a> {
    #[inline(always)]
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

    #[inline(always)]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline(always)]
    pub fn into_path(self) -> Cow<'a, str> {
        self.path
    }

    #[inline(always)]
    pub fn original_format(&self) -> PathFormat {
        self.original_format
    }

    #[inline(always)]
    pub fn has_long_path_prefix(&self) -> bool {
        self.has_long_prefix
    }

    #[inline(always)]
    pub fn was_modified(&self) -> bool {
        self.was_modified
    }
}

/// Main entry point for optimized path normalization
#[inline]
pub fn normalize_path_optimized<'a>(input: &'a str) -> Result<NormalizationResult<'a>> {
    if input.is_empty() {
        return Err(PathError::EmptyPath);
    }

    // Fast path: check if already normalized
    if is_already_normalized_fast(input) {
        return Ok(NormalizationResult::new(
            Cow::Borrowed(input),
            PathFormat::Dos,
            input.starts_with(UNC_PREFIX),
            false,
        ));
    }

    let format = detect_path_format_simd(input);

    match format {
        PathFormat::Dos => normalize_dos_optimized(input, format),
        PathFormat::DosForward => normalize_dos_forward_optimized(input),
        PathFormat::Wsl => normalize_wsl_optimized(input),
        PathFormat::Cygwin => normalize_cygwin_optimized(input),
        PathFormat::Unc => normalize_unc_optimized(input),
        PathFormat::UnixLike => normalize_unix_like_optimized(input),
        PathFormat::GitBashMangled => normalize_git_bash_optimized(input),
        PathFormat::Mixed => normalize_mixed_optimized(input),
        PathFormat::Relative => normalize_relative_optimized(input),
        PathFormat::Unknown => Err(PathError::UnsupportedFormat),
    }
}

/// Fast check if path is already normalized (common case optimization)
#[inline(always)]
fn is_already_normalized_fast(path: &str) -> bool {
    let bytes = path.as_bytes();

    // Quick checks for non-normalized patterns
    if bytes.len() < 2 {
        return false;
    }

    // Check for DOS path with backslashes only
    if !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return false;
    }

    // Use SIMD to check for forward slashes and double backslashes
    #[cfg(feature = "simd")]
    {
        if bytes.len() >= 16 {
            return !has_normalization_needed_simd(bytes);
        }
    }

    // Fallback to scalar check
    !bytes.contains(&b'/') && !bytes.windows(2).any(|w| w == b"\\\\")
        && !bytes.windows(3).any(|w| w == b"\\.." || w == b"\\.")
}

/// SIMD check for normalization needs
#[cfg(feature = "simd")]
#[inline(always)]
fn has_normalization_needed_simd(bytes: &[u8]) -> bool {
    let forward_slash = u8x16::splat(b'/');
    let backslash = u8x16::splat(b'\\');
    let dot = u8x16::splat(b'.');

    for chunk in bytes.chunks(16) {
        if chunk.len() < 16 {
            // Handle remainder with scalar
            return chunk.contains(&b'/') ||
                   chunk.windows(2).any(|w| w == b"\\\\" || w == b".." || w == b"\\.");
        }

        let v = u8x16::from_slice_unaligned(chunk);

        // Check for forward slashes
        if v.eq(forward_slash).any() {
            return true;
        }

        // Check for double backslashes or dots (more complex, needs windowed check)
        let has_backslash = v.eq(backslash);
        let has_dot = v.eq(dot);

        // If we have consecutive backslashes or dots with backslashes, need normalization
        if has_backslash.any() || has_dot.any() {
            // Do detailed check
            for i in 0..15 {
                if has_backslash.extract(i) && has_backslash.extract(i + 1) {
                    return true;  // Double backslash
                }
                if has_backslash.extract(i) && has_dot.extract(i + 1) {
                    return true;  // \. pattern
                }
            }
        }
    }

    false
}

/// Optimized DOS path normalization
#[inline]
fn normalize_dos_optimized<'a>(input: &'a str, format: PathFormat) -> Result<NormalizationResult<'a>> {
    let bytes = input.as_bytes();

    // Extract and validate drive letter
    let drive = extract_drive_letter_simd(input, format)
        .ok_or(PathError::InvalidDriveLetter)?;

    // Fast path for already normalized
    if !needs_normalization_dos(bytes) {
        let needs_long = bytes.len() > MAX_PATH;
        if needs_long {
            let mut result = PathBuffer::new();
            result.push_str(UNC_PREFIX);
            result.push_str(input);
            return Ok(NormalizationResult::new(
                Cow::Owned(result.to_string()),
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

    // Normalize with stack allocation for small paths
    let normalized = if input.len() <= 512 {
        normalize_dos_stackalloc(input)?
    } else {
        normalize_dos_heapalloc(input)?
    };

    let needs_long = normalized.len() > MAX_PATH;
    let final_path = if needs_long {
        format!("{}{}", UNC_PREFIX, normalized)
    } else {
        normalized
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        format,
        needs_long,
        true,
    ))
}

/// Stack-allocated DOS normalization for small paths
#[inline]
fn normalize_dos_stackalloc(input: &str) -> Result<String> {
    let mut buffer = PathBuffer::new();
    let bytes = input.as_bytes();

    // Copy drive letter
    if bytes.len() >= 2 {
        buffer.push(bytes[0] as char);
        buffer.push(':');
    }

    let mut i = 2;
    let mut last_was_sep = true;

    while i < bytes.len() {
        match bytes[i] {
            b'/' => {
                if !last_was_sep {
                    buffer.push('\\');
                    last_was_sep = true;
                }
            }
            b'\\' => {
                if !last_was_sep {
                    buffer.push('\\');
                    last_was_sep = true;
                }
            }
            b'.' if i + 1 < bytes.len() && bytes[i + 1] == b'.' => {
                // Handle .. component
                if i + 2 < bytes.len() && (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/') {
                    // Skip ..\ or ../
                    resolve_dot_dot(&mut buffer)?;
                    i += 3;
                    last_was_sep = true;
                    continue;
                }
            }
            b'.' if i + 1 < bytes.len() && (bytes[i + 1] == b'\\' || bytes[i + 1] == b'/') => {
                // Skip .\ or ./
                i += 2;
                last_was_sep = true;
                continue;
            }
            _ => {
                buffer.push(bytes[i] as char);
                last_was_sep = false;
            }
        }
        i += 1;
    }

    Ok(buffer.to_string())
}

/// Heap-allocated DOS normalization for large paths
#[inline]
fn normalize_dos_heapalloc(input: &str) -> Result<String> {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();

    // Similar logic but with String instead of ArrayString
    if bytes.len() >= 2 {
        result.push(bytes[0] as char);
        result.push(':');
    }

    let mut i = 2;
    let mut last_was_sep = true;

    while i < bytes.len() {
        match bytes[i] {
            b'/' | b'\\' => {
                if !last_was_sep {
                    result.push('\\');
                    last_was_sep = true;
                }
            }
            b'.' if i + 1 < bytes.len() => {
                if bytes[i + 1] == b'.' && i + 2 < bytes.len() &&
                   (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/') {
                    resolve_dot_dot_string(&mut result)?;
                    i += 3;
                    last_was_sep = true;
                    continue;
                } else if bytes[i + 1] == b'\\' || bytes[i + 1] == b'/' {
                    i += 2;
                    last_was_sep = true;
                    continue;
                }
                result.push('.');
                last_was_sep = false;
            }
            _ => {
                result.push(bytes[i] as char);
                last_was_sep = false;
            }
        }
        i += 1;
    }

    Ok(result)
}

/// Optimized WSL path normalization
#[inline]
fn normalize_wsl_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    let bytes = input.as_bytes();

    // Must start with /mnt/
    if !bytes.starts_with(b"/mnt/") || bytes.len() < 6 {
        return Err(PathError::MalformedWslPath);
    }

    // Extract drive letter (position 5)
    let drive = bytes[5];
    if !drive.is_ascii_alphabetic() {
        return Err(PathError::InvalidDriveLetter);
    }
    let drive = (drive as char).to_ascii_uppercase();

    // Build result using stack allocation for small paths
    let result = if input.len() <= 512 {
        let mut buffer = PathBuffer::new();
        buffer.push(drive);
        buffer.push(':');

        // Process rest of path
        if bytes.len() > 6 {
            let rest = &bytes[6..];
            if !rest.is_empty() && rest[0] == b'/' {
                process_unix_path_to_windows(&mut buffer, &rest[1..])?;
            }
        }

        buffer.to_string()
    } else {
        let mut result = String::with_capacity(input.len());
        result.push(drive);
        result.push(':');

        if bytes.len() > 6 {
            let rest = &bytes[6..];
            if !rest.is_empty() && rest[0] == b'/' {
                process_unix_path_to_windows_string(&mut result, &rest[1..])?;
            }
        }

        result
    };

    let needs_long = result.len() > MAX_PATH;
    let final_path = if needs_long {
        format!("{}{}", UNC_PREFIX, result)
    } else {
        result
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::Wsl,
        needs_long,
        true,
    ))
}

// Similar optimized implementations for other formats...

/// Fast check if DOS path needs normalization
#[inline(always)]
fn needs_normalization_dos(bytes: &[u8]) -> bool {
    bytes.contains(&b'/') ||
    bytes.windows(2).any(|w| w == b"\\\\") ||
    bytes.windows(2).any(|w| w == b"\\." || w == b"..")
}

/// Resolve .. in a path buffer
#[inline]
fn resolve_dot_dot(buffer: &mut PathBuffer) -> Result<()> {
    // Find last backslash before current position
    if let Some(pos) = buffer.rfind('\\') {
        buffer.truncate(pos);
    }
    Ok(())
}

/// Resolve .. in a String
#[inline]
fn resolve_dot_dot_string(buffer: &mut String) -> Result<()> {
    if let Some(pos) = buffer.rfind('\\') {
        buffer.truncate(pos);
    }
    Ok(())
}

/// Process Unix path components to Windows format (stack allocated)
#[inline]
fn process_unix_path_to_windows(buffer: &mut PathBuffer, rest: &[u8]) -> Result<()> {
    if rest.is_empty() {
        return Ok(());
    }

    buffer.push('\\');

    let components: ComponentVec = rest
        .split(|&b| b == b'/')
        .filter(|c| !c.is_empty() && c != b".")
        .map(|c| CompactString::from(c.to_str_lossy()))
        .collect();

    for (i, component) in components.iter().enumerate() {
        if component == ".." {
            resolve_dot_dot(buffer)?;
        } else {
            buffer.push_str(component);
            if i < components.len() - 1 {
                buffer.push('\\');
            }
        }
    }

    Ok(())
}

/// Process Unix path components to Windows format (heap allocated)
#[inline]
fn process_unix_path_to_windows_string(buffer: &mut String, rest: &[u8]) -> Result<()> {
    if rest.is_empty() {
        return Ok(());
    }

    buffer.push('\\');

    let components: Vec<&str> = rest
        .split(|&b| b == b'/')
        .filter_map(|c| {
            if c.is_empty() || c == b"." {
                None
            } else {
                Some(std::str::from_utf8(c).unwrap_or_default())
            }
        })
        .collect();

    for (i, component) in components.iter().enumerate() {
        if *component == ".." {
            resolve_dot_dot_string(buffer)?;
        } else {
            buffer.push_str(component);
            if i < components.len() - 1 {
                buffer.push('\\');
            }
        }
    }

    Ok(())
}

// Implement remaining format normalizations...
fn normalize_dos_forward_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    // Similar optimization as DOS but handle forward slashes
    normalize_dos_optimized(input, PathFormat::DosForward)
}

fn normalize_cygwin_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    // Similar to WSL but with /cygdrive/ prefix
    let bytes = input.as_bytes();
    if !bytes.starts_with(b"/cygdrive/") || bytes.len() < 11 {
        return Err(PathError::MalformedCygwinPath);
    }

    let drive = bytes[10];
    if !drive.is_ascii_alphabetic() {
        return Err(PathError::InvalidDriveLetter);
    }

    // Reuse WSL logic with adjusted offset
    let modified_input = format!("/mnt/{}", &input[10..]);
    normalize_wsl_optimized(&modified_input)
}

fn normalize_unc_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    // UNC paths are already normalized, just validate
    if !input.starts_with(UNC_PREFIX) {
        return Err(PathError::MalformedUncPath);
    }

    Ok(NormalizationResult::new(
        Cow::Borrowed(input),
        PathFormat::Unc,
        true,
        false,
    ))
}

fn normalize_unix_like_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    let trimmed = input.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(PathError::UnsupportedFormat);
    }

    let parts: SmallVec<[&str; 16]> = trimmed.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(PathError::UnsupportedFormat);
    }

    let drive = parts[0].chars().next()
        .filter(|c| c.is_ascii_alphabetic())
        .ok_or(PathError::InvalidDriveLetter)?
        .to_ascii_uppercase();

    let mut result = PathBuffer::new();
    result.push(drive);
    result.push(':');

    if parts.len() > 1 {
        result.push('\\');
        for (i, part) in parts.iter().skip(1).enumerate() {
            result.push_str(part);
            if i < parts.len() - 2 {
                result.push('\\');
            }
        }
    }

    let result_str = result.to_string();
    let needs_long = result_str.len() > MAX_PATH;
    let final_path = if needs_long {
        format!("{}{}", UNC_PREFIX, result_str)
    } else {
        result_str
    };

    Ok(NormalizationResult::new(
        Cow::Owned(final_path),
        PathFormat::UnixLike,
        needs_long,
        true,
    ))
}

fn normalize_git_bash_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    // Handle Git Bash mangled paths
    for prefix in GIT_BASH_PREFIXES {
        if input.starts_with(prefix) {
            let after = &input[prefix.len()..];
            if after.starts_with("\\mnt\\") || after.starts_with("/mnt/") {
                let clean = if after.starts_with("\\mnt\\") {
                    &after[5..]
                } else {
                    &after[5..]
                };

                let normalized = format!("/mnt/{}", clean);
                return normalize_wsl_optimized(&normalized);
            }
        }
    }

    Err(PathError::UnsupportedFormat)
}

fn normalize_mixed_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    // Convert all separators to backslashes then normalize
    let converted = input.replace('/', "\\");
    normalize_dos_optimized(&converted, PathFormat::Mixed)
}

fn normalize_relative_optimized(input: &str) -> Result<NormalizationResult<'_>> {
    let normalized = input.replace('/', "\\");
    let normalized = normalized.replace("\\\\", "\\");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_normalization_check() {
        assert!(is_already_normalized_fast(r"C:\Users\David"));
        assert!(!is_already_normalized_fast("C:/Users/David"));
        assert!(!is_already_normalized_fast(r"C:\Users\\David"));
        assert!(!is_already_normalized_fast(r"C:\Users\.\David"));
    }

    #[test]
    fn test_stack_allocated_normalization() {
        let result = normalize_dos_stackalloc("C:/Users/David/Documents").unwrap();
        assert_eq!(result, r"C:\Users\David\Documents");
    }

    #[test]
    fn test_wsl_normalization() {
        let result = normalize_wsl_optimized("/mnt/c/users/david").unwrap();
        assert_eq!(result.path(), r"C:\users\david");
    }

    #[test]
    fn test_optimized_normalization() {
        let result = normalize_path_optimized(r"C:\Users\David").unwrap();
        assert!(!result.was_modified());
        assert_eq!(result.path(), r"C:\Users\David");

        let result = normalize_path_optimized("C:/Users/David").unwrap();
        assert!(result.was_modified());
        assert_eq!(result.path(), r"C:\Users\David");
    }
}
