//! Windows-specific functionality for find wrapper

#[cfg(windows)]
use anyhow::{anyhow, Result};
#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::*;
#[cfg(windows)]
use windows_sys::Win32::System::IO::*;
#[cfg(windows)]
use crate::output::WindowsAttributes;

/// Get Windows file attributes for a path
#[cfg(windows)]
pub fn get_windows_attributes(path: &Path) -> Result<WindowsAttributes> {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let attributes = unsafe { GetFileAttributesW(wide_path.as_ptr()) };

    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(anyhow!("Failed to get file attributes for {:?}", path));
    }

    Ok(WindowsAttributes {
        hidden: (attributes & FILE_ATTRIBUTE_HIDDEN) != 0,
        system: (attributes & FILE_ATTRIBUTE_SYSTEM) != 0,
        readonly: (attributes & FILE_ATTRIBUTE_READONLY) != 0,
        archive: (attributes & FILE_ATTRIBUTE_ARCHIVE) != 0,
        directory: (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0,
        compressed: (attributes & FILE_ATTRIBUTE_COMPRESSED) != 0,
        encrypted: (attributes & FILE_ATTRIBUTE_ENCRYPTED) != 0,
        reparse_point: (attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0,
    })
}

/// Check if a file is hidden on Windows
#[cfg(windows)]
pub fn is_windows_hidden(path: &Path) -> bool {
    get_windows_attributes(path)
        .map(|attrs| attrs.hidden)
        .unwrap_or(false)
}

/// Check if a path is a junction point
#[cfg(windows)]
pub fn is_junction_point(path: &Path) -> bool {
    get_windows_attributes(path)
        .map(|attrs| attrs.reparse_point && attrs.directory)
        .unwrap_or(false)
}

/// Get NTFS alternate data streams for a file
#[cfg(windows)]
pub fn get_ntfs_streams(path: &Path) -> Result<Vec<String>> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut streams = Vec::new();
    let mut stream_context = std::ptr::null_mut();

    unsafe {
        let mut stream_id = WIN32_FIND_STREAM_DATA {
            StreamSize: Default::default(),
            cStreamName: [0; 296], // MAX_PATH + :$DATA
        };

        let handle = FindFirstStreamW(
            wide_path.as_ptr(),
            FindStreamInfoStandard,
            &mut stream_id as *mut _ as *mut _,
            0,
        );

        if handle == INVALID_HANDLE_VALUE {
            // No streams or error - return empty list
            return Ok(streams);
        }

        loop {
            // Convert stream name from wide string
            let stream_name_len = stream_id.cStreamName
                .iter()
                .position(|&x| x == 0)
                .unwrap_or(stream_id.cStreamName.len());

            if stream_name_len > 0 {
                let stream_name = OsString::from_wide(&stream_id.cStreamName[..stream_name_len]);
                let stream_str = stream_name.to_string_lossy().to_string();

                // Skip the default data stream
                if !stream_str.ends_with(":$DATA") || stream_str != "::$DATA" {
                    streams.push(stream_str);
                }
            }

            if FindNextStreamW(handle, &mut stream_id as *mut _ as *mut _) == 0 {
                break;
            }
        }

        FindClose(handle);
    }

    Ok(streams)
}

/// Check if a path points to a symbolic link
#[cfg(windows)]
pub fn is_symbolic_link(path: &Path) -> bool {
    if let Ok(attrs) = get_windows_attributes(path) {
        attrs.reparse_point && !attrs.directory
    } else {
        false
    }
}

/// Get the target of a symbolic link or junction
#[cfg(windows)]
pub fn get_reparse_target(path: &Path) -> Result<String> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::ffi::OsStringExt;
    use std::ffi::OsString;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateFileW(
            wide_path.as_ptr(),
            0, // No access needed for reading reparse data
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        );

        if handle == INVALID_HANDLE_VALUE {
            return Err(anyhow!("Failed to open reparse point"));
        }

        let mut buffer = vec![0u8; 16384]; // MAXIMUM_REPARSE_DATA_BUFFER_SIZE
        let mut bytes_returned = 0;

        let result = DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        CloseHandle(handle);

        if result == 0 {
            return Err(anyhow!("Failed to read reparse point data"));
        }

        // Parse reparse data
        if bytes_returned < 8 {
            return Err(anyhow!("Invalid reparse data"));
        }

        // Read the reparse tag
        let reparse_tag = u32::from_le_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3]
        ]);

        match reparse_tag {
            IO_REPARSE_TAG_SYMLINK => {
                // Symbolic link
                parse_symlink_reparse_data(&buffer[8..bytes_returned as usize])
            }
            IO_REPARSE_TAG_MOUNT_POINT => {
                // Junction/mount point
                parse_mount_point_reparse_data(&buffer[8..bytes_returned as usize])
            }
            _ => Err(anyhow!("Unsupported reparse point type: 0x{:08X}", reparse_tag)),
        }
    }
}

/// Parse symbolic link reparse data
#[cfg(windows)]
fn parse_symlink_reparse_data(data: &[u8]) -> Result<String> {
    if data.len() < 12 {
        return Err(anyhow!("Invalid symlink reparse data"));
    }

    let substitute_name_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_name_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_name_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_name_length = u16::from_le_bytes([data[6], data[7]]) as usize;

    // Use print name if available, otherwise substitute name
    let (offset, length) = if print_name_length > 0 {
        (print_name_offset, print_name_length)
    } else {
        (substitute_name_offset, substitute_name_length)
    };

    if data.len() < 12 + offset + length {
        return Err(anyhow!("Invalid symlink reparse data"));
    }

    let path_data = &data[12 + offset..12 + offset + length];
    let wide_chars: Vec<u16> = path_data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let target = OsString::from_wide(&wide_chars);
    Ok(target.to_string_lossy().to_string())
}

/// Parse mount point reparse data
#[cfg(windows)]
fn parse_mount_point_reparse_data(data: &[u8]) -> Result<String> {
    if data.len() < 8 {
        return Err(anyhow!("Invalid mount point reparse data"));
    }

    let substitute_name_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_name_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_name_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_name_length = u16::from_le_bytes([data[6], data[7]]) as usize;

    // Use print name if available, otherwise substitute name
    let (offset, length) = if print_name_length > 0 {
        (print_name_offset, print_name_length)
    } else {
        (substitute_name_offset, substitute_name_length)
    };

    if data.len() < 8 + offset + length {
        return Err(anyhow!("Invalid mount point reparse data"));
    }

    let path_data = &data[8 + offset..8 + offset + length];
    let wide_chars: Vec<u16> = path_data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let target = OsString::from_wide(&wide_chars);
    Ok(target.to_string_lossy().to_string())
}

// Non-Windows stub implementations
#[cfg(not(windows))]
pub fn get_windows_attributes(_path: &Path) -> Result<crate::output::WindowsAttributes> {
    use anyhow::anyhow;
    Err(anyhow!("Windows attributes not available on this platform"))
}

#[cfg(not(windows))]
pub fn is_windows_hidden(_path: &Path) -> bool {
    false
}

#[cfg(not(windows))]
pub fn is_junction_point(_path: &Path) -> bool {
    false
}

#[cfg(not(windows))]
pub fn get_ntfs_streams(_path: &Path) -> Result<Vec<String>> {
    Ok(Vec::new())
}

#[cfg(not(windows))]
pub fn is_symbolic_link(path: &Path) -> bool {
    path.is_symlink()
}

#[cfg(not(windows))]
pub fn get_reparse_target(_path: &Path) -> Result<String> {
    use anyhow::anyhow;
    Err(anyhow!("Reparse points not available on this platform"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[cfg(windows)]
    fn test_windows_attributes() {
        // Test with Windows directory
        let windows_dir = PathBuf::from("C:\\Windows");
        if windows_dir.exists() {
            let attrs = get_windows_attributes(&windows_dir).unwrap();
            assert!(attrs.directory);
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn test_non_windows_stubs() {
        let path = PathBuf::from("/tmp");
        assert!(!is_windows_hidden(&path));
        assert!(!is_junction_point(&path));
        assert!(get_ntfs_streams(&path).unwrap().is_empty());
    }
}
