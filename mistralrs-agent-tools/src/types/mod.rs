//! Core types and error handling for agent tools.

pub mod security;

use std::path::PathBuf;

pub use security::{
    CommandPolicy, NetworkPolicy, ResourceLimits, SandboxPolicy, SecurityLevel, SecurityPolicy,
};

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Errors that can occur during agent operations
#[derive(Debug, Clone)]
pub enum AgentError {
    /// Path-related errors
    PathError(String),
    /// Sandbox violation
    SandboxViolation(String),
    /// I/O error
    IoError(String),
    /// Invalid input
    InvalidInput(String),
    /// Permission denied
    PermissionDenied(String),
    /// File not found
    NotFound(String),
    /// Operation not supported
    Unsupported(String),
    /// Encoding error
    EncodingError(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathError(msg) => write!(f, "Path error: {}", msg),
            Self::SandboxViolation(msg) => write!(f, "Sandbox violation: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            Self::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound(err.to_string()),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied(err.to_string()),
            _ => Self::IoError(err.to_string()),
        }
    }
}

impl From<crate::pathlib::PathError> for AgentError {
    fn from(err: crate::pathlib::PathError) -> Self {
        Self::PathError(err.to_string())
    }
}

/// Sandbox configuration for agent operations
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Root directory that operations are restricted to
    pub root: PathBuf,
    /// Whether to allow reading outside sandbox (but not writing)
    /// DEPRECATED: Use security_policy.sandbox_policy.allow_read_outside instead
    pub allow_read_outside: bool,
    /// Maximum file size to read (in bytes)
    /// DEPRECATED: Use security_policy.resource_limits.max_file_size instead
    pub max_read_size: usize,
    /// Maximum number of files to process in batch operations
    /// DEPRECATED: Use security_policy.resource_limits.max_batch_size instead
    pub max_batch_size: usize,
    /// Security policy (optional, for enhanced security controls)
    pub security_policy: Option<SecurityPolicy>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            allow_read_outside: false,
            max_read_size: 100 * 1024 * 1024, // 100MB
            max_batch_size: 1000,
            security_policy: None, // Legacy mode by default
        }
    }
}

impl SandboxConfig {
    /// Creates a new sandbox config with the given root
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            ..Default::default()
        }
    }

    /// Creates a new sandbox config with a security policy
    pub fn with_security_policy(root: PathBuf, policy: SecurityPolicy) -> Self {
        Self {
            root,
            allow_read_outside: false,
            max_read_size: 100 * 1024 * 1024,
            max_batch_size: 1000,
            security_policy: Some(policy),
        }
    }

    /// Sets whether reading outside sandbox is allowed
    /// Note: If security_policy is set, this will be ignored
    pub fn allow_read_outside(mut self, allow: bool) -> Self {
        self.allow_read_outside = allow;
        self
    }

    /// Sets maximum read size
    /// Note: If security_policy is set, this will be ignored
    pub fn max_read_size(mut self, size: usize) -> Self {
        self.max_read_size = size;
        self
    }

    /// Sets maximum batch size
    /// Note: If security_policy is set, this will be ignored
    pub fn max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Sets the security policy
    pub fn with_policy(mut self, policy: SecurityPolicy) -> Self {
        self.security_policy = Some(policy);
        self
    }

    /// Gets the effective resource limits (from policy or legacy config)
    pub fn effective_max_file_size(&self) -> usize {
        self.security_policy
            .as_ref()
            .map(|p| p.resource_limits.max_file_size)
            .unwrap_or(self.max_read_size)
    }

    /// Gets the effective max batch size
    pub fn effective_max_batch_size(&self) -> usize {
        self.security_policy
            .as_ref()
            .map(|p| p.resource_limits.max_batch_size)
            .unwrap_or(self.max_batch_size)
    }

    /// Gets the effective allow_read_outside setting
    pub fn effective_allow_read_outside(&self) -> bool {
        self.security_policy
            .as_ref()
            .map(|p| p.sandbox_policy.allow_read_outside)
            .unwrap_or(self.allow_read_outside)
    }

    /// Gets the effective allow_write_outside setting
    pub fn effective_allow_write_outside(&self) -> bool {
        self.security_policy
            .as_ref()
            .map(|p| p.sandbox_policy.allow_write_outside)
            .unwrap_or(false) // Legacy mode: never allow write outside
    }
}

/// Options for cat operation
#[derive(Debug, Clone, Default)]
pub struct CatOptions {
    /// Show line numbers
    pub number_lines: bool,
    /// Show non-printing characters
    pub show_ends: bool,
    /// Squeeze multiple blank lines
    pub squeeze_blank: bool,
}

/// Options for ls operation
#[derive(Debug, Clone, Default)]
pub struct LsOptions {
    /// Show hidden files
    pub all: bool,
    /// Long format with details
    pub long: bool,
    /// Human-readable sizes
    pub human_readable: bool,
    /// Recursive listing
    pub recursive: bool,
    /// Sort by modification time
    pub sort_by_time: bool,
    /// Reverse sort order
    pub reverse: bool,
}

/// Options for head operation
#[derive(Debug, Clone)]
pub struct HeadOptions {
    /// Number of lines to display (default: 10)
    pub lines: usize,
    /// Number of bytes to display (if Some, overrides lines)
    pub bytes: Option<usize>,
    /// Show filename headers for multiple files
    pub verbose: bool,
    /// Never show filename headers
    pub quiet: bool,
}

impl Default for HeadOptions {
    fn default() -> Self {
        Self {
            lines: 10,
            bytes: None,
            verbose: false,
            quiet: false,
        }
    }
}

/// Options for tail operation
#[derive(Debug, Clone)]
pub struct TailOptions {
    /// Number of lines to display (default: 10)
    pub lines: usize,
    /// Number of bytes to display (if Some, overrides lines)
    pub bytes: Option<usize>,
    /// Show filename headers for multiple files
    pub verbose: bool,
    /// Never show filename headers
    pub quiet: bool,
}

impl Default for TailOptions {
    fn default() -> Self {
        Self {
            lines: 10,
            bytes: None,
            verbose: false,
            quiet: false,
        }
    }
}

/// Options for wc operation
#[derive(Debug, Clone, Default)]
pub struct WcOptions {
    /// Count lines
    pub lines: bool,
    /// Count words
    pub words: bool,
    /// Count bytes
    pub bytes: bool,
    /// Count characters
    pub chars: bool,
}

/// Result of wc operation
#[derive(Debug, Clone)]
pub struct WcResult {
    /// Line count
    pub lines: usize,
    /// Word count
    pub words: usize,
    /// Byte count
    pub bytes: usize,
    /// Character count
    pub chars: usize,
}

/// Options for grep operation
#[derive(Debug, Clone, Default)]
pub struct GrepOptions {
    /// Case-insensitive search
    pub ignore_case: bool,
    /// Invert match (show non-matching lines)
    pub invert_match: bool,
    /// Show line numbers
    pub line_number: bool,
    /// Show only count of matching lines
    pub count: bool,
    /// Show only filenames with matches
    pub files_with_matches: bool,
    /// Show only filenames without matches
    pub files_without_match: bool,
    /// Number of context lines before match
    pub before_context: usize,
    /// Number of context lines after match
    pub after_context: usize,
    /// Use extended regex
    pub extended_regexp: bool,
    /// Use fixed strings (not regex)
    pub fixed_strings: bool,
    /// Recursive search in directories
    pub recursive: bool,
}

/// Match information from grep
#[derive(Debug, Clone)]
pub struct GrepMatch {
    /// File path
    pub path: String,
    /// Line number (1-indexed)
    pub line_number: usize,
    /// Matched line content
    pub line: String,
    /// Lines before match (for context)
    pub before: Vec<String>,
    /// Lines after match (for context)
    pub after: Vec<String>,
}

/// Options for sort operation
#[derive(Debug, Clone, Default)]
pub struct SortOptions {
    /// Reverse sort order
    pub reverse: bool,
    /// Numeric sort
    pub numeric: bool,
    /// Unique lines only
    pub unique: bool,
    /// Case-insensitive sort
    pub ignore_case: bool,
    /// Version sort (natural sort)
    pub version_sort: bool,
    /// Sort by month name
    pub month_sort: bool,
    /// Human numeric sort (1K, 1M, 1G)
    pub human_numeric: bool,
}

/// Options for uniq operation
#[derive(Debug, Clone, Default)]
pub struct UniqOptions {
    /// Prefix lines with count
    pub count: bool,
    /// Only print duplicate lines
    pub repeated: bool,
    /// Only print unique lines
    pub unique: bool,
    /// Case-insensitive comparison
    pub ignore_case: bool,
    /// Skip N fields before comparing
    pub skip_fields: usize,
    /// Skip N characters before comparing
    pub skip_chars: usize,
}

/// Shell type for command execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    /// PowerShell (Windows default)
    PowerShell,
    /// Command Prompt (Windows)
    Cmd,
    /// Bash (Unix-like)
    Bash,
}

impl Default for ShellType {
    fn default() -> Self {
        #[cfg(windows)]
        {
            Self::PowerShell
        }
        #[cfg(not(windows))]
        {
            Self::Bash
        }
    }
}

impl ShellType {
    /// Returns the executable name for this shell
    pub fn executable(&self) -> &'static str {
        match self {
            Self::PowerShell => "powershell.exe",
            Self::Cmd => "cmd.exe",
            Self::Bash => "bash",
        }
    }

    /// Returns the command argument flag
    pub fn command_flag(&self) -> &'static str {
        match self {
            Self::PowerShell => "-Command",
            Self::Cmd => "/C",
            Self::Bash => "-c",
        }
    }
}

/// Options for shell command execution
#[derive(Debug, Clone)]
pub struct CommandOptions {
    /// Shell type to use
    pub shell: ShellType,
    /// Working directory for command
    pub working_dir: Option<PathBuf>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Timeout in seconds (None for no timeout)
    pub timeout: Option<u64>,
    /// Capture stdout
    pub capture_stdout: bool,
    /// Capture stderr
    pub capture_stderr: bool,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            shell: ShellType::default(),
            working_dir: None,
            env: Vec::new(),
            timeout: Some(30), // 30 second default timeout
            capture_stdout: true,
            capture_stderr: true,
        }
    }
}

/// Result of command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit status code
    pub status: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// File entry information
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// Is directory
    pub is_dir: bool,
    /// File size in bytes
    pub size: u64,
    /// Last modified time (Unix timestamp)
    pub modified: Option<u64>,
    /// Permissions (Unix-style, 0 if not available)
    pub permissions: u32,
}

/// Result of ls operation
#[derive(Debug, Clone)]
pub struct LsResult {
    /// List of file entries
    pub entries: Vec<FileEntry>,
    /// Total number of entries
    pub total: usize,
    /// Total size of all files
    pub total_size: u64,
}

/// Byte Order Mark (BOM) detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bom {
    /// No BOM detected
    None,
    /// UTF-8 BOM
    Utf8,
    /// UTF-16 Little Endian
    Utf16Le,
    /// UTF-16 Big Endian
    Utf16Be,
}

impl Bom {
    /// Detects BOM from byte slice
    pub fn detect(bytes: &[u8]) -> Self {
        if bytes.len() >= 3 && bytes[0..3] == [0xEF, 0xBB, 0xBF] {
            Self::Utf8
        } else if bytes.len() >= 2 && bytes[0..2] == [0xFF, 0xFE] {
            Self::Utf16Le
        } else if bytes.len() >= 2 && bytes[0..2] == [0xFE, 0xFF] {
            Self::Utf16Be
        } else {
            Self::None
        }
    }

    /// Returns the size of the BOM in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Utf8 => 3,
            Self::Utf16Le | Self::Utf16Be => 2,
        }
    }
}

/// Line ending style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// Unix (LF)
    Unix,
    /// Windows (CRLF)
    Windows,
    /// Old Mac (CR)
    Mac,
}

impl LineEnding {
    /// Returns the string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unix => "\n",
            Self::Windows => "\r\n",
            Self::Mac => "\r",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bom_detection() {
        assert_eq!(Bom::detect(&[0xEF, 0xBB, 0xBF, 0x68]), Bom::Utf8);
        assert_eq!(Bom::detect(&[0xFF, 0xFE]), Bom::Utf16Le);
        assert_eq!(Bom::detect(&[0xFE, 0xFF]), Bom::Utf16Be);
        assert_eq!(Bom::detect(&[0x68, 0x65, 0x6C, 0x6C]), Bom::None);
    }

    #[test]
    fn test_bom_size() {
        assert_eq!(Bom::None.size(), 0);
        assert_eq!(Bom::Utf8.size(), 3);
        assert_eq!(Bom::Utf16Le.size(), 2);
    }

    #[test]
    fn test_sandbox_config() {
        let config = SandboxConfig::default()
            .allow_read_outside(true)
            .max_read_size(50 * 1024 * 1024);

        assert!(config.allow_read_outside);
        assert_eq!(config.max_read_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let agent_err: AgentError = io_err.into();
        assert!(matches!(agent_err, AgentError::NotFound(_)));
    }
}
