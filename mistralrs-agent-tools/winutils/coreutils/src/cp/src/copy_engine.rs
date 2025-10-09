//! Windows copy engine with API optimizations
//!
//! This module provides the core copying functionality using Windows APIs
//! for optimal performance, including CopyFileEx, unbuffered I/O, and
//! cross-drive optimizations.

use std::ffi::CString;
use std::fs::{File, Metadata};
use std::io::{self, Read, Write, BufRead, BufReader};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use memmap2::Mmap;
use uucore::error::{UResult, UError, USimpleError};
use windows_sys::Win32::Foundation::{BOOL, TRUE, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CopyFileExW, CreateFileW, GetDriveTypeW, GetFileAttributesW, SetFileAttributesW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM,
    FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_COMPRESSED,
    FILE_ATTRIBUTE_ENCRYPTED, FILE_FLAG_NO_BUFFERING, FILE_FLAG_SEQUENTIAL_SCAN,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING, CREATE_ALWAYS,
    ReadFile, WriteFile,
};
use windows_sys::Win32::System::IO::{OVERLAPPED, GetOverlappedResult};
use windows_sys::Win32::NetworkManagement::WNet::WNetGetConnectionW;

use crate::file_attributes::WindowsFileAttributes;
use crate::progress::ProgressReporter;
use crate::windows_cp::WindowsCpOptions;

/// Windows-optimized copy engine
pub struct CopyEngine {
    large_file_threshold: u64,
    buffer_size: usize,
    use_unbuffered_io: bool,
    preserve_attributes: bool,
    preserve_security: bool,
    preserve_streams: bool,
    force_overwrite: bool,
}

impl CopyEngine {
    /// Create a new copy engine with the specified options
    pub fn new(options: &WindowsCpOptions) -> UResult<Self> {
        // Determine optimal buffer size based on system and files
        let buffer_size = if options.unbuffered {
            64 * 1024 * 1024 // 64MB for unbuffered I/O (must be sector-aligned)
        } else {
            4 * 1024 * 1024 // 4MB for regular I/O
        };

        Ok(CopyEngine {
            large_file_threshold: 100 * 1024 * 1024, // 100MB
            buffer_size,
            use_unbuffered_io: options.unbuffered,
            preserve_attributes: options.preserve_all || options.preserve_permissions,
            preserve_security: options.preserve_security,
            preserve_streams: options.preserve_streams,
            force_overwrite: options.force,
        })
    }

    /// Copy a single file using the most appropriate method
    pub fn copy_file(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        let source_metadata = source.metadata()
            .map_err(|e| USimpleError::new(1, format!("cannot stat '{}': {}", source.display(), e)))?;

        let file_size = source_metadata.len();

        // Choose copy method based on file size and drive configuration
        if self.should_use_copyfile_ex(source, destination, file_size)? {
            self.copy_with_copyfile_ex(source, destination, progress_reporter)
        } else if file_size > self.large_file_threshold {
            self.copy_large_file(source, destination, progress_reporter)
        } else {
            self.copy_standard_file(source, destination, progress_reporter)
        }
    }

    /// Determine if we should use CopyFileEx
    fn should_use_copyfile_ex(&self, source: &Path, destination: &Path, file_size: u64) -> UResult<bool> {
        // Use CopyFileEx for:
        // 1. Cross-drive copies
        // 2. Network shares
        // 3. When preserving streams or security
        // 4. Large files that benefit from OS-level optimization

        let src_drive_type = get_drive_type(source)?;
        let dest_drive_type = get_drive_type(destination)?;

        let is_cross_drive = get_drive_letter(source) != get_drive_letter(destination);
        let has_network = src_drive_type == DriveType::Remote || dest_drive_type == DriveType::Remote;
        let is_large_file = file_size > self.large_file_threshold;

        Ok(is_cross_drive || has_network || self.preserve_streams || self.preserve_security || is_large_file)
    }

    /// Copy file using Windows CopyFileEx API
    fn copy_with_copyfile_ex(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        let source_wide = to_wide_string(source)?;
        let dest_wide = to_wide_string(destination)?;

        let mut flags = 0u32;

        if !self.force_overwrite {
            flags |= 0x00000001; // COPY_FILE_FAIL_IF_EXISTS
        }

        if self.preserve_streams {
            flags |= 0x00000400; // COPY_FILE_ALLOW_DECRYPTED_DESTINATION
        }

        // Set up progress callback if needed
        let progress_data = if progress_reporter.should_show_progress() {
            Some(Arc::new(ProgressData::new(source, destination)))
        } else {
            None
        };

        let progress_callback = if progress_data.is_some() {
            Some(copy_progress_callback as unsafe extern "system" fn(i64, i64, i64, i64, u32, u32, isize, isize, *const std::ffi::c_void) -> u32)
        } else {
            None
        };

        let progress_param = progress_data
            .as_ref()
            .map(|data| Arc::as_ptr(data) as *const _)
            .unwrap_or(ptr::null());

        unsafe {
            let result = CopyFileExW(
                source_wide.as_ptr(),
                dest_wide.as_ptr(),
                progress_callback,
                progress_param as *const _,
                ptr::null_mut(),
                flags,
            );

            if result == 0 {
                let error = io::Error::last_os_error();
                return Err(USimpleError::new(1, format!(
                    "failed to copy '{}' to '{}': {}",
                    source.display(),
                    destination.display(),
                    error
                )));
            }
        }

        // Copy additional attributes if needed
        if self.preserve_attributes {
            self.copy_file_attributes(source, destination)?;
        }

        Ok(())
    }

    /// Copy large file with optimized I/O
    fn copy_large_file(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        if self.use_unbuffered_io {
            self.copy_with_unbuffered_io(source, destination, progress_reporter)
        } else {
            self.copy_with_memory_map(source, destination, progress_reporter)
        }
    }

    /// Copy file using unbuffered I/O for maximum performance
    fn copy_with_unbuffered_io(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        let source_wide = to_wide_string(source)?;
        let dest_wide = to_wide_string(destination)?;

        unsafe {
            // Open source file with unbuffered read
            let src_handle = CreateFileW(
                source_wide.as_ptr(),
                FILE_GENERIC_READ,
                0, // No sharing during copy
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN,
                INVALID_HANDLE_VALUE,
            );

            if src_handle == INVALID_HANDLE_VALUE {
                return Err(USimpleError::new(1, format!(
                    "cannot open source file '{}': {}",
                    source.display(),
                    io::Error::last_os_error()
                )));
            }

            // Open destination file with unbuffered write
            let dest_handle = CreateFileW(
                dest_wide.as_ptr(),
                FILE_GENERIC_WRITE,
                0, // No sharing during copy
                ptr::null_mut(),
                CREATE_ALWAYS,
                FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN,
                INVALID_HANDLE_VALUE,
            );

            if dest_handle == INVALID_HANDLE_VALUE {
                windows_sys::Win32::Foundation::CloseHandle(src_handle);
                return Err(USimpleError::new(1, format!(
                    "cannot create destination file '{}': {}",
                    destination.display(),
                    io::Error::last_os_error()
                )));
            }

            // Use sector-aligned buffer for unbuffered I/O
            let sector_size = get_sector_size(source)?;
            let aligned_buffer_size = align_to_sector(self.buffer_size, sector_size);

            // Allocate aligned buffer
            let mut buffer = vec![0u8; aligned_buffer_size];
            let file_size = source.metadata()?.len();
            let mut bytes_copied = 0u64;

            loop {
                let mut bytes_read = 0u32;
                let read_result = ReadFile(
                    src_handle,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    &mut bytes_read,
                    ptr::null_mut(),
                );

                if read_result == 0 || bytes_read == 0 {
                    break;
                }

                // Align write size to sector boundary for unbuffered I/O
                let write_size = if bytes_copied + bytes_read as u64 >= file_size {
                    // Last chunk - write exact remaining bytes
                    (file_size - bytes_copied) as u32
                } else {
                    // Align to sector size
                    align_to_sector(bytes_read as usize, sector_size) as u32
                };

                let mut bytes_written = 0u32;
                let write_result = WriteFile(
                    dest_handle,
                    buffer.as_ptr() as *const _,
                    write_size,
                    &mut bytes_written,
                    ptr::null_mut(),
                );

                if write_result == 0 {
                    windows_sys::Win32::Foundation::CloseHandle(src_handle);
                    windows_sys::Win32::Foundation::CloseHandle(dest_handle);
                    return Err(USimpleError::new(1, format!(
                        "write failed: {}",
                        io::Error::last_os_error()
                    )));
                }

                bytes_copied += bytes_read as u64;
                progress_reporter.update_progress(bytes_copied, file_size);
            }

            windows_sys::Win32::Foundation::CloseHandle(src_handle);
            windows_sys::Win32::Foundation::CloseHandle(dest_handle);
        }

        // Copy attributes
        if self.preserve_attributes {
            self.copy_file_attributes(source, destination)?;
        }

        Ok(())
    }

    /// Copy file using memory mapping for efficient large file copying
    fn copy_with_memory_map(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        let source_file = File::open(source)
            .map_err(|e| USimpleError::new(1, format!("cannot open '{}': {}", source.display(), e)))?;

        let file_size = source_file.metadata()?.len();

        if file_size == 0 {
            // Handle empty files
            std::fs::copy(source, destination)
                .map_err(|e| USimpleError::new(1, format!("copy failed: {}", e)))?;
            return Ok(());
        }

        let mmap = unsafe {
            Mmap::map(&source_file)
                .map_err(|e| USimpleError::new(1, format!("memory map failed: {}", e)))?
        };

        let mut dest_file = File::create(destination)
            .map_err(|e| USimpleError::new(1, format!("cannot create '{}': {}", destination.display(), e)))?;

        // Copy in chunks with progress reporting
        let chunk_size = self.buffer_size;
        let mut bytes_written = 0u64;

        for chunk in mmap.chunks(chunk_size) {
            dest_file.write_all(chunk)
                .map_err(|e| USimpleError::new(1, format!("write failed: {}", e)))?;

            bytes_written += chunk.len() as u64;
            progress_reporter.update_progress(bytes_written, file_size);
        }

        dest_file.sync_all()
            .map_err(|e| USimpleError::new(1, format!("sync failed: {}", e)))?;

        // Copy attributes
        if self.preserve_attributes {
            self.copy_file_attributes(source, destination)?;
        }

        Ok(())
    }

    /// Copy standard-sized file using buffered I/O
    fn copy_standard_file(
        &self,
        source: &Path,
        destination: &Path,
        progress_reporter: &ProgressReporter,
    ) -> UResult<()> {
        let mut source_file = File::open(source)
            .map_err(|e| USimpleError::new(1, format!("cannot open '{}': {}", source.display(), e)))?;

        let mut dest_file = File::create(destination)
            .map_err(|e| USimpleError::new(1, format!("cannot create '{}': {}", destination.display(), e)))?;

        let file_size = source_file.metadata()?.len();
        let mut buffer = vec![0u8; self.buffer_size];
        let mut bytes_copied = 0u64;

        loop {
            let bytes_read = source_file.read(&mut buffer)
                .map_err(|e| USimpleError::new(1, format!("read failed: {}", e)))?;

            if bytes_read == 0 {
                break;
            }

            dest_file.write_all(&buffer[..bytes_read])
                .map_err(|e| USimpleError::new(1, format!("write failed: {}", e)))?;

            bytes_copied += bytes_read as u64;
            progress_reporter.update_progress(bytes_copied, file_size);
        }

        dest_file.sync_all()
            .map_err(|e| USimpleError::new(1, format!("sync failed: {}", e)))?;

        // Copy attributes
        if self.preserve_attributes {
            self.copy_file_attributes(source, destination)?;
        }

        Ok(())
    }

    /// Copy file attributes (Windows-specific)
    fn copy_file_attributes(&self, source: &Path, destination: &Path) -> UResult<()> {
        let attrs = WindowsFileAttributes::from_path(source)?;
        attrs.apply_to_path(destination)?;
        Ok(())
    }

    /// Copy directory attributes
    pub fn copy_directory_attributes(&self, source_dir: &Path, dest_dir: &Path) -> UResult<()> {
        if self.preserve_attributes {
            self.copy_file_attributes(source_dir, dest_dir)?;
        }
        Ok(())
    }
}

/// Progress data for CopyFileEx callback
struct ProgressData {
    source: PathBuf,
    destination: PathBuf,
    bytes_copied: AtomicU64,
    total_bytes: AtomicU64,
    cancelled: AtomicBool,
}

impl ProgressData {
    fn new(source: &Path, destination: &Path) -> Self {
        Self {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            bytes_copied: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            cancelled: AtomicBool::new(false),
        }
    }
}

/// Progress callback for CopyFileEx
unsafe extern "system" fn copy_progress_callback(
    total_file_size: i64,
    total_bytes_transferred: i64,
    _stream_size: i64,
    _stream_bytes_transferred: i64,
    _stream_number: u32,
    _callback_reason: u32,
    _source_file: HANDLE,
    _destination_file: HANDLE,
    lpdata: *const std::ffi::c_void,
) -> u32 {
    if lpdata.is_null() {
        return 0; // PROGRESS_CONTINUE
    }

    let progress_data = &*(lpdata as *const ProgressData);

    progress_data.total_bytes.store(total_file_size as u64, Ordering::Relaxed);
    progress_data.bytes_copied.store(total_bytes_transferred as u64, Ordering::Relaxed);

    if progress_data.cancelled.load(Ordering::Relaxed) {
        return 1; // PROGRESS_CANCEL
    }

    0 // PROGRESS_CONTINUE
}

/// Drive type enumeration
#[derive(Debug, PartialEq)]
enum DriveType {
    Unknown,
    Fixed,
    Removable,
    Remote,
    CdRom,
    RamDisk,
}

/// Get drive type for a path
fn get_drive_type(path: &Path) -> UResult<DriveType> {
    let drive_letter = get_drive_letter(path);
    let drive_path = format!("{}\\", drive_letter);
    let drive_wide = to_wide_string(&PathBuf::from(drive_path))?;

    unsafe {
        let drive_type = GetDriveTypeW(drive_wide.as_ptr());
        match drive_type {
            2 => Ok(DriveType::Removable), // DRIVE_REMOVABLE
            3 => Ok(DriveType::Fixed),     // DRIVE_FIXED
            4 => Ok(DriveType::Remote),    // DRIVE_REMOTE
            5 => Ok(DriveType::CdRom),     // DRIVE_CDROM
            6 => Ok(DriveType::RamDisk),   // DRIVE_RAMDISK
            _ => Ok(DriveType::Unknown),
        }
    }
}

/// Get drive letter from path
fn get_drive_letter(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// Convert path to wide string for Windows APIs
fn to_wide_string(path: &Path) -> UResult<Vec<u16>> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0); // Null terminator
    Ok(wide)
}

/// Get sector size for a drive
fn get_sector_size(path: &Path) -> UResult<usize> {
    // Default to 4KB sectors for most modern drives
    // TODO @codex: Implement actual sector size detection using GetDiskFreeSpace
    Ok(4096)
}

/// Align size to sector boundary
fn align_to_sector(size: usize, sector_size: usize) -> usize {
    (size + sector_size - 1) & !(sector_size - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::write;

    #[test]
    fn test_get_drive_letter() {
        assert_eq!(get_drive_letter(Path::new("C:\\temp\\file.txt")), "C:");
        assert_eq!(get_drive_letter(Path::new("D:\\folder")), "D:");
    }

    #[test]
    fn test_align_to_sector() {
        assert_eq!(align_to_sector(1000, 512), 1024);
        assert_eq!(align_to_sector(512, 512), 512);
        assert_eq!(align_to_sector(1, 4096), 4096);
    }

    #[test]
    fn test_copy_engine_creation() {
        let options = WindowsCpOptions {
            sources: vec![],
            destination: PathBuf::new(),
            recursive: false,
            preserve_all: false,
            preserve_timestamps: false,
            preserve_ownership: false,
            preserve_permissions: false,
            preserve_links: false,
            preserve_security: false,
            preserve_streams: false,
            follow_junctions: false,
            preserve_junctions: false,
            force: false,
            interactive: false,
            no_clobber: false,
            verbose: false,
            progress: false,
            unbuffered: true,
            parallel_threads: None,
            target_directory: None,
            no_target_directory: false,
            update: false,
            backup: None,
            dereference: false,
            no_dereference: false,
            strip_trailing_slashes: false,
        };

        let engine = CopyEngine::new(&options).unwrap();
        assert!(engine.use_unbuffered_io);
        assert_eq!(engine.buffer_size, 64 * 1024 * 1024);
    }
}
