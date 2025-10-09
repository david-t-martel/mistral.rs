//! NTFS junction point and symbolic link handler
//!
//! This module provides functionality for detecting, reading, and creating
//! NTFS junction points and Windows symbolic links during copy operations.

use std::ffi::OsString;
use std::fs;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use uucore::error::{UResult, UError, USimpleError};
use windows_sys::Win32::Foundation::{BOOL, TRUE, FALSE, HANDLE, INVALID_HANDLE_VALUE, CloseHandle};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileAttributesW, CreateSymbolicLinkW, CreateHardLinkW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING,
    SYMBOLIC_LINK_FLAG_DIRECTORY, SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
};
use windows_sys::Win32::System::IO::{OVERLAPPED, DeviceIoControl};

// FSCTL_GET_REPARSE_POINT constant
const FSCTL_GET_REPARSE_POINT: u32 = 0x000900A8;

use crate::windows_cp::WindowsCpOptions;

/// NTFS reparse point tag constants
const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA0000003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000000C;

/// Maximum size for reparse point data
const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16 * 1024;

/// Junction and symbolic link handler
pub struct JunctionHandler {
    follow_junctions: bool,
    preserve_junctions: bool,
}

/// Reparse point data structure
#[repr(C)]
struct ReparseDataBuffer {
    reparse_tag: u32,
    reparse_data_length: u16,
    reserved: u16,
    data: [u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE],
}

/// Mount point (junction) reparse data
#[repr(C)]
struct MountPointReparseBuffer {
    substitute_name_offset: u16,
    substitute_name_length: u16,
    print_name_offset: u16,
    print_name_length: u16,
    path_buffer: [u16; 1], // Variable length
}

/// Symbolic link reparse data
#[repr(C)]
struct SymbolicLinkReparseBuffer {
    substitute_name_offset: u16,
    substitute_name_length: u16,
    print_name_offset: u16,
    print_name_length: u16,
    flags: u32,
    path_buffer: [u16; 1], // Variable length
}

/// Type of reparse point
#[derive(Debug, Clone, PartialEq)]
pub enum ReparsePointType {
    Junction,
    SymbolicLink,
    Other(u32),
}

/// Reparse point information
#[derive(Debug, Clone)]
pub struct ReparsePointInfo {
    pub point_type: ReparsePointType,
    pub target: PathBuf,
    pub is_directory: bool,
}

impl JunctionHandler {
    /// Create a new junction handler
    pub fn new(follow_junctions: bool, preserve_junctions: bool) -> Self {
        Self {
            follow_junctions,
            preserve_junctions,
        }
    }

    /// Check if a path is a junction point or symbolic link
    pub fn is_junction(&self, path: &Path) -> UResult<bool> {
        let wide_path = to_wide_string(path)?;

        unsafe {
            let attributes = GetFileAttributesW(wide_path.as_ptr());
            if attributes == u32::MAX {
                return Ok(false);
            }

            Ok((attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
        }
    }

    /// Get reparse point information
    pub fn get_reparse_info(&self, path: &Path) -> UResult<ReparsePointInfo> {
        let wide_path = to_wide_string(path)?;

        unsafe {
            // Open the reparse point without following it
            let handle = CreateFileW(
                wide_path.as_ptr(),
                FILE_GENERIC_READ,
                FILE_SHARE_READ,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                INVALID_HANDLE_VALUE,
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(uucore::error::USimpleError::new(
                    1,
                    format!("cannot open reparse point '{}': {}", path.display(), std::io::Error::last_os_error())
                ).into());
            }

            let mut reparse_buffer = ReparseDataBuffer {
                reparse_tag: 0,
                reparse_data_length: 0,
                reserved: 0,
                data: [0; MAXIMUM_REPARSE_DATA_BUFFER_SIZE],
            };

            let mut bytes_returned = 0u32;
            let result = DeviceIoControl(
                handle,
                FSCTL_GET_REPARSE_POINT,
                ptr::null_mut(),
                0,
                &mut reparse_buffer as *mut _ as *mut _,
                std::mem::size_of::<ReparseDataBuffer>() as u32,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            CloseHandle(handle);

            if result == 0 {
                return Err(std::io::Error::last_os_error().into());
            }

            self.parse_reparse_data(&reparse_buffer, path)
        }
    }

    /// Parse reparse point data
    fn parse_reparse_data(&self, buffer: &ReparseDataBuffer, path: &Path) -> UResult<ReparsePointInfo> {
        let point_type = match buffer.reparse_tag {
            IO_REPARSE_TAG_MOUNT_POINT => ReparsePointType::Junction,
            IO_REPARSE_TAG_SYMLINK => ReparsePointType::SymbolicLink,
            tag => ReparsePointType::Other(tag),
        };

        let target = match point_type {
            ReparsePointType::Junction => self.parse_junction_target(buffer)?,
            ReparsePointType::SymbolicLink => self.parse_symlink_target(buffer)?,
            ReparsePointType::Other(_) => {
                return Err(USimpleError::new(
                    1,
                    format!("unsupported reparse point type: 0x{:X}", buffer.reparse_tag)
                ));
            }
        };

        // Check if target is a directory
        let is_directory = target.is_dir();

        Ok(ReparsePointInfo {
            point_type,
            target,
            is_directory,
        })
    }

    /// Parse junction target path
    fn parse_junction_target(&self, buffer: &ReparseDataBuffer) -> UResult<PathBuf> {
        unsafe {
            let mount_point_data = &*(buffer.data.as_ptr() as *const MountPointReparseBuffer);

            let substitute_name_ptr = buffer.data.as_ptr().add(
                std::mem::size_of::<MountPointReparseBuffer>() - std::mem::size_of::<[u16; 1]>()
                + mount_point_data.substitute_name_offset as usize
            ) as *const u16;

            let substitute_name_len = mount_point_data.substitute_name_length as usize / 2;

            let substitute_name_slice = std::slice::from_raw_parts(substitute_name_ptr, substitute_name_len);
            let target_string = OsString::from_wide(substitute_name_slice);

            // Convert NT path to standard path
            let target_path = self.normalize_target_path(target_string.into())?;
            Ok(target_path)
        }
    }

    /// Parse symbolic link target path
    fn parse_symlink_target(&self, buffer: &ReparseDataBuffer) -> UResult<PathBuf> {
        unsafe {
            let symlink_data = &*(buffer.data.as_ptr() as *const SymbolicLinkReparseBuffer);

            let substitute_name_ptr = buffer.data.as_ptr().add(
                std::mem::size_of::<SymbolicLinkReparseBuffer>() - std::mem::size_of::<[u16; 1]>()
                + symlink_data.substitute_name_offset as usize
            ) as *const u16;

            let substitute_name_len = symlink_data.substitute_name_length as usize / 2;

            let substitute_name_slice = std::slice::from_raw_parts(substitute_name_ptr, substitute_name_len);
            let target_string = OsString::from_wide(substitute_name_slice);

            // Convert NT path to standard path
            let target_path = self.normalize_target_path(target_string.into())?;
            Ok(target_path)
        }
    }

    /// Normalize NT-style paths to standard Windows paths
    fn normalize_target_path(&self, path: PathBuf) -> UResult<PathBuf> {
        let path_str = path.to_string_lossy();

        // Handle NT device paths
        if path_str.starts_with("\\??\\") {
            let normalized = path_str.strip_prefix("\\??\\").unwrap_or(&path_str);
            return Ok(PathBuf::from(normalized));
        }

        if path_str.starts_with("\\Device\\") {
            // TODO @gemini: Convert device paths to drive letters
            // For now, return as-is
            return Ok(path);
        }

        Ok(path)
    }

    /// Copy a junction point or symbolic link
    pub fn copy_junction(
        &self,
        source: &Path,
        destination: &Path,
        options: &WindowsCpOptions,
    ) -> UResult<()> {
        if self.follow_junctions {
            // Follow the junction and copy the target content
            let reparse_info = self.get_reparse_info(source)?;
            return self.copy_junction_target(&reparse_info.target, destination, options);
        }

        if self.preserve_junctions {
            // Preserve the junction point itself
            return self.create_junction_copy(source, destination, options);
        }

        // Default behavior: skip junction points
        if options.verbose {
            println!("skipping junction point '{}'", source.display());
        }
        Ok(())
    }

    /// Copy the target of a junction point
    fn copy_junction_target(
        &self,
        target: &Path,
        destination: &Path,
        options: &WindowsCpOptions,
    ) -> UResult<()> {
        if target.is_dir() {
            if !options.recursive {
                return Err(USimpleError::new(1, format!(
                    "omitting directory '{}' (junction target)",
                    target.display()
                )));
            }
            // TODO @codex: Copy directory recursively
            // For now, delegate to standard copy
            std::fs::create_dir_all(destination)
                .map_err(|e| USimpleError::new(1, format!("cannot create directory: {}", e)))?;
        } else {
            // Copy regular file
            std::fs::copy(target, destination)
                .map_err(|e| USimpleError::new(1, format!("copy failed: {}", e)))?;
        }

        Ok(())
    }

    /// Create a copy of a junction point
    fn create_junction_copy(
        &self,
        source: &Path,
        destination: &Path,
        _options: &WindowsCpOptions,
    ) -> UResult<()> {
        let reparse_info = self.get_reparse_info(source)?;

        match reparse_info.point_type {
            ReparsePointType::Junction => {
                self.create_junction(destination, &reparse_info.target)?;
            }
            ReparsePointType::SymbolicLink => {
                self.create_symbolic_link(destination, &reparse_info.target, reparse_info.is_directory)?;
            }
            ReparsePointType::Other(tag) => {
                return Err(uucore::error::USimpleError::new(
                    1,
                    format!("cannot copy unsupported reparse point type: 0x{:X}", tag)
                ).into());
            }
        }

        Ok(())
    }

    /// Create a junction point
    fn create_junction(&self, junction_path: &Path, target_path: &Path) -> UResult<()> {
        // For junction points, we need to create a directory first, then set the reparse point
        // This is complex and requires direct FSCTL_SET_REPARSE_POINT calls
        // For now, we'll use a simpler approach and create a symbolic link instead

        if target_path.is_dir() {
            std::fs::create_dir_all(junction_path)
                .map_err(|e| USimpleError::new(1, format!("cannot create junction directory: {}", e)))?;
        }

        self.create_symbolic_link(junction_path, target_path, target_path.is_dir())
    }

    /// Create a symbolic link
    fn create_symbolic_link(&self, link_path: &Path, target_path: &Path, is_directory: bool) -> UResult<()> {
        let link_wide = to_wide_string(link_path)?;
        let target_wide = to_wide_string(target_path)?;

        let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
        if is_directory {
            flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
        }

        unsafe {
            let result = CreateSymbolicLinkW(
                link_wide.as_ptr(),
                target_wide.as_ptr(),
                flags,
            );

            if result == 0 {
                return Err(USimpleError::new(1, format!(
                    "cannot create symbolic link '{}' -> '{}': {}",
                    link_path.display(),
                    target_path.display(),
                    std::io::Error::last_os_error()
                )));
            }
        }

        Ok(())
    }

    /// Create a hard link
    pub fn create_hard_link(&self, link_path: &Path, target_path: &Path) -> UResult<()> {
        let link_wide = to_wide_string(link_path)?;
        let target_wide = to_wide_string(target_path)?;

        unsafe {
            let result = CreateHardLinkW(
                link_wide.as_ptr(),
                target_wide.as_ptr(),
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(USimpleError::new(1, format!(
                    "cannot create hard link '{}' -> '{}': {}",
                    link_path.display(),
                    target_path.display(),
                    std::io::Error::last_os_error()
                )));
            }
        }

        Ok(())
    }
}

/// Convert path to wide string for Windows APIs
fn to_wide_string(path: &Path) -> UResult<Vec<u16>> {
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
    fn test_junction_handler_creation() {
        let handler = JunctionHandler::new(true, false);
        assert!(handler.follow_junctions);
        assert!(!handler.preserve_junctions);
    }

    #[test]
    fn test_reparse_point_type_matching() {
        assert_eq!(
            match IO_REPARSE_TAG_MOUNT_POINT {
                IO_REPARSE_TAG_MOUNT_POINT => ReparsePointType::Junction,
                IO_REPARSE_TAG_SYMLINK => ReparsePointType::SymbolicLink,
                tag => ReparsePointType::Other(tag),
            },
            ReparsePointType::Junction
        );

        assert_eq!(
            match IO_REPARSE_TAG_SYMLINK {
                IO_REPARSE_TAG_MOUNT_POINT => ReparsePointType::Junction,
                IO_REPARSE_TAG_SYMLINK => ReparsePointType::SymbolicLink,
                tag => ReparsePointType::Other(tag),
            },
            ReparsePointType::SymbolicLink
        );
    }

    #[test]
    fn test_normalize_target_path() {
        let handler = JunctionHandler::new(false, false);

        let nt_path = PathBuf::from("\\??\\C:\\Test\\Path");
        let normalized = handler.normalize_target_path(nt_path).unwrap();
        assert_eq!(normalized, PathBuf::from("C:\\Test\\Path"));

        let normal_path = PathBuf::from("C:\\Test\\Path");
        let unchanged = handler.normalize_target_path(normal_path.clone()).unwrap();
        assert_eq!(unchanged, normal_path);
    }

    #[test]
    fn test_is_junction_with_regular_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("regular_file.txt");
        File::create(&file_path).unwrap();

        let handler = JunctionHandler::new(false, false);
        let is_junction = handler.is_junction(&file_path).unwrap();
        assert!(!is_junction);
    }
}
