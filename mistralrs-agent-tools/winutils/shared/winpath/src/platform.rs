//! Platform-specific Windows path operations and optimizations.

#![cfg(windows)]

use crate::{constants::*, error::{PathError, Result}};
use alloc::{string::String, vec::Vec};
use core::mem::size_of;

/// Windows-specific path operations using native APIs.
pub struct WindowsPathOps;

impl WindowsPathOps {
    /// Gets the full path name using Windows API.
    ///
    /// This function uses GetFullPathNameW to resolve relative paths
    /// and normalize path separators using the Windows filesystem.
    pub fn get_full_path_name(path: &str) -> Result<String> {
        use windows_sys::Win32::Storage::FileSystem::GetFullPathNameW;
        use windows_sys::Win32::Foundation::GetLastError;

        // Convert to UTF-16
        let wide_path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();

        // First call to get required buffer size
        let required_size = unsafe {
            GetFullPathNameW(
                wide_path.as_ptr(),
                0,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };

        if required_size == 0 {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetFullPathNameW failed: {}", error)));
        }

        // Allocate buffer and get the full path
        let mut buffer: Vec<u16> = vec![0; required_size as usize];
        let result = unsafe {
            GetFullPathNameW(
                wide_path.as_ptr(),
                required_size,
                buffer.as_mut_ptr(),
                core::ptr::null_mut(),
            )
        };

        if result == 0 || result >= required_size {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetFullPathNameW failed: {}", error)));
        }

        // Convert back to UTF-8
        buffer.truncate(result as usize);
        String::from_utf16(&buffer)
            .map_err(|_| PathError::PlatformError("Invalid UTF-16 in path".to_string()))
    }

    /// Gets the long path name using Windows API.
    ///
    /// This function uses GetLongPathNameW to convert 8.3 format names
    /// to their full long equivalents.
    pub fn get_long_path_name(path: &str) -> Result<String> {
        use windows_sys::Win32::Storage::FileSystem::GetLongPathNameW;
        use windows_sys::Win32::Foundation::GetLastError;

        // Convert to UTF-16
        let wide_path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();

        // First call to get required buffer size
        let required_size = unsafe {
            GetLongPathNameW(
                wide_path.as_ptr(),
                core::ptr::null_mut(),
                0,
            )
        };

        if required_size == 0 {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetLongPathNameW failed: {}", error)));
        }

        // Allocate buffer and get the long path
        let mut buffer: Vec<u16> = vec![0; required_size as usize];
        let result = unsafe {
            GetLongPathNameW(
                wide_path.as_ptr(),
                buffer.as_mut_ptr(),
                required_size,
            )
        };

        if result == 0 || result >= required_size {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetLongPathNameW failed: {}", error)));
        }

        // Convert back to UTF-8
        buffer.truncate(result as usize);
        String::from_utf16(&buffer)
            .map_err(|_| PathError::PlatformError("Invalid UTF-16 in path".to_string()))
    }

    /// Gets the short path name using Windows API.
    ///
    /// This function uses GetShortPathNameW to convert long path names
    /// to their 8.3 format equivalents.
    pub fn get_short_path_name(path: &str) -> Result<String> {
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
        use windows_sys::Win32::Foundation::GetLastError;

        // Convert to UTF-16
        let wide_path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();

        // First call to get required buffer size
        let required_size = unsafe {
            GetShortPathNameW(
                wide_path.as_ptr(),
                core::ptr::null_mut(),
                0,
            )
        };

        if required_size == 0 {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetShortPathNameW failed: {}", error)));
        }

        // Allocate buffer and get the short path
        let mut buffer: Vec<u16> = vec![0; required_size as usize];
        let result = unsafe {
            GetShortPathNameW(
                wide_path.as_ptr(),
                buffer.as_mut_ptr(),
                required_size,
            )
        };

        if result == 0 || result >= required_size {
            let error = unsafe { GetLastError() };
            return Err(PathError::PlatformError(format!("GetShortPathNameW failed: {}", error)));
        }

        // Convert back to UTF-8
        buffer.truncate(result as usize);
        String::from_utf16(&buffer)
            .map_err(|_| PathError::PlatformError("Invalid UTF-16 in path".to_string()))
    }

    /// Checks if a path exists using Windows API.
    pub fn path_exists(path: &str) -> bool {
        use windows_sys::Win32::Storage::FileSystem::GetFileAttributesW;
        use windows_sys::Win32::Storage::FileSystem::INVALID_FILE_ATTRIBUTES;

        // Convert to UTF-16
        let wide_path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();

        let attributes = unsafe { GetFileAttributesW(wide_path.as_ptr()) };
        attributes != INVALID_FILE_ATTRIBUTES
    }

    /// Gets file attributes using Windows API.
    pub fn get_file_attributes(path: &str) -> Result<u32> {
        use windows_sys::Win32::Storage::FileSystem::GetFileAttributesW;
        use windows_sys::Win32::Storage::FileSystem::INVALID_FILE_ATTRIBUTES;
        use windows_sys::Win32::Foundation::GetLastError;

        // Convert to UTF-16
        let wide_path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();

        let attributes = unsafe { GetFileAttributesW(wide_path.as_ptr()) };

        if attributes == INVALID_FILE_ATTRIBUTES {
            let error = unsafe { GetLastError() };
            Err(PathError::PlatformError(format!("GetFileAttributesW failed: {}", error)))
        } else {
            Ok(attributes)
        }
    }

    /// Checks if a path is a directory using Windows API.
    pub fn is_directory(path: &str) -> Result<bool> {
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

        let attributes = Self::get_file_attributes(path)?;
        Ok((attributes & FILE_ATTRIBUTE_DIRECTORY) != 0)
    }

    /// Checks if a path is a file using Windows API.
    pub fn is_file(path: &str) -> Result<bool> {
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

        let attributes = Self::get_file_attributes(path)?;
        Ok((attributes & FILE_ATTRIBUTE_DIRECTORY) == 0)
    }

    /// Gets the case-sensitive path using Windows API.
    ///
    /// This function queries the filesystem for the actual case of path components,
    /// which is useful for normalizing paths to their canonical form.
    pub fn get_case_correct_path(path: &str) -> Result<String> {
        use windows_sys::Win32::Storage::FileSystem::{FindFirstFileW, FindClose, WIN32_FIND_DATAW};
        use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};

        let mut result = String::new();
        let mut remaining_path = path;

        // Handle drive letter first
        if remaining_path.len() >= 2 && remaining_path.chars().nth(1) == Some(':') {
            let drive = remaining_path.chars().nth(0).unwrap().to_ascii_uppercase();
            result.push(drive);
            result.push(':');

            if remaining_path.len() > 2 && remaining_path.chars().nth(2) == Some('\\') {
                result.push('\\');
                remaining_path = &remaining_path[3..];
            } else {
                remaining_path = &remaining_path[2..];
            }
        }

        // Process each path component
        while !remaining_path.is_empty() {
            let next_separator = remaining_path.find('\\').unwrap_or(remaining_path.len());
            let component = &remaining_path[..next_separator];

            if component.is_empty() {
                break;
            }

            // Build search pattern
            let search_path = if result.is_empty() {
                component.to_string()
            } else {
                format!("{}\\{}", result, component)
            };

            // Convert to UTF-16 for Windows API
            let wide_search: Vec<u16> = search_path.encode_utf16().chain(core::iter::once(0)).collect();

            // Find the file to get correct case
            let mut find_data: WIN32_FIND_DATAW = unsafe { core::mem::zeroed() };
            let handle = unsafe { FindFirstFileW(wide_search.as_ptr(), &mut find_data) };

            if handle == INVALID_HANDLE_VALUE {
                let error = unsafe { GetLastError() };
                return Err(PathError::PlatformError(format!("FindFirstFileW failed: {}", error)));
            }

            unsafe { FindClose(handle) };

            // Extract the correct filename from find_data
            let filename_end = find_data.cFileName.iter().position(|&c| c == 0).unwrap_or(find_data.cFileName.len());
            let correct_name = String::from_utf16(&find_data.cFileName[..filename_end])
                .map_err(|_| PathError::PlatformError("Invalid UTF-16 in filename".to_string()))?;

            // Append to result
            if !result.is_empty() {
                result.push('\\');
            }
            result.push_str(&correct_name);

            // Move to next component
            if next_separator < remaining_path.len() {
                remaining_path = &remaining_path[next_separator + 1..];
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Checks if long path support is enabled on the system.
    pub fn is_long_path_support_enabled() -> bool {
        use windows_sys::Win32::System::Registry::{
            RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
            HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
        };
        use windows_sys::Win32::Foundation::ERROR_SUCCESS;

        let key_path = "SYSTEM\\CurrentControlSet\\Control\\FileSystem\0"
            .encode_utf16()
            .collect::<Vec<_>>();

        let value_name = "LongPathsEnabled\0"
            .encode_utf16()
            .collect::<Vec<_>>();

        let mut hkey = core::ptr::null_mut();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                key_path.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };

        if result != ERROR_SUCCESS {
            return false;
        }

        let mut value: u32 = 0;
        let mut value_size = size_of::<u32>() as u32;
        let mut value_type = 0;

        let query_result = unsafe {
            RegQueryValueExW(
                hkey,
                value_name.as_ptr(),
                core::ptr::null_mut(),
                &mut value_type,
                &mut value as *mut u32 as *mut u8,
                &mut value_size,
            )
        };

        unsafe { RegCloseKey(hkey) };

        query_result == ERROR_SUCCESS && value_type == REG_DWORD && value == 1
    }

    /// Gets the maximum path length supported by the system.
    pub fn get_max_path_length() -> usize {
        if Self::is_long_path_support_enabled() {
            32767 // Extended path length limit
        } else {
            MAX_PATH // Traditional MAX_PATH limit
        }
    }
}

/// Windows-specific path utilities.
pub struct WindowsPathUtils;

impl WindowsPathUtils {
    /// Converts a path to use the optimal format for the current system.
    ///
    /// This function determines whether to use long path format based on
    /// the path length and system capabilities.
    pub fn optimize_path_format(path: &str) -> Result<String> {
        let max_length = WindowsPathOps::get_max_path_length();

        if path.len() > MAX_PATH && path.len() <= max_length {
            // Use long path format
            if !path.starts_with(UNC_PREFIX) {
                Ok(format!("{}{}", UNC_PREFIX, path))
            } else {
                Ok(path.to_string())
            }
        } else if path.len() > max_length {
            Err(PathError::PathTooLong(path.len()))
        } else {
            // Standard path format is fine
            Ok(path.to_string())
        }
    }

    /// Validates that a path is compatible with Windows filesystem.
    pub fn validate_windows_path(path: &str) -> Result<()> {
        // Check overall length
        let max_length = WindowsPathOps::get_max_path_length();
        if path.len() > max_length {
            return Err(PathError::PathTooLong(path.len()));
        }

        // Check individual component lengths
        for component in path.split(['\\', '/']) {
            if component.len() > 255 {
                return Err(PathError::InvalidComponent(
                    format!("Component too long: {}", component)
                ));
            }
        }

        // Additional Windows-specific validations would go here
        Ok(())
    }

    /// Converts WSL path to Windows path using the Windows Subsystem for Linux.
    ///
    /// This function can use the `wslpath` utility if available for more
    /// accurate conversions.
    pub fn convert_wsl_path_native(wsl_path: &str) -> Result<String> {
        // This is a simplified version. In practice, you might want to
        // use the wslpath utility or WSL APIs for more accurate conversion.

        if !wsl_path.starts_with("/mnt/") {
            return Err(PathError::MalformedWslPath);
        }

        let path_without_mnt = &wsl_path[5..]; // Remove "/mnt/"
        if path_without_mnt.is_empty() {
            return Err(PathError::MalformedWslPath);
        }

        let parts: Vec<&str> = path_without_mnt.split('/').collect();
        if parts.is_empty() || parts[0].len() != 1 {
            return Err(PathError::MalformedWslPath);
        }

        let drive_letter = parts[0].chars().next().unwrap().to_ascii_uppercase();
        if !drive_letter.is_ascii_alphabetic() {
            return Err(PathError::InvalidDriveLetter(drive_letter));
        }

        let mut windows_path = format!("{}:", drive_letter);
        if parts.len() > 1 {
            windows_path.push('\\');
            windows_path.push_str(&parts[1..].join("\\"));
        }

        Ok(windows_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_exists() {
        // Test with known system path
        assert!(WindowsPathOps::path_exists("C:\\"));
        assert!(!WindowsPathOps::path_exists("Z:\\NonexistentPath"));
    }

    #[test]
    fn test_get_max_path_length() {
        let max_length = WindowsPathOps::get_max_path_length();
        assert!(max_length >= MAX_PATH);
    }

    #[test]
    fn test_optimize_path_format() {
        // Short path should remain unchanged
        let short_path = "C:\\Users";
        let result = WindowsPathUtils::optimize_path_format(short_path).unwrap();
        assert_eq!(result, short_path);

        // Long path should get UNC prefix if needed
        let long_path = "C:\\".to_string() + &"a".repeat(300);
        let result = WindowsPathUtils::optimize_path_format(&long_path).unwrap();
        if long_path.len() > MAX_PATH {
            assert!(result.starts_with(UNC_PREFIX));
        }
    }

    #[test]
    fn test_validate_windows_path() {
        assert!(WindowsPathUtils::validate_windows_path("C:\\Users\\David").is_ok());

        // Test component too long
        let long_component = "a".repeat(300);
        let long_path = format!("C:\\{}", long_component);
        assert!(WindowsPathUtils::validate_windows_path(&long_path).is_err());
    }

    #[test]
    fn test_convert_wsl_path_native() {
        let result = WindowsPathUtils::convert_wsl_path_native("/mnt/c/users/david").unwrap();
        assert_eq!(result, "C:\\users\\david");

        let result = WindowsPathUtils::convert_wsl_path_native("/mnt/d").unwrap();
        assert_eq!(result, "D:");

        assert!(WindowsPathUtils::convert_wsl_path_native("/mnt/").is_err());
        assert!(WindowsPathUtils::convert_wsl_path_native("/invalid/path").is_err());
    }

    #[test]
    fn test_is_long_path_support_enabled() {
        // This test just ensures the function runs without crashing
        let _is_enabled = WindowsPathOps::is_long_path_support_enabled();
    }

    // Note: Tests for functions that require actual file system operations
    // (like get_full_path_name, get_case_correct_path) would need existing
    // files to test against, so they're omitted here.
}
