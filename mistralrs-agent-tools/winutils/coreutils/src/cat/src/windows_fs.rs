//! Windows-specific file system operations and optimizations
//!
//! This module provides Windows-specific file handling optimizations including:
//! - FILE_FLAG_SEQUENTIAL_SCAN for better cache behavior
//! - Handle locked files with alternate access modes
//! - Optimized file opening strategies
//! - Windows error handling

use crate::{CatConfig, CatError, Result};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    },
    System::IO::DeviceIoControl,
};

/// Open a file with Windows-specific optimizations
pub fn open_file_optimized(path: &Path, config: &CatConfig) -> Result<File> {
    // Try different opening strategies in order of preference
    let strategies = [
        FileOpenStrategy::Optimized,
        FileOpenStrategy::Shared,
        FileOpenStrategy::ReadOnly,
        FileOpenStrategy::Basic,
    ];

    let mut last_error = None;

    for strategy in &strategies {
        match try_open_file(path, strategy, config) {
            Ok(file) => return Ok(file),
            Err(e) => {
                last_error = Some(e);
                // Continue to next strategy
            }
        }
    }

    // If all strategies failed, return the last error
    Err(last_error.unwrap_or_else(|| {
        CatError::Io(io::Error::new(
            io::ErrorKind::Other,
            "Failed to open file with any strategy",
        ))
    }))
}

#[derive(Debug, Clone, Copy)]
enum FileOpenStrategy {
    /// Optimized opening with FILE_FLAG_SEQUENTIAL_SCAN
    Optimized,
    /// Shared access mode for potentially locked files
    Shared,
    /// Read-only mode
    ReadOnly,
    /// Basic opening mode
    Basic,
}

fn try_open_file(path: &Path, strategy: &FileOpenStrategy, _config: &CatConfig) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);

    #[cfg(windows)]
    {
        match strategy {
            FileOpenStrategy::Optimized => {
                // Use FILE_FLAG_SEQUENTIAL_SCAN for better cache behavior
                options.custom_flags(FILE_FLAG_SEQUENTIAL_SCAN);
                options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
            }
            FileOpenStrategy::Shared => {
                // Allow sharing with other processes
                options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
            }
            FileOpenStrategy::ReadOnly => {
                // Minimal permissions
                options.share_mode(FILE_SHARE_READ);
            }
            FileOpenStrategy::Basic => {
                // No special flags
            }
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows systems, just use basic opening
        let _ = strategy; // Suppress unused variable warning
    }

    options.open(path).map_err(|e| {
        CatError::Io(enhance_windows_error(e, path))
    })
}

/// Check if a file appears to be locked by another process
pub fn is_file_locked(file: &File) -> Result<bool> {
    #[cfg(windows)]
    {
        // Try to get exclusive access temporarily to check if file is locked
        // This is a heuristic and may not be 100% accurate
        use std::os::windows::io::AsRawHandle;

        let handle = file.as_raw_handle() as HANDLE;
        if handle == INVALID_HANDLE_VALUE {
            return Ok(false);
        }

        // Try a non-destructive operation that would fail if file is locked
        // We use DeviceIoControl with a safe operation
        let mut bytes_returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                handle,
                0, // Use a safe, non-destructive control code
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        // If the operation fails with sharing violation, file is likely locked
        if result == 0 {
            let error = io::Error::last_os_error();
            Ok(error.raw_os_error() == Some(32)) // ERROR_SHARING_VIOLATION
        } else {
            Ok(false)
        }
    }

    #[cfg(not(windows))]
    {
        let _ = file; // Suppress unused variable warning
        Ok(false) // On non-Windows, assume file is not locked
    }
}

/// Enhance Windows-specific error messages
fn enhance_windows_error(error: io::Error, path: &Path) -> io::Error {
    #[cfg(windows)]
    {
        match error.raw_os_error() {
            Some(2) => {
                // ERROR_FILE_NOT_FOUND
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File not found: '{}'", path.display()),
                )
            }
            Some(3) => {
                // ERROR_PATH_NOT_FOUND
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Path not found: '{}'", path.display()),
                )
            }
            Some(5) => {
                // ERROR_ACCESS_DENIED
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Access denied: '{}' (check file permissions or if file is locked by another process)", path.display()),
                )
            }
            Some(32) => {
                // ERROR_SHARING_VIOLATION
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Sharing violation: '{}' is locked by another process", path.display()),
                )
            }
            Some(123) => {
                // ERROR_INVALID_NAME
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid filename: '{}'", path.display()),
                )
            }
            Some(267) => {
                // ERROR_DIRECTORY
                io::Error::new(
                    io::ErrorKind::IsADirectory,
                    format!("Path is a directory, not a file: '{}'", path.display()),
                )
            }
            _ => {
                // Keep original error but add path context
                io::Error::new(
                    error.kind(),
                    format!("{}: '{}'", error, path.display()),
                )
            }
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows, just add path context
        io::Error::new(
            error.kind(),
            format!("{}: '{}'", error, path.display()),
        )
    }
}

/// Get Windows-specific file information
#[allow(dead_code)]
pub struct WindowsFileInfo {
    pub is_sparse: bool,
    pub is_compressed: bool,
    pub is_encrypted: bool,
    pub is_reparse_point: bool,
}

#[allow(dead_code)]
pub fn get_windows_file_info(_path: &Path) -> Result<WindowsFileInfo> {
    #[cfg(windows)]
    {
        // This could be implemented using GetFileAttributes or similar
        // For now, return default values
        Ok(WindowsFileInfo {
            is_sparse: false,
            is_compressed: false,
            is_encrypted: false,
            is_reparse_point: false,
        })
    }

    #[cfg(not(windows))]
    {
        Ok(WindowsFileInfo {
            is_sparse: false,
            is_compressed: false,
            is_encrypted: false,
            is_reparse_point: false,
        })
    }
}

/// Check if the current process has the required privileges to read a file
pub fn check_read_permissions(path: &Path) -> Result<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            // If we can read metadata, we likely can read the file
            Ok(!metadata.permissions().readonly() || true) // readonly doesn't prevent reading
        }
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Ok(false),
            io::ErrorKind::PermissionDenied => Ok(false),
            _ => Err(CatError::Io(e)),
        }
    }
}

/// Attempt to resolve Windows junction points and symbolic links
pub fn resolve_path(path: &Path) -> Result<std::path::PathBuf> {
    #[cfg(windows)]
    {
        // Try to canonicalize the path to resolve junctions and symlinks
        match path.canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(_) => {
                // If canonicalization fails, return original path
                Ok(path.to_path_buf())
            }
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows, try standard canonicalization
        match path.canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(_) => Ok(path.to_path_buf()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config() -> CatConfig {
        CatConfig::default()
    }

    #[test]
    fn test_open_file_optimized() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();

        let config = create_test_config();
        let result = open_file_optimized(temp_file.path(), &config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_open_nonexistent_file() {
        let config = create_test_config();
        let result = open_file_optimized(Path::new("nonexistent_file.txt"), &config);

        assert!(result.is_err());
        if let Err(CatError::Io(io_err)) = result {
            assert_eq!(io_err.kind(), io::ErrorKind::NotFound);
        }
    }

    #[test]
    fn test_check_read_permissions() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();

        let result = check_read_permissions(temp_file.path());
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should have read permissions
    }

    #[test]
    fn test_resolve_path() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();

        let result = resolve_path(temp_file.path());
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert!(resolved.exists());
    }

    #[test]
    fn test_enhance_windows_error() {
        let original_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let path = Path::new("test_file.txt");

        let enhanced = enhance_windows_error(original_error, path);
        let error_message = enhanced.to_string();

        assert!(error_message.contains("test_file.txt"));
    }
}
