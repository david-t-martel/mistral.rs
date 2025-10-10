use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::*,
    System::Console::*,
};

// Constants that might not be available in windows-sys
const CP_UTF8: u32 = 65001;
const FILE_SUPPORTS_LONG_NAMES: u32 = 0x0000004;
const FILE_FILE_COMPRESSION: u32 = 0x0000010;
const FILE_SUPPORTS_ENCRYPTION: u32 = 0x0020000;

/// Windows file attributes representation
#[derive(Debug, Clone)]
pub struct FileAttributes {
    pub hidden: bool,
    pub system: bool,
    pub archive: bool,
    pub readonly: bool,
    pub compressed: bool,
    pub encrypted: bool,
    pub temporary: bool,
    pub sparse: bool,
    pub reparse_point: bool,
    pub offline: bool,
}

impl FileAttributes {
    pub fn from_path(path: &Path) -> Result<Self> {
        let wide_path = to_wide_string(path)?;

        unsafe {
            let attrs = GetFileAttributesW(wide_path.as_ptr());
            if attrs == INVALID_FILE_ATTRIBUTES {
                return Err(std::io::Error::last_os_error().into());
            }

            Ok(Self::from_raw(attrs))
        }
    }

    fn from_raw(attrs: u32) -> Self {
        Self {
            hidden: attrs & FILE_ATTRIBUTE_HIDDEN != 0,
            system: attrs & FILE_ATTRIBUTE_SYSTEM != 0,
            archive: attrs & FILE_ATTRIBUTE_ARCHIVE != 0,
            readonly: attrs & FILE_ATTRIBUTE_READONLY != 0,
            compressed: attrs & FILE_ATTRIBUTE_COMPRESSED != 0,
            encrypted: attrs & FILE_ATTRIBUTE_ENCRYPTED != 0,
            temporary: attrs & FILE_ATTRIBUTE_TEMPORARY != 0,
            sparse: attrs & FILE_ATTRIBUTE_SPARSE_FILE != 0,
            reparse_point: attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0,
            offline: attrs & FILE_ATTRIBUTE_OFFLINE != 0,
        }
    }

    /// Get compact attribute string (e.g., "HAS" for Hidden, System, Archive)
    pub fn to_compact_string(&self) -> String {
        let mut attrs = String::new();
        if self.hidden { attrs.push('H'); }
        if self.system { attrs.push('S'); }
        if self.archive { attrs.push('A'); }
        if self.readonly { attrs.push('R'); }
        if self.compressed { attrs.push('C'); }
        if self.encrypted { attrs.push('E'); }
        if self.temporary { attrs.push('T'); }
        if self.sparse { attrs.push('P'); }
        if self.reparse_point { attrs.push('L'); }
        if self.offline { attrs.push('O'); }
        attrs
    }

    /// Get full attribute description
    pub fn to_description(&self) -> Vec<&'static str> {
        let mut desc = Vec::new();
        if self.hidden { desc.push("Hidden"); }
        if self.system { desc.push("System"); }
        if self.archive { desc.push("Archive"); }
        if self.readonly { desc.push("Read-only"); }
        if self.compressed { desc.push("Compressed"); }
        if self.encrypted { desc.push("Encrypted"); }
        if self.temporary { desc.push("Temporary"); }
        if self.sparse { desc.push("Sparse"); }
        if self.reparse_point { desc.push("Reparse Point"); }
        if self.offline { desc.push("Offline"); }
        desc
    }
}

/// Junction point and symbolic link information
#[derive(Debug, Clone)]
pub struct ReparsePointInfo {
    pub is_symlink: bool,
    pub is_junction: bool,
    pub target: Option<PathBuf>,
}

impl ReparsePointInfo {
    pub fn from_path(path: &Path) -> Result<Option<Self>> {
        let attrs = FileAttributes::from_path(path)?;
        if !attrs.reparse_point {
            return Ok(None);
        }

        // For now, we'll use basic detection
        // TODO @gemini: Implement full reparse point parsing
        let metadata = std::fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            let target = std::fs::read_link(path).ok();
            Ok(Some(Self {
                is_symlink: true,
                is_junction: false,
                target,
            }))
        } else {
            // Assume it's a junction point if it's a reparse point but not a symlink
            Ok(Some(Self {
                is_symlink: false,
                is_junction: true,
                target: None, // TODO @codex: Read junction target
            }))
        }
    }
}

/// Alternate Data Streams information
#[derive(Debug, Clone)]
pub struct AlternateDataStreams {
    pub streams: Vec<String>,
}

impl AlternateDataStreams {
    pub fn from_path(_path: &Path) -> Result<Self> {
        // TODO @gemini: Implement ADS enumeration using FindFirstStreamW/FindNextStreamW
        Ok(Self {
            streams: Vec::new(),
        })
    }
}

/// Set up Windows console for proper Unicode and color support
pub fn setup_console() {
    unsafe {
        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout_handle != INVALID_HANDLE_VALUE {
            let mut mode = 0;
            if GetConsoleMode(stdout_handle, &mut mode) != 0 {
                // Enable virtual terminal processing for color support
                let new_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                SetConsoleMode(stdout_handle, new_mode);
            }
        }

        // Set console output to UTF-8
        SetConsoleOutputCP(CP_UTF8);
    }
}

/// Check if a path supports long path names (>260 characters)
pub fn supports_long_paths(path: &Path) -> bool {
    // Check if the path is already a long path or if we can access it
    if path.as_os_str().len() > 260 {
        return path.exists(); // If it exists, long paths are supported
    }

    // For now, assume long paths are supported on Windows 10 1607+
    true
}

/// Convert a path to a wide string for Windows APIs
fn to_wide_string(path: &Path) -> Result<Vec<u16>> {
    let path_str = path.as_os_str();
    let mut wide: Vec<u16> = path_str.encode_wide().collect();

    // Handle long paths by prefixing with \\?\
    if wide.len() > 260 && !path_str.to_string_lossy().starts_with(r"\\?\") {
        let long_prefix: Vec<u16> = OsString::from(r"\\?\").encode_wide().collect();
        let mut long_path = long_prefix;
        long_path.extend_from_slice(&wide);
        wide = long_path;
    }

    wide.push(0); // Null terminator
    Ok(wide)
}

/// Get the volume information for a path
pub fn get_volume_info(path: &Path) -> Result<VolumeInfo> {
    let root_path = get_volume_root(path)?;
    let wide_root = to_wide_string(&root_path)?;

    let mut volume_name = vec![0u16; MAX_PATH as usize];
    let mut file_system_name = vec![0u16; MAX_PATH as usize];
    let mut serial_number = 0;
    let mut max_component_length = 0;
    let mut file_system_flags = 0;

    unsafe {
        let success = GetVolumeInformationW(
            wide_root.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut serial_number,
            &mut max_component_length,
            &mut file_system_flags,
            file_system_name.as_mut_ptr(),
            file_system_name.len() as u32,
        );

        if success == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }

    Ok(VolumeInfo {
        volume_name: wide_string_to_string(&volume_name),
        file_system: wide_string_to_string(&file_system_name),
        serial_number,
        max_component_length,
        supports_long_filenames: file_system_flags & FILE_SUPPORTS_LONG_NAMES != 0,
        supports_compression: file_system_flags & FILE_FILE_COMPRESSION != 0,
        supports_encryption: file_system_flags & FILE_SUPPORTS_ENCRYPTION != 0,
    })
}

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub volume_name: String,
    pub file_system: String,
    pub serial_number: u32,
    pub max_component_length: u32,
    pub supports_long_filenames: bool,
    pub supports_compression: bool,
    pub supports_encryption: bool,
}

fn get_volume_root(path: &Path) -> Result<PathBuf> {
    let absolute = dunce::canonicalize(path)?;
    let mut components = absolute.components();

    match components.next() {
        Some(std::path::Component::Prefix(prefix)) => {
            let mut root = PathBuf::new();
            root.push(prefix.as_os_str());
            root.push("\\");
            Ok(root)
        }
        _ => Ok(PathBuf::from("\\")),
    }
}

fn wide_string_to_string(wide: &[u16]) -> String {
    let null_pos = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    OsString::from_wide(&wide[..null_pos]).to_string_lossy().into_owned()
}
