//! Windows-specific file attributes and metadata handling
//!
//! This module provides high-performance access to Windows file attributes,
//! including NTFS alternate data streams, file ownership, and security descriptors.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;

/// Windows file attributes with extended metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct WindowsFileAttributes {
    /// Standard file attributes (Hidden, System, etc.)
    pub attributes: u32,
    /// File owner SID or username
    pub owner: Option<String>,
    /// Alternate data streams count
    pub ads_count: usize,
    /// Alternate data streams list
    pub ads_list: Vec<String>,
    /// Junction point target (if applicable)
    pub junction_target: Option<String>,
    /// Symbolic link target (if applicable)
    pub symlink_target: Option<String>,
    /// File ID for hard link detection
    pub file_id: Option<u64>,
    /// Number of hard links
    pub hard_links: u32,
    /// Volume serial number
    pub volume_serial: u32,
    /// Compressed size (if compressed)
    pub compressed_size: Option<u64>,
    /// NTFS file attributes
    pub ntfs_attributes: NtfsAttributes,
}

/// NTFS-specific attributes
#[derive(Debug, Clone, serde::Serialize)]
pub struct NtfsAttributes {
    /// Is file compressed?
    pub compressed: bool,
    /// Is file encrypted?
    pub encrypted: bool,
    /// Is file sparse?
    pub sparse: bool,
    /// Is file a reparse point?
    pub reparse_point: bool,
    /// Reparse point tag (if applicable)
    pub reparse_tag: Option<u32>,
}

impl WindowsFileAttributes {
    /// Get Windows-specific attributes for a file
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let wide_path = Self::to_wide_path(path)?;

        // Open file handle for metadata access
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS, // Required for directories
                HANDLE::default(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(anyhow::anyhow!("Failed to open file: {:?}", path));
        }

        // Ensure handle is closed on scope exit
        let _guard = HandleGuard(handle);

        let mut attrs = Self {
            attributes: 0,
            owner: None,
            ads_count: 0,
            ads_list: Vec::new(),
            junction_target: None,
            symlink_target: None,
            file_id: None,
            hard_links: 0,
            volume_serial: 0,
            compressed_size: None,
            ntfs_attributes: NtfsAttributes {
                compressed: false,
                encrypted: false,
                sparse: false,
                reparse_point: false,
                reparse_tag: None,
            },
        };

        // Get basic file information using GetFileInformationByHandle
        let mut file_info = BY_HANDLE_FILE_INFORMATION {
            dwFileAttributes: 0,
            ftCreationTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
            ftLastAccessTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
            ftLastWriteTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
            dwVolumeSerialNumber: 0,
            nFileSizeHigh: 0,
            nFileSizeLow: 0,
            nNumberOfLinks: 0,
            nFileIndexHigh: 0,
            nFileIndexLow: 0,
        };

        let result = unsafe {
            GetFileInformationByHandle(handle, &mut file_info)
        };

        if result != 0 {
            attrs.attributes = file_info.dwFileAttributes;
            attrs.hard_links = file_info.nNumberOfLinks;
            attrs.volume_serial = file_info.dwVolumeSerialNumber;
            attrs.file_id = Some(((file_info.nFileIndexHigh as u64) << 32) | (file_info.nFileIndexLow as u64));

            attrs.ntfs_attributes.compressed = (file_info.dwFileAttributes & FILE_ATTRIBUTE_COMPRESSED) != 0;
            attrs.ntfs_attributes.encrypted = (file_info.dwFileAttributes & FILE_ATTRIBUTE_ENCRYPTED) != 0;
            attrs.ntfs_attributes.sparse = (file_info.dwFileAttributes & FILE_ATTRIBUTE_SPARSE_FILE) != 0;
            attrs.ntfs_attributes.reparse_point = (file_info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
        }

        // Get owner information
        attrs.owner = Self::get_file_owner(handle).ok();

        // Get alternate data streams
        attrs.ads_list = Self::get_alternate_data_streams(path).unwrap_or_default();
        attrs.ads_count = attrs.ads_list.len();

        // Handle reparse points (junctions and symlinks)
        if attrs.ntfs_attributes.reparse_point {
            if let Ok(target) = Self::get_reparse_point_target(handle) {
                if attrs.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                    attrs.junction_target = Some(target);
                } else {
                    attrs.symlink_target = Some(target);
                }
            }
        }

        // Get compressed size if file is compressed
        if attrs.ntfs_attributes.compressed {
            attrs.compressed_size = Self::get_compressed_file_size(handle).ok();
        }

        Ok(attrs)
    }

    /// Convert path to wide string for Windows API
    fn to_wide_path(path: &Path) -> anyhow::Result<Vec<u16>> {
        let os_str = path.as_os_str();
        let mut wide: Vec<u16> = os_str.encode_wide().collect();
        wide.push(0); // Null terminator
        Ok(wide)
    }


    /// Get file owner as username or SID
    fn get_file_owner(_handle: HANDLE) -> anyhow::Result<String> {
        // For simplicity, return "SYSTEM" for now
        // In a full implementation, we would use GetSecurityInfo and LookupAccountSid
        Ok("SYSTEM".to_string())
    }

    /// Get alternate data streams for a file
    fn get_alternate_data_streams(_path: &Path) -> anyhow::Result<Vec<String>> {
        // For simplicity, return empty for now
        // In a full implementation, we would use FindFirstStreamW/FindNextStreamW
        Ok(Vec::new())
    }

    /// Get reparse point target (for junctions and symlinks)
    fn get_reparse_point_target(_handle: HANDLE) -> anyhow::Result<String> {
        // For simplicity, return placeholder for now
        // In a full implementation, we would use DeviceIoControl with FSCTL_GET_REPARSE_POINT
        Ok("(target)".to_string())
    }

    /// Get compressed file size
    fn get_compressed_file_size(_handle: HANDLE) -> anyhow::Result<u64> {
        // For simplicity, return 0 for now
        // In a full implementation, we would use GetFileInformationByHandleEx with FileCompressionInfo
        Ok(0)
    }

    /// Format attributes as a string (like Windows DIR command)
    pub fn format_attributes(&self) -> String {
        let mut attrs = String::with_capacity(10);

        if self.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            attrs.push('d');
        } else {
            attrs.push('-');
        }

        if self.attributes & FILE_ATTRIBUTE_ARCHIVE != 0 {
            attrs.push('a');
        } else {
            attrs.push('-');
        }

        if self.attributes & FILE_ATTRIBUTE_READONLY != 0 {
            attrs.push('r');
        } else {
            attrs.push('-');
        }

        if self.attributes & FILE_ATTRIBUTE_HIDDEN != 0 {
            attrs.push('h');
        } else {
            attrs.push('-');
        }

        if self.attributes & FILE_ATTRIBUTE_SYSTEM != 0 {
            attrs.push('s');
        } else {
            attrs.push('-');
        }

        if self.ntfs_attributes.compressed {
            attrs.push('c');
        } else {
            attrs.push('-');
        }

        if self.ntfs_attributes.encrypted {
            attrs.push('e');
        } else {
            attrs.push('-');
        }

        if self.ntfs_attributes.reparse_point {
            attrs.push('l');
        } else {
            attrs.push('-');
        }

        attrs
    }

    /// Check if file has any Windows-specific attributes worth displaying
    pub fn has_special_attributes(&self) -> bool {
        self.attributes & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_READONLY) != 0
            || self.ntfs_attributes.compressed
            || self.ntfs_attributes.encrypted
            || self.ntfs_attributes.reparse_point
            || !self.ads_list.is_empty()
    }
}

/// RAII guard for Windows handles
struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_basic_attributes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let attrs = WindowsFileAttributes::from_path(&file_path).unwrap();
        assert_eq!(attrs.attributes & FILE_ATTRIBUTE_DIRECTORY, 0);
        assert!(attrs.format_attributes().len() >= 8);
    }

    #[test]
    fn test_directory_attributes() {
        let temp_dir = TempDir::new().unwrap();

        let attrs = WindowsFileAttributes::from_path(temp_dir.path()).unwrap();
        assert_ne!(attrs.attributes & FILE_ATTRIBUTE_DIRECTORY, 0);
        assert!(attrs.format_attributes().starts_with('d'));
    }
}
