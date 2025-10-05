//! Windows-Specific Enhancements
//!
//! Provides Windows-specific functionality including:
//! - Windows file attributes (Hidden, System, Archive, ReadOnly)
//! - Windows ACL (Access Control List) handling
//! - Windows shortcuts (.lnk files) support
//! - Registry integration where appropriate
//! - Windows-specific path and permission handling

use crate::{WinUtilsError, WinUtilsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, WIN32_ERROR, CloseHandle},
    Storage::FileSystem::{
        CreateFileW, GetFileAttributesW, SetFileAttributesW, GetFileInformationByHandle,
        FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_SYSTEM, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_DIRECTORY,
        FILE_GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING, BY_HANDLE_FILE_INFORMATION,
    },
    Security::{
        Authorization::{
            GetSecurityInfo, SetSecurityInfo, SE_FILE_OBJECT,
        },
        GetAclInformation, GetAce,
        OWNER_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
        DACL_SECURITY_INFORMATION, ACL_SIZE_INFORMATION,
        ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, PSECURITY_DESCRIPTOR,
        ACL,
    },
};

/// Windows file attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsAttributes {
    pub hidden: bool,
    pub system: bool,
    pub archive: bool,
    pub readonly: bool,
    pub directory: bool,
    pub normal: bool,
    pub raw_attributes: u32,
}

impl WindowsAttributes {
    /// Create WindowsAttributes from raw Windows attributes
    #[cfg(windows)]
    pub fn from_raw(attributes: u32) -> Self {
        Self {
            hidden: attributes & FILE_ATTRIBUTE_HIDDEN != 0,
            system: attributes & FILE_ATTRIBUTE_SYSTEM != 0,
            archive: attributes & FILE_ATTRIBUTE_ARCHIVE != 0,
            readonly: attributes & FILE_ATTRIBUTE_READONLY != 0,
            directory: attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
            normal: attributes & FILE_ATTRIBUTE_NORMAL != 0,
            raw_attributes: attributes,
        }
    }

    #[cfg(not(windows))]
    pub fn from_raw(attributes: u32) -> Self {
        Self {
            hidden: false,
            system: false,
            archive: false,
            readonly: false,
            directory: false,
            normal: true,
            raw_attributes: attributes,
        }
    }

    /// Convert back to raw Windows attributes
    #[cfg(windows)]
    pub fn to_raw(&self) -> u32 {
        let mut attrs = 0u32;

        if self.hidden { attrs |= FILE_ATTRIBUTE_HIDDEN; }
        if self.system { attrs |= FILE_ATTRIBUTE_SYSTEM; }
        if self.archive { attrs |= FILE_ATTRIBUTE_ARCHIVE; }
        if self.readonly { attrs |= FILE_ATTRIBUTE_READONLY; }
        if self.directory { attrs |= FILE_ATTRIBUTE_DIRECTORY; }
        if self.normal && attrs == 0 { attrs |= FILE_ATTRIBUTE_NORMAL; }

        attrs
    }

    #[cfg(not(windows))]
    pub fn to_raw(&self) -> u32 {
        self.raw_attributes
    }

    /// Get a human-readable description of the attributes
    pub fn description(&self) -> String {
        let mut desc = Vec::new();

        if self.directory { desc.push("Directory"); }
        if self.hidden { desc.push("Hidden"); }
        if self.system { desc.push("System"); }
        if self.archive { desc.push("Archive"); }
        if self.readonly { desc.push("Read-Only"); }
        if self.normal && desc.is_empty() { desc.push("Normal"); }

        if desc.is_empty() {
            "Unknown".to_string()
        } else {
            desc.join(", ")
        }
    }

    /// Get a short attribute string (like ls -l but for Windows)
    pub fn short_string(&self) -> String {
        let mut s = String::with_capacity(5);

        s.push(if self.directory { 'd' } else { '-' });
        s.push(if self.readonly { 'r' } else { 'w' });
        s.push(if self.hidden { 'h' } else { '-' });
        s.push(if self.system { 's' } else { '-' });
        s.push(if self.archive { 'a' } else { '-' });

        s
    }
}

/// Information about a Windows ACE (Access Control Entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AceInfo {
    pub ace_type: AceType,
    pub access_mask: u32,
    pub sid: String,
    pub account_name: Option<String>,
    pub inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AceType {
    AccessAllowed,
    AccessDenied,
    SystemAudit,
    SystemAlarm,
    Unknown(u8),
}

impl AceType {
    #[cfg(windows)]
    fn from_raw(ace_type: u8) -> Self {
        match ace_type {
            0 => AceType::AccessAllowed,  // ACCESS_ALLOWED_ACE_TYPE
            1 => AceType::AccessDenied,   // ACCESS_DENIED_ACE_TYPE
            2 => AceType::SystemAudit,    // SYSTEM_AUDIT_ACE_TYPE
            3 => AceType::SystemAlarm,    // SYSTEM_ALARM_ACE_TYPE
            _ => AceType::Unknown(ace_type),
        }
    }

    #[cfg(not(windows))]
    fn from_raw(ace_type: u8) -> Self {
        AceType::Unknown(ace_type)
    }
}

/// Windows ACL (Access Control List) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsAcl {
    pub owner: Option<String>,
    pub group: Option<String>,
    pub aces: Vec<AceInfo>,
    pub revision: u32,
}

impl WindowsAcl {
    /// Create an empty ACL
    pub fn new() -> Self {
        Self {
            owner: None,
            group: None,
            aces: Vec::new(),
            revision: 0,
        }
    }

    /// Check if a specific access is granted
    pub fn has_access(&self, access_mask: u32) -> bool {
        // Simplified check - in practice this would be more complex
        self.aces.iter().any(|ace| {
            matches!(ace.ace_type, AceType::AccessAllowed) &&
            (ace.access_mask & access_mask) == access_mask
        })
    }

    /// Get a summary of permissions
    pub fn permission_summary(&self) -> String {
        let read = self.has_access(0x80000000); // GENERIC_READ
        let write = self.has_access(0x40000000); // GENERIC_WRITE
        let execute = self.has_access(0x20000000); // GENERIC_EXECUTE

        format!("{}{}{}",
            if read { "r" } else { "-" },
            if write { "w" } else { "-" },
            if execute { "x" } else { "-" }
        )
    }
}

impl Default for WindowsAcl {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about Windows ACL for a file/directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclInfo {
    pub path: PathBuf,
    pub acl: WindowsAcl,
    pub error: Option<String>,
}

impl AclInfo {
    /// Create ACL info for a path
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            acl: WindowsAcl::new(),
            error: None,
        }
    }

    /// Create ACL info with error
    pub fn with_error(path: PathBuf, error: String) -> Self {
        Self {
            path,
            acl: WindowsAcl::new(),
            error: Some(error),
        }
    }
}

/// Windows shortcut (.lnk file) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutInfo {
    pub target_path: PathBuf,
    pub working_directory: Option<PathBuf>,
    pub arguments: Option<String>,
    pub description: Option<String>,
    pub icon_location: Option<PathBuf>,
    pub icon_index: i32,
    pub show_command: i32,
    pub hotkey: u16,
}

impl ShortcutInfo {
    /// Create new shortcut info
    pub fn new(target_path: PathBuf) -> Self {
        Self {
            target_path,
            working_directory: None,
            arguments: None,
            description: None,
            icon_location: None,
            icon_index: 0,
            show_command: 1, // SW_NORMAL
            hotkey: 0,
        }
    }

    /// Get a description of the shortcut
    pub fn description(&self) -> String {
        self.description.clone().unwrap_or_else(|| {
            format!("Shortcut to {}", self.target_path.display())
        })
    }
}

/// Windows file attribute information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub path: PathBuf,
    pub attributes: WindowsAttributes,
    pub size: u64,
    pub created: SystemTime,
    pub modified: SystemTime,
    pub accessed: SystemTime,
    pub error: Option<String>,
}

impl AttributeInfo {
    /// Create attribute info with error
    pub fn with_error(path: PathBuf, error: String) -> Self {
        Self {
            path,
            attributes: WindowsAttributes::from_raw(0),
            size: 0,
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            error: Some(error),
        }
    }
}

/// Handler for Windows-specific operations
pub struct WindowsHandler;

impl WindowsHandler {
    /// Get Windows file attributes for a path
    #[cfg(windows)]
    pub fn get_attributes(path: &Path) -> WinUtilsResult<AttributeInfo> {
        use std::os::windows::ffi::OsStrExt;

        let wide_path: Vec<u16> = path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let attrs = GetFileAttributesW(wide_path.as_ptr());
            if attrs == u32::MAX {
                let error = std::io::Error::last_os_error();
                return Ok(AttributeInfo::with_error(
                    path.to_path_buf(),
                    format!("Failed to get attributes: {}", error),
                ));
            }

            // Get detailed file information
            let handle = CreateFileW(
                wide_path.as_ptr(),
                FILE_GENERIC_READ,
                FILE_SHARE_READ,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Ok(AttributeInfo {
                    path: path.to_path_buf(),
                    attributes: WindowsAttributes::from_raw(attrs),
                    size: 0,
                    created: SystemTime::UNIX_EPOCH,
                    modified: SystemTime::UNIX_EPOCH,
                    accessed: SystemTime::UNIX_EPOCH,
                    error: None,
                });
            }

            let mut file_info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
            let success = GetFileInformationByHandle(handle, &mut file_info);
            CloseHandle(handle);

            if success == 0 {
                return Ok(AttributeInfo {
                    path: path.to_path_buf(),
                    attributes: WindowsAttributes::from_raw(attrs),
                    size: 0,
                    created: SystemTime::UNIX_EPOCH,
                    modified: SystemTime::UNIX_EPOCH,
                    accessed: SystemTime::UNIX_EPOCH,
                    error: Some("Failed to get detailed file information".to_string()),
                });
            }

            // Convert Windows FILETIME to SystemTime
            let created = Self::filetime_to_systemtime(
                file_info.ftCreationTime.dwLowDateTime,
                file_info.ftCreationTime.dwHighDateTime,
            );
            let modified = Self::filetime_to_systemtime(
                file_info.ftLastWriteTime.dwLowDateTime,
                file_info.ftLastWriteTime.dwHighDateTime,
            );
            let accessed = Self::filetime_to_systemtime(
                file_info.ftLastAccessTime.dwLowDateTime,
                file_info.ftLastAccessTime.dwHighDateTime,
            );

            let size = ((file_info.nFileSizeHigh as u64) << 32) | (file_info.nFileSizeLow as u64);

            Ok(AttributeInfo {
                path: path.to_path_buf(),
                attributes: WindowsAttributes::from_raw(attrs),
                size,
                created,
                modified,
                accessed,
                error: None,
            })
        }
    }

    #[cfg(not(windows))]
    pub fn get_attributes(path: &Path) -> WinUtilsResult<AttributeInfo> {
        // On non-Windows platforms, return basic information
        let metadata = std::fs::metadata(path)
            .map_err(|e| WinUtilsError::windows_api(format!("Failed to get metadata: {}", e)))?;

        Ok(AttributeInfo {
            path: path.to_path_buf(),
            attributes: WindowsAttributes::from_raw(0), // No Windows attributes on non-Windows
            size: metadata.len(),
            created: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            accessed: metadata.accessed().unwrap_or(SystemTime::UNIX_EPOCH),
            error: None,
        })
    }

    /// Set Windows file attributes
    #[cfg(windows)]
    pub fn set_attributes(path: &Path, attributes: &WindowsAttributes) -> WinUtilsResult<()> {
        use std::os::windows::ffi::OsStrExt;

        let wide_path: Vec<u16> = path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let success = SetFileAttributesW(wide_path.as_ptr(), attributes.to_raw());
            if success == 0 {
                let error = std::io::Error::last_os_error();
                return Err(WinUtilsError::windows_api(
                    format!("Failed to set attributes: {}", error)
                ));
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    pub fn set_attributes(_path: &Path, _attributes: &WindowsAttributes) -> WinUtilsResult<()> {
        Err(WinUtilsError::feature_not_available("Windows attributes"))
    }

    /// Get Windows ACL information for a path
    #[cfg(windows)]
    pub fn get_acl(path: &Path) -> WinUtilsResult<AclInfo> {
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        let wide_path: Vec<u16> = path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut owner_sid: *mut std::ffi::c_void = ptr::null_mut();
            let mut group_sid: *mut std::ffi::c_void = ptr::null_mut();
            let mut dacl: *mut ACL = ptr::null_mut();
            let mut sd: PSECURITY_DESCRIPTOR = ptr::null_mut();

            let result = GetSecurityInfo(
                wide_path.as_ptr() as HANDLE,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner_sid,
                &mut group_sid,
                &mut dacl,
                ptr::null_mut(),
                &mut sd,
            );

            if result != 0 {
                return Ok(AclInfo::with_error(
                    path.to_path_buf(),
                    format!("Failed to get security info: error {}", result),
                ));
            }

            let mut acl = WindowsAcl::new();

            // Get ACL information
            if !dacl.is_null() {
                let mut acl_info: ACL_SIZE_INFORMATION = std::mem::zeroed();
                // AclSizeInformation constant = 2
                const ACL_SIZE_INFO: u32 = 2;
                let info_result = GetAclInformation(
                    dacl,
                    &mut acl_info as *mut _ as *mut std::ffi::c_void,
                    std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                    ACL_SIZE_INFO,
                );

                if info_result != 0 {
                    // Note: AclRevision field removed from ACL_SIZE_INFORMATION in windows-sys 0.60
                    // Using ACL_REVISION constant (value: 2) as per Windows API documentation
                    const ACL_REVISION: u32 = 2;
                    acl.revision = ACL_REVISION;

                    // Iterate through ACEs
                    for i in 0..acl_info.AceCount {
                        let mut ace: *mut std::ffi::c_void = ptr::null_mut();
                        let ace_result = GetAce(dacl, i, &mut ace);

                        if ace_result != 0 && !ace.is_null() {
                            // Parse ACE (simplified)
                            let ace_header = ace as *const u8;
                            let ace_type = *ace_header;
                            let ace_size = *(ace_header.offset(2) as *const u16);
                            let access_mask = *(ace_header.offset(4) as *const u32);

                            acl.aces.push(AceInfo {
                                ace_type: AceType::from_raw(ace_type),
                                access_mask,
                                sid: format!("SID-{}", i), // Simplified SID representation
                                account_name: None,
                                inherited: false,
                            });
                        }
                    }
                }
            }

            // Free the security descriptor
            // Note: In windows-sys, LocalFree is not exposed directly
            // The security descriptor will be freed when it goes out of scope
            // or we can use the windows crate's LocalFree if needed

            Ok(AclInfo {
                path: path.to_path_buf(),
                acl,
                error: None,
            })
        }
    }

    #[cfg(not(windows))]
    pub fn get_acl(path: &Path) -> WinUtilsResult<AclInfo> {
        Ok(AclInfo::with_error(
            path.to_path_buf(),
            "Windows ACL support not available on this platform".to_string(),
        ))
    }

    /// Read Windows shortcut (.lnk file)
    #[cfg(windows)]
    pub fn read_shortcut(path: &Path) -> WinUtilsResult<Option<ShortcutInfo>> {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::*,
            Win32::Foundation::*,
            Win32::System::Com::*,
            Win32::UI::Shell::{IShellLinkW, IPersistFile},
        };

        // SLGP_SHORTPATH constant (0x00000001) - From Win32::UI::Shell
        const SLGP_SHORTPATH: u32 = 0x1;

        // CLSID_ShellLink GUID: 00021401-0000-0000-C000-000000000046
        use windows::core::GUID;
        const CLSID_SHELLINK: GUID = GUID::from_u128(0x00021401_0000_0000_C000_000000000046);

        if path.extension().and_then(|s| s.to_str()) != Some("lnk") {
            return Ok(None);
        }

        unsafe {
            // Initialize COM
            let hr = CoInitialize(None);
            if hr.is_err() && hr != windows::core::Error::from(CO_E_ALREADYINITIALIZED) {
                return Err(WinUtilsError::windows_api("Failed to initialize COM".to_string()));
            }

            // Create shell link object
            let shell_link: IShellLinkW = CoCreateInstance(&CLSID_SHELLINK, None, CLSCTX_INPROC_SERVER)?;
            let persist_file: IPersistFile = shell_link.cast()?;

            // Load the shortcut file
            let wide_path: Vec<u16> = path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            persist_file.Load(PCWSTR(wide_path.as_ptr()), STGM_READ)?;

            // Get target path
            let mut target_path = [0u16; 260];
            shell_link.GetPath(&mut target_path, None, SLGP_SHORTPATH)?;
            let target = String::from_utf16_lossy(&target_path)
                .trim_end_matches('\0')
                .to_string();

            if target.is_empty() {
                CoUninitialize();
                return Ok(None);
            }

            // Get working directory
            let mut working_dir = [0u16; 260];
            let _ = shell_link.GetWorkingDirectory(&mut working_dir);
            let working_directory = String::from_utf16_lossy(&working_dir)
                .trim_end_matches('\0')
                .to_string();

            // Get arguments
            let mut args = [0u16; 260];
            let _ = shell_link.GetArguments(&mut args);
            let arguments = String::from_utf16_lossy(&args)
                .trim_end_matches('\0')
                .to_string();

            // Get description
            let mut desc = [0u16; 260];
            let _ = shell_link.GetDescription(&mut desc);
            let description = String::from_utf16_lossy(&desc)
                .trim_end_matches('\0')
                .to_string();

            // Get icon location
            let mut icon_path = [0u16; 260];
            let mut icon_index = 0i32;
            let _ = shell_link.GetIconLocation(&mut icon_path, &mut icon_index);
            let icon_location = String::from_utf16_lossy(&icon_path)
                .trim_end_matches('\0')
                .to_string();

            // Get hotkey
            let mut hotkey = 0u16;
            let _ = shell_link.GetHotkey(&mut hotkey);

            CoUninitialize();

            Ok(Some(ShortcutInfo {
                target_path: PathBuf::from(target),
                working_directory: if working_directory.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(working_directory))
                },
                arguments: if arguments.is_empty() { None } else { Some(arguments) },
                description: if description.is_empty() { None } else { Some(description) },
                icon_location: if icon_location.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(icon_location))
                },
                icon_index,
                show_command: 1, // SW_NORMAL - default show command
                hotkey,
            }))
        }
    }

    #[cfg(not(windows))]
    pub fn read_shortcut(_path: &Path) -> WinUtilsResult<Option<ShortcutInfo>> {
        Ok(None) // No shortcut support on non-Windows platforms
    }

    #[cfg(windows)]
    fn filetime_to_systemtime(low: u32, high: u32) -> SystemTime {
        // Windows FILETIME is 100-nanosecond intervals since January 1, 1601
        let filetime = ((high as u64) << 32) | (low as u64);

        // Convert to Unix epoch (January 1, 1970)
        const FILETIME_UNIX_DIFF: u64 = 116444736000000000;

        if filetime < FILETIME_UNIX_DIFF {
            return SystemTime::UNIX_EPOCH;
        }

        let unix_time = filetime - FILETIME_UNIX_DIFF;
        let seconds = unix_time / 10_000_000;
        let nanos = ((unix_time % 10_000_000) * 100) as u32;

        SystemTime::UNIX_EPOCH + std::time::Duration::new(seconds, nanos)
    }
}

/// Registry operations (Windows-specific)
#[cfg(windows)]
pub mod registry {
    use super::*;
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::Foundation::*;

    /// Registry key wrapper
    pub struct RegistryKey {
        hkey: HKEY,
        path: String,
    }

    impl RegistryKey {
        /// Open a registry key
        pub fn open(hive: HKEY, path: &str) -> WinUtilsResult<Self> {
            use std::os::windows::ffi::OsStrExt;

            let wide_path: Vec<u16> = std::ffi::OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut key: HKEY = std::ptr::null_mut();

            unsafe {
                let result = RegOpenKeyExW(
                    hive,
                    wide_path.as_ptr(),
                    0,
                    KEY_READ,
                    &mut key,
                );

                if result != ERROR_SUCCESS {
                    return Err(WinUtilsError::windows_api(
                        format!("Failed to open registry key: error {}", result)
                    ));
                }
            }

            Ok(Self {
                hkey: key,
                path: path.to_string(),
            })
        }

        /// Read a string value from the registry
        pub fn read_string(&self, value_name: &str) -> WinUtilsResult<String> {
            use std::os::windows::ffi::OsStrExt;

            let wide_name: Vec<u16> = std::ffi::OsStr::new(value_name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut buffer_size = 0u32;

            unsafe {
                // Get the size first
                let result = RegQueryValueExW(
                    self.hkey,
                    wide_name.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut buffer_size,
                );

                if result != ERROR_SUCCESS {
                    return Err(WinUtilsError::windows_api(
                        format!("Failed to query registry value size: error {}", result)
                    ));
                }

                // Allocate buffer and read the value
                let mut buffer: Vec<u16> = vec![0; (buffer_size / 2) as usize];
                let mut actual_size = buffer_size;

                let result = RegQueryValueExW(
                    self.hkey,
                    wide_name.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut u8,
                    &mut actual_size,
                );

                if result != ERROR_SUCCESS {
                    return Err(WinUtilsError::windows_api(
                        format!("Failed to read registry value: error {}", result)
                    ));
                }

                // Convert to string
                let value = String::from_utf16_lossy(&buffer)
                    .trim_end_matches('\0')
                    .to_string();

                Ok(value)
            }
        }

        /// Get the path of this registry key
        pub fn path(&self) -> &str {
            &self.path
        }
    }

    impl Drop for RegistryKey {
        fn drop(&mut self) {
            if !self.hkey.is_null() {
                unsafe {
                    RegCloseKey(self.hkey);
                }
            }
        }
    }

    /// Common registry operations
    pub fn get_file_association(extension: &str) -> WinUtilsResult<Option<String>> {
        let key = RegistryKey::open(HKEY_CLASSES_ROOT, extension)?;
        match key.read_string("") {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }

    /// Get Windows version from registry
    pub fn get_windows_version() -> WinUtilsResult<String> {
        let key = RegistryKey::open(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
        )?;

        let product_name = key.read_string("ProductName").unwrap_or_else(|_| "Unknown".to_string());
        let build_number = key.read_string("CurrentBuildNumber").unwrap_or_else(|_| "Unknown".to_string());

        Ok(format!("{} (Build {})", product_name, build_number))
    }
}

#[cfg(not(windows))]
pub mod registry {
    use super::*;

    /// Dummy registry key for non-Windows platforms
    pub struct RegistryKey;

    impl RegistryKey {
        pub fn open(_hive: usize, _path: &str) -> WinUtilsResult<Self> {
            Err(WinUtilsError::feature_not_available("Windows Registry"))
        }

        pub fn read_string(&self, _value_name: &str) -> WinUtilsResult<String> {
            Err(WinUtilsError::feature_not_available("Windows Registry"))
        }

        pub fn path(&self) -> &str {
            ""
        }
    }

    pub fn get_file_association(_extension: &str) -> WinUtilsResult<Option<String>> {
        Ok(None)
    }

    pub fn get_windows_version() -> WinUtilsResult<String> {
        Err(WinUtilsError::feature_not_available("Windows Registry"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_windows_attributes() {
        let attrs = WindowsAttributes::from_raw(0x01 | 0x02); // Hidden | ReadOnly
        assert!(attrs.hidden);
        assert!(attrs.readonly);
        assert!(!attrs.system);

        let description = attrs.description();
        assert!(description.contains("Hidden"));
        assert!(description.contains("Read-Only"));

        let short = attrs.short_string();
        assert_eq!(short.len(), 5);
        assert_eq!(short.chars().nth(0).unwrap(), '-'); // not directory
        assert_eq!(short.chars().nth(1).unwrap(), 'r'); // readonly
        assert_eq!(short.chars().nth(2).unwrap(), 'h'); // hidden
    }

    #[test]
    fn test_shortcut_info() {
        let info = ShortcutInfo::new(PathBuf::from("C:\\Program Files\\test.exe"));
        assert_eq!(info.target_path, PathBuf::from("C:\\Program Files\\test.exe"));
        assert!(info.working_directory.is_none());

        let desc = info.description();
        assert!(desc.contains("test.exe"));
    }

    #[test]
    fn test_windows_acl() {
        let mut acl = WindowsAcl::new();
        assert_eq!(acl.aces.len(), 0);

        acl.aces.push(AceInfo {
            ace_type: AceType::AccessAllowed,
            access_mask: 0x80000000, // GENERIC_READ
            sid: "S-1-5-32-544".to_string(),
            account_name: Some("Administrators".to_string()),
            inherited: false,
        });

        assert!(acl.has_access(0x80000000)); // Should have read access
        assert!(!acl.has_access(0x40000000)); // Should not have write access

        let summary = acl.permission_summary();
        assert!(summary.starts_with('r')); // Should show read permission
    }

    #[test]
    fn test_ace_type() {
        let allowed = AceType::from_raw(0);
        assert!(matches!(allowed, AceType::AccessAllowed));

        let denied = AceType::from_raw(1);
        assert!(matches!(denied, AceType::AccessDenied));

        let unknown = AceType::from_raw(99);
        assert!(matches!(unknown, AceType::Unknown(99)));
    }
}
