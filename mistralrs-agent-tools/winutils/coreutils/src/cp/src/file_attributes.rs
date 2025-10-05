//! Windows file attributes handling
//!
//! This module provides functionality for preserving Windows-specific file attributes,
//! including security descriptors, alternate data streams, and file flags.

use std::ffi::CString;
use std::path::Path;
use std::ptr;
use filetime::{FileTime, set_file_times};
use uucore::error::{UResult, UError, USimpleError};
use windows_sys::Win32::Foundation::{BOOL, TRUE, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileAttributesW, SetFileAttributesW, CreateFileW, GetFileTime, SetFileTime,
    FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, FILE_ATTRIBUTE_TEMPORARY,
    FILE_SHARE_READ, OPEN_EXISTING, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_FLAG_BACKUP_SEMANTICS,
};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Security::{
    GetFileSecurityW, SetFileSecurityW, GetSecurityDescriptorLength,
    OWNER_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
    DACL_SECURITY_INFORMATION, SACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR,
};

/// Windows file attributes container
#[derive(Debug, Clone)]
pub struct WindowsFileAttributes {
    pub file_attributes: u32,
    pub creation_time: Option<FileTime>,
    pub last_access_time: Option<FileTime>,
    pub last_write_time: Option<FileTime>,
    pub security_descriptor: Option<Vec<u8>>,
    pub alternate_streams: Vec<AlternateDataStream>,
}

/// Alternate data stream information
#[derive(Debug, Clone)]
pub struct AlternateDataStream {
    pub name: String,
    pub data: Vec<u8>,
}

impl WindowsFileAttributes {
    /// Read Windows attributes from a file
    pub fn from_path(path: &Path) -> UResult<Self> {
        let wide_path = to_wide_string(path)?;

        // Get basic file attributes
        let file_attributes = unsafe { GetFileAttributesW(wide_path.as_ptr()) };
        if file_attributes == u32::MAX {
            return Err(USimpleError::new(1, format!(
                "cannot get file attributes for '{}': {}",
                path.display(),
                std::io::Error::last_os_error()
            )));
        }

        // Get file times
        let (creation_time, last_access_time, last_write_time) = get_file_times(path)?;

        // Get security descriptor if needed
        let security_descriptor = get_security_descriptor(path).ok();

        // Get alternate data streams if needed
        let alternate_streams = get_alternate_streams(path).unwrap_or_default();

        Ok(WindowsFileAttributes {
            file_attributes,
            creation_time,
            last_access_time,
            last_write_time,
            security_descriptor,
            alternate_streams,
        })
    }

    /// Apply these attributes to a target file
    pub fn apply_to_path(&self, path: &Path) -> UResult<()> {
        let wide_path = to_wide_string(path)?;

        // Set basic file attributes
        unsafe {
            let result = SetFileAttributesW(wide_path.as_ptr(), self.file_attributes);
            if result == 0 {
                return Err(USimpleError::new(1, format!(
                    "cannot set file attributes for '{}': {}",
                    path.display(),
                    std::io::Error::last_os_error()
                )));
            }
        }

        // Set file times
        if let (Some(ct), Some(at), Some(wt)) = (
            &self.creation_time,
            &self.last_access_time,
            &self.last_write_time,
        ) {
            set_file_times(path, *at, *wt)
                .map_err(|e| USimpleError::new(1, format!("cannot set file times: {}", e)))?;

            // Set creation time separately (not supported by filetime crate)
            set_creation_time(path, ct)?;
        }

        // Set security descriptor
        if let Some(ref security_data) = self.security_descriptor {
            set_security_descriptor(path, security_data)?;
        }

        // Set alternate data streams
        for stream in &self.alternate_streams {
            set_alternate_stream(path, &stream.name, &stream.data)?;
        }

        Ok(())
    }

    /// Check if file has Windows-specific attributes that need preservation
    pub fn has_special_attributes(&self) -> bool {
        // Check for attributes beyond the basic ones
        let basic_attrs = FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_NORMAL;
        let special_attrs = self.file_attributes & !basic_attrs;

        special_attrs != 0
            || self.security_descriptor.is_some()
            || !self.alternate_streams.is_empty()
    }

    /// Check if file is hidden
    pub fn is_hidden(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_HIDDEN) != 0
    }

    /// Check if file is system file
    pub fn is_system(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_SYSTEM) != 0
    }

    /// Check if file is compressed
    pub fn is_compressed(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_COMPRESSED) != 0
    }

    /// Check if file is encrypted
    pub fn is_encrypted(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_ENCRYPTED) != 0
    }

    /// Check if file is read-only
    pub fn is_readonly(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_READONLY) != 0
    }
}

/// Get file times using Windows API
fn get_file_times(path: &Path) -> UResult<(Option<FileTime>, Option<FileTime>, Option<FileTime>)> {
    let wide_path = to_wide_string(path)?;

    unsafe {
        let handle = CreateFileW(
            wide_path.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS, // Needed for directories
            INVALID_HANDLE_VALUE,
        );

        if handle == INVALID_HANDLE_VALUE {
            return Ok((None, None, None));
        }

        let mut creation_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut last_access_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut last_write_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };

        let result = GetFileTime(
            handle,
            &mut creation_time,
            &mut last_access_time,
            &mut last_write_time,
        );

        windows_sys::Win32::Foundation::CloseHandle(handle);

        if result == 0 {
            return Ok((None, None, None));
        }

        let creation = filetime_from_windows(&creation_time);
        let access = filetime_from_windows(&last_access_time);
        let write = filetime_from_windows(&last_write_time);

        Ok((Some(creation), Some(access), Some(write)))
    }
}

/// Set creation time using Windows API
fn set_creation_time(path: &Path, creation_time: &FileTime) -> UResult<()> {
    let wide_path = to_wide_string(path)?;

    unsafe {
        let handle = CreateFileW(
            wide_path.as_ptr(),
            FILE_GENERIC_WRITE,
            FILE_SHARE_READ,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            INVALID_HANDLE_VALUE,
        );

        if handle == INVALID_HANDLE_VALUE {
            return Err(USimpleError::new(1, format!(
                "cannot open file for time modification: {}",
                std::io::Error::last_os_error()
            )));
        }

        let windows_time = filetime_to_windows(creation_time);

        let result = SetFileTime(handle, &windows_time, ptr::null(), ptr::null());

        windows_sys::Win32::Foundation::CloseHandle(handle);

        if result == 0 {
            return Err(USimpleError::new(1, format!(
                "cannot set creation time: {}",
                std::io::Error::last_os_error()
            )));
        }
    }

    Ok(())
}

/// Get security descriptor for a file
fn get_security_descriptor(path: &Path) -> UResult<Vec<u8>> {
    let wide_path = to_wide_string(path)?;

    let security_info = OWNER_SECURITY_INFORMATION
        | GROUP_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION
        | SACL_SECURITY_INFORMATION;

    unsafe {
        // First call to get required buffer size
        let mut buffer_size = 0u32;
        GetFileSecurityW(
            wide_path.as_ptr(),
            security_info,
            ptr::null_mut(),
            0,
            &mut buffer_size,
        );

        if buffer_size == 0 {
            return Err(USimpleError::new(1, "cannot determine security descriptor size".to_string()));
        }

        // Second call to get actual security descriptor
        let mut buffer = vec![0u8; buffer_size as usize];
        let result = GetFileSecurityW(
            wide_path.as_ptr(),
            security_info,
            buffer.as_mut_ptr() as PSECURITY_DESCRIPTOR,
            buffer_size,
            &mut buffer_size,
        );

        if result == 0 {
            return Err(USimpleError::new(1, format!(
                "cannot get security descriptor: {}",
                std::io::Error::last_os_error()
            )));
        }

        Ok(buffer)
    }
}

/// Set security descriptor for a file
fn set_security_descriptor(path: &Path, security_data: &[u8]) -> UResult<()> {
    let wide_path = to_wide_string(path)?;

    let security_info = OWNER_SECURITY_INFORMATION
        | GROUP_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION;

    unsafe {
        let result = SetFileSecurityW(
            wide_path.as_ptr(),
            security_info,
            security_data.as_ptr() as PSECURITY_DESCRIPTOR,
        );

        if result == 0 {
            return Err(USimpleError::new(1, format!(
                "cannot set security descriptor: {}",
                std::io::Error::last_os_error()
            )));
        }
    }

    Ok(())
}

/// Get alternate data streams for a file
fn get_alternate_streams(path: &Path) -> UResult<Vec<AlternateDataStream>> {
    // TODO: Implement ADS enumeration using FindFirstStreamW/FindNextStreamW
    // For now, return empty vector
    Ok(Vec::new())
}

/// Set alternate data stream
fn set_alternate_stream(path: &Path, stream_name: &str, data: &[u8]) -> UResult<()> {
    // TODO: Implement ADS writing
    // For now, this is a placeholder
    Ok(())
}

/// Convert Windows FILETIME to rust FileTime
fn filetime_from_windows(ft: &FILETIME) -> FileTime {
    let windows_ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    // Windows epoch is January 1, 1601; Unix epoch is January 1, 1970
    // 116444736000000000 is the number of 100-nanosecond intervals between the two epochs
    let unix_ticks = windows_ticks.saturating_sub(116444736000000000);
    let seconds = unix_ticks / 10_000_000;
    let nanos = ((unix_ticks % 10_000_000) * 100) as u32;

    FileTime::from_unix_time(seconds as i64, nanos)
}

/// Convert rust FileTime to Windows FILETIME
fn filetime_to_windows(ft: &FileTime) -> FILETIME {
    let unix_time = ft.unix_seconds() as u64;
    let nanos = ft.nanoseconds();
    let windows_ticks = (unix_time * 10_000_000) + (nanos as u64 / 100) + 116444736000000000;

    FILETIME {
        dwLowDateTime: (windows_ticks & 0xFFFFFFFF) as u32,
        dwHighDateTime: (windows_ticks >> 32) as u32,
    }
}

/// Convert path to wide string for Windows APIs
fn to_wide_string(path: &Path) -> UResult<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0); // Null terminator
    Ok(wide)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;

    #[test]
    fn test_windows_attributes_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let attrs = WindowsFileAttributes::from_path(&file_path).unwrap();
        assert!(attrs.file_attributes != 0);
        assert!(attrs.creation_time.is_some());
        assert!(attrs.last_access_time.is_some());
        assert!(attrs.last_write_time.is_some());
    }

    #[test]
    fn test_filetime_conversion() {
        let original = FileTime::now();
        let windows_time = filetime_to_windows(&original);
        let converted = filetime_from_windows(&windows_time);

        // Allow for small precision loss in conversion
        let diff = (original.unix_seconds() - converted.unix_seconds()).abs();
        assert!(diff <= 1);
    }

    #[test]
    fn test_attribute_flags() {
        let attrs = WindowsFileAttributes {
            file_attributes: FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM,
            creation_time: None,
            last_access_time: None,
            last_write_time: None,
            security_descriptor: None,
            alternate_streams: Vec::new(),
        };

        assert!(attrs.is_hidden());
        assert!(attrs.is_system());
        assert!(!attrs.is_readonly());
        assert!(!attrs.is_compressed());
        assert!(!attrs.is_encrypted());
    }

    #[test]
    fn test_special_attributes_detection() {
        let normal_attrs = WindowsFileAttributes {
            file_attributes: FILE_ATTRIBUTE_NORMAL,
            creation_time: None,
            last_access_time: None,
            last_write_time: None,
            security_descriptor: None,
            alternate_streams: Vec::new(),
        };
        assert!(!normal_attrs.has_special_attributes());

        let special_attrs = WindowsFileAttributes {
            file_attributes: FILE_ATTRIBUTE_HIDDEN,
            creation_time: None,
            last_access_time: None,
            last_write_time: None,
            security_descriptor: None,
            alternate_streams: Vec::new(),
        };
        assert!(special_attrs.has_special_attributes());
    }
}
