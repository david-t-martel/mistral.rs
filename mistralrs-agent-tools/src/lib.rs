//! Agent Tools - Comprehensive filesystem and system utilities for LLM agents
//!
//! This crate provides a type-safe, sandboxed toolkit for filesystem operations,
//! text processing, system information, and more.
//!
//! # Features
//!
//! - **90+ Unix utilities**: Full coreutils implementation
//! - **Shell execution**: PowerShell, cmd, bash support (coming soon)
//! - **Sandbox enforcement**: All operations restricted to configured boundaries
//! - **Path normalization**: Windows/WSL/Cygwin/Git Bash compatibility
//! - **Type-safe API**: Rich types with comprehensive error handling

// Module declarations
pub mod pathlib;
pub mod tools;
pub mod types;

// Core integration with mistralrs-core
pub mod core_integration;

// MCP server for exposing tools via Model Context Protocol
#[cfg(feature = "sandbox")]
pub mod mcp_server;

// Test utilities (only available in tests)
#[cfg(test)]
pub mod test_utils;

// Re-exports for convenience
use std::path::PathBuf;
pub use tools::file::{cat, ls};
pub use tools::sandbox::Sandbox;
pub use tools::shell::execute;
pub use tools::text::{grep, head, sort, tail, uniq, wc};
pub use types::{
    AgentError, AgentResult, Bom, CatOptions, CommandOptions, CommandResult, FileEntry, GrepMatch,
    GrepOptions, HeadOptions, LineEnding, LsOptions, LsResult, SandboxConfig, SecurityLevel,
    SecurityPolicy, ShellType, SortOptions, TailOptions, UniqOptions, WcOptions, WcResult,
};

// Core integration exports
pub use core_integration::AgentToolProvider;

// MCP server exports
#[cfg(feature = "sandbox")]
pub use mcp_server::McpServer;

/// Main agent toolkit providing high-level API for all operations
#[derive(Debug, Clone)]
pub struct AgentToolkit {
    sandbox: Sandbox,
}

impl AgentToolkit {
    /// Create a new toolkit with the given sandbox configuration
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            sandbox: Sandbox::new(config),
        }
    }

    /// Create toolkit with default configuration (current directory)
    pub fn with_defaults() -> Self {
        Self::new(SandboxConfig::default())
    }

    /// Create toolkit with a specific sandbox root
    pub fn with_root(root: PathBuf) -> Self {
        Self::new(SandboxConfig::new(root))
    }

    /// Get reference to the sandbox
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    // File operations

    /// Concatenate and display files
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, CatOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = CatOptions {
    ///     number_lines: true,
    ///     ..Default::default()
    /// };
    /// let content = toolkit.cat(&[Path::new("file.txt")], &options).unwrap();
    /// println!("{}", content);
    /// ```
    pub fn cat(&self, paths: &[&std::path::Path], options: &CatOptions) -> AgentResult<String> {
        tools::file::cat(&self.sandbox, paths, options)
    }

    /// List directory contents
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, LsOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = LsOptions {
    ///     all: true,
    ///     long: true,
    ///     ..Default::default()
    /// };
    /// let result = toolkit.ls(Path::new("."), &options).unwrap();
    /// for entry in result.entries {
    ///     println!("{}", entry.name);
    /// }
    /// ```
    pub fn ls(&self, path: &std::path::Path, options: &LsOptions) -> AgentResult<LsResult> {
        tools::file::ls(&self.sandbox, path, options)
    }

    // Text processing operations

    /// Display first part of files
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, HeadOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = HeadOptions {
    ///     lines: 20,
    ///     ..Default::default()
    /// };
    /// let content = toolkit.head(&[Path::new("file.txt")], &options).unwrap();
    /// println!("{}", content);
    /// ```
    pub fn head(&self, paths: &[&std::path::Path], options: &HeadOptions) -> AgentResult<String> {
        tools::text::head(&self.sandbox, paths, options)
    }

    /// Display last part of files
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, TailOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = TailOptions {
    ///     lines: 20,
    ///     ..Default::default()
    /// };
    /// let content = toolkit.tail(&[Path::new("file.txt")], &options).unwrap();
    /// println!("{}", content);
    /// ```
    pub fn tail(&self, paths: &[&std::path::Path], options: &TailOptions) -> AgentResult<String> {
        tools::text::tail(&self.sandbox, paths, options)
    }

    /// Count lines, words, bytes, and/or characters
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, WcOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = WcOptions {
    ///     lines: true,
    ///     words: true,
    ///     ..Default::default()
    /// };
    /// let results = toolkit.wc(&[Path::new("file.txt")], &options).unwrap();
    /// for (path, result) in results {
    ///     println!("{}: {} lines, {} words", path, result.lines, result.words);
    /// }
    /// ```
    pub fn wc(
        &self,
        paths: &[&std::path::Path],
        options: &WcOptions,
    ) -> AgentResult<Vec<(String, WcResult)>> {
        tools::text::wc(&self.sandbox, paths, options)
    }

    /// Search for patterns in files
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, GrepOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = GrepOptions {
    ///     ignore_case: true,
    ///     line_number: true,
    ///     ..Default::default()
    /// };
    /// let matches = toolkit.grep("pattern", &[Path::new("file.txt")], &options).unwrap();
    /// for m in matches {
    ///     println!("{}:{}: {}", m.path, m.line_number, m.line);
    /// }
    /// ```
    pub fn grep(
        &self,
        pattern: &str,
        paths: &[&std::path::Path],
        options: &GrepOptions,
    ) -> AgentResult<Vec<GrepMatch>> {
        tools::text::grep(&self.sandbox, pattern, paths, options)
    }

    /// Sort lines from files
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, SortOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = SortOptions {
    ///     numeric: true,
    ///     reverse: true,
    ///     ..Default::default()
    /// };
    /// let sorted = toolkit.sort(&[Path::new("file.txt")], &options).unwrap();
    /// println!("{}", sorted);
    /// ```
    pub fn sort(&self, paths: &[&std::path::Path], options: &SortOptions) -> AgentResult<String> {
        tools::text::sort(&self.sandbox, paths, options)
    }

    /// Filter adjacent duplicate lines
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, UniqOptions};
    /// use std::path::Path;
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = UniqOptions {
    ///     count: true,
    ///     ..Default::default()
    /// };
    /// let unique = toolkit.uniq(&[Path::new("file.txt")], &options).unwrap();
    /// println!("{}", unique);
    /// ```
    pub fn uniq(&self, paths: &[&std::path::Path], options: &UniqOptions) -> AgentResult<String> {
        tools::text::uniq(&self.sandbox, paths, options)
    }

    // Shell execution operations

    /// Execute a shell command
    ///
    /// # Example
    /// ```no_run
    /// use mistralrs_agent_tools::{AgentToolkit, CommandOptions, ShellType};
    ///
    /// let toolkit = AgentToolkit::with_defaults();
    /// let options = CommandOptions {
    ///     shell: ShellType::PowerShell,
    ///     timeout: Some(10),
    ///     ..Default::default()
    /// };
    /// let result = toolkit.execute("Get-Process", &options).unwrap();
    /// println!("Status: {}", result.status);
    /// println!("Output:\n{}", result.stdout);
    /// ```
    pub fn execute(&self, command: &str, options: &CommandOptions) -> AgentResult<CommandResult> {
        tools::shell::execute(&self.sandbox, command, options)
    }

    // Winutils text processing operations

    /// Cut - extract fields from lines
    pub fn cut(
        &self,
        paths: &[&std::path::Path],
        fields: &str,
        delimiter: Option<char>,
    ) -> AgentResult<String> {
        tools::winutils::text::cut(&self.sandbox, paths, fields, delimiter)
    }

    /// Translate or delete characters
    pub fn tr(
        &self,
        paths: &[&std::path::Path],
        from_chars: &str,
        to_chars: Option<&str>,
    ) -> AgentResult<String> {
        tools::winutils::text::tr(&self.sandbox, paths, from_chars, to_chars)
    }

    /// Expand tabs to spaces
    pub fn expand(
        &self,
        paths: &[&std::path::Path],
        tab_stops: Option<usize>,
    ) -> AgentResult<String> {
        tools::winutils::text::expand(&self.sandbox, paths, tab_stops)
    }

    /// Reverse lines (tac)
    pub fn tac(&self, paths: &[&std::path::Path]) -> AgentResult<String> {
        tools::winutils::text::tac(&self.sandbox, paths)
    }

    /// Number lines
    pub fn nl(&self, paths: &[&std::path::Path], start: Option<usize>) -> AgentResult<String> {
        tools::winutils::text::nl(&self.sandbox, paths, start)
    }

    // Winutils encoding operations

    /// Base64 encode/decode
    pub fn base64_util(&self, path: &std::path::Path, decode: bool) -> AgentResult<String> {
        tools::winutils::encoding::base64(&self.sandbox, path, decode)
    }

    /// Base32 encode/decode
    pub fn base32_util(&self, path: &std::path::Path, decode: bool) -> AgentResult<String> {
        tools::winutils::encoding::base32(&self.sandbox, path, decode)
    }

    // Winutils file operations

    /// Copy files with winutils
    pub fn cp_util(
        &self,
        source: &std::path::Path,
        dest: &std::path::Path,
        recursive: bool,
    ) -> AgentResult<()> {
        tools::winutils::fileops::cp(&self.sandbox, source, dest, recursive)
    }

    /// Move files with winutils
    pub fn mv_util(&self, source: &std::path::Path, dest: &std::path::Path) -> AgentResult<()> {
        tools::winutils::fileops::mv(&self.sandbox, source, dest)
    }

    /// Remove files with winutils
    pub fn rm_util(&self, path: &std::path::Path, recursive: bool, force: bool) -> AgentResult<()> {
        tools::winutils::fileops::rm(&self.sandbox, path, recursive, force)
    }

    /// Make directory with winutils
    pub fn mkdir_util(&self, path: &std::path::Path, parents: bool) -> AgentResult<()> {
        tools::winutils::fileops::mkdir(&self.sandbox, path, parents)
    }

    /// Touch file with winutils
    pub fn touch_util(&self, path: &std::path::Path) -> AgentResult<()> {
        tools::winutils::fileops::touch(&self.sandbox, path)
    }
}

// Keep legacy AgentTools API for backwards compatibility
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use tracing::{info, warn};
use walkdir::WalkDir;

/// Maximum file size for read operations (5 MiB)
const MAX_READ_SIZE: u64 = 5 * 1024 * 1024;

/// Maximum number of results for find/tree operations
const MAX_RESULTS: usize = 1000;

/// Legacy sandbox configuration (deprecated, use SandboxConfig instead)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacySandboxConfig {
    pub root: Utf8PathBuf,
    pub readonly_paths: Vec<Utf8PathBuf>,
    pub enforce: bool,
}

impl Default for LegacySandboxConfig {
    fn default() -> Self {
        let root = std::env::var("MISTRALRS_AGENT_SANDBOX_ROOT")
            .ok()
            .and_then(|p| Utf8PathBuf::from_path_buf(PathBuf::from(p)).ok())
            .unwrap_or_else(|| {
                Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap_or_default())
                    .unwrap_or_else(|_| Utf8PathBuf::from("."))
            });

        let root = root.canonicalize_utf8().unwrap_or(root);

        Self {
            root,
            readonly_paths: vec![
                Utf8PathBuf::from(".git"),
                Utf8PathBuf::from("target"),
                Utf8PathBuf::from("node_modules"),
            ],
            enforce: true,
        }
    }
}

/// Legacy filesystem errors
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Path '{0}' is outside the sandbox root")]
    OutsideSandbox(String),
    #[error("Path '{0}' is read-only")]
    ReadOnly(String),
    #[error("File too large: {0} bytes (max {1})")]
    FileTooLarge(u64, u64),
    #[error("Too many results: {0} (max {1})")]
    TooManyResults(usize, usize),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Legacy result type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsResult {
    pub success: bool,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

impl FsResult {
    pub fn success(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: true,
            path: path.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(
        path: impl Into<String>,
        message: impl Into<String>,
        data: String,
    ) -> Self {
        Self {
            success: true,
            path: path.into(),
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(path: impl Into<String>, error: FsError) -> Self {
        Self {
            success: false,
            path: path.into(),
            message: error.to_string(),
            data: None,
        }
    }
}

/// Legacy AgentTools (deprecated, use AgentToolkit instead)
///
/// # Deprecation Notice
///
/// This type is deprecated and will be removed in a future version.
/// Use `AgentToolkit` instead, which provides:
/// - 90+ Unix-like utilities (vs. 7 in AgentTools)
/// - Type-safe API with rich types
/// - Better sandbox enforcement
/// - Path normalization for Windows/WSL/Cygwin/Git Bash
/// - Integration with mistralrs-core tool callbacks
///
/// # Migration Guide
///
/// Old code:
/// ```no_run
/// use mistralrs_agent_tools::AgentTools;
/// let tools = AgentTools::with_defaults();
/// let result = tools.read("file.txt");
/// ```
///
/// New code:
/// ```no_run
/// use mistralrs_agent_tools::{AgentToolkit, CatOptions};
/// use std::path::Path;
/// let toolkit = AgentToolkit::with_defaults();
/// let content = toolkit.cat(&[Path::new("file.txt")], &CatOptions::default()).unwrap();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use AgentToolkit instead. AgentToolkit provides 90+ tools, type-safe API, and better sandbox enforcement."
)]
pub struct AgentTools {
    config: LegacySandboxConfig,
}

impl AgentTools {
    #[deprecated(since = "0.2.0", note = "Use AgentToolkit::new() instead")]
    pub fn new(config: LegacySandboxConfig) -> Self {
        info!(
            "Initializing agent filesystem tools with sandbox root: {}",
            config.root
        );
        Self { config }
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit::with_defaults() instead")]
    pub fn with_defaults() -> Self {
        Self::new(LegacySandboxConfig::default())
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit::config() instead")]
    pub fn config(&self) -> &LegacySandboxConfig {
        &self.config
    }

    fn validate_path(&self, path: &str) -> Result<Utf8PathBuf, FsError> {
        let path = Utf8Path::new(path);
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.root.join(path)
        };

        let normalized = if absolute.exists() {
            absolute
                .canonicalize_utf8()
                .map_err(|e| FsError::InvalidPath(format!("{}: {}", path, e)))?
        } else if let Some(parent) = absolute.parent() {
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize_utf8()
                    .map_err(|e| FsError::InvalidPath(format!("{}: {}", path, e)))?;
                if let Some(filename) = absolute.file_name() {
                    canonical_parent.join(filename)
                } else {
                    absolute
                }
            } else {
                absolute
            }
        } else {
            absolute
        };

        if self.config.enforce && !normalized.starts_with(&self.config.root) {
            warn!("Path traversal attempt blocked: {}", normalized);
            return Err(FsError::OutsideSandbox(normalized.to_string()));
        }

        Ok(normalized)
    }

    fn is_readonly(&self, path: &Utf8Path) -> bool {
        self.config
            .readonly_paths
            .iter()
            .any(|ro| path.components().any(|comp| comp.as_str() == ro.as_str()))
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit::cat() instead")]
    pub fn read(&self, path: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;
        info!("Reading file: {}", validated_path);

        let metadata = fs::metadata(&validated_path)?;
        if metadata.len() > MAX_READ_SIZE {
            return Err(FsError::FileTooLarge(metadata.len(), MAX_READ_SIZE));
        }

        let contents = fs::read_to_string(&validated_path)?;
        Ok(FsResult::success_with_data(
            validated_path.as_str(),
            format!("Read {} bytes", contents.len()),
            contents,
        ))
    }

    #[deprecated(
        since = "0.2.0",
        note = "Use AgentToolkit methods for file writing operations"
    )]
    pub fn write(
        &self,
        path: &str,
        content: &str,
        create: bool,
        overwrite: bool,
    ) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!(
            "Writing file: {} (create={}, overwrite={})",
            validated_path, create, overwrite
        );

        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if validated_path.exists() && !overwrite {
            return Err(FsError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "File exists and overwrite=false",
            )));
        }

        fs::write(&validated_path, content)?;
        Ok(FsResult::success(
            validated_path.as_str(),
            format!("Wrote {} bytes", content.len()),
        ))
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit methods for file operations")]
    pub fn append(&self, path: &str, content: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Appending to file: {}", validated_path);

        use std::fs::OpenOptions;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&validated_path)?;

        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        Ok(FsResult::success(
            validated_path.as_str(),
            format!("Appended {} bytes", content.len()),
        ))
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit methods for file operations")]
    pub fn delete(&self, path: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Deleting file: {}", validated_path);
        fs::remove_file(&validated_path)?;
        Ok(FsResult::success(validated_path.as_str(), "Deleted"))
    }

    #[deprecated(since = "0.2.0", note = "Use AgentToolkit methods for path checks")]
    pub fn exists(&self, path: &str) -> Result<bool, FsError> {
        let validated_path = self.validate_path(path)?;
        Ok(validated_path.exists())
    }

    #[deprecated(
        since = "0.2.0",
        note = "Use AgentToolkit methods for file search operations"
    )]
    pub fn find(&self, pattern: &str, max_depth: Option<usize>) -> Result<Vec<String>, FsError> {
        info!("Finding files with pattern: {}", pattern);

        let mut results = Vec::new();
        let walker = if let Some(depth) = max_depth {
            WalkDir::new(&self.config.root).max_depth(depth)
        } else {
            WalkDir::new(&self.config.root)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if results.len() >= MAX_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_RESULTS));
            }

            let path = entry.path();
            if let Some(path_str) = path.to_str() {
                if path_str.contains(pattern) {
                    results.push(path_str.to_string());
                }
            }
        }

        Ok(results)
    }

    #[deprecated(
        since = "0.2.0",
        note = "Use AgentToolkit methods for directory traversal"
    )]
    pub fn tree(
        &self,
        root: Option<String>,
        max_depth: Option<usize>,
    ) -> Result<Vec<String>, FsError> {
        let start_path = if let Some(r) = root {
            self.validate_path(&r)?
        } else {
            self.config.root.clone()
        };

        info!("Listing tree from: {}", start_path);

        let mut results = Vec::new();
        let walker = if let Some(depth) = max_depth {
            WalkDir::new(&start_path).max_depth(depth)
        } else {
            WalkDir::new(&start_path)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if results.len() >= MAX_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_RESULTS));
            }

            if let Some(path_str) = entry.path().to_str() {
                results.push(path_str.to_string());
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_agent_toolkit_cat() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let toolkit = AgentToolkit::with_root(temp_dir.path().to_path_buf());
        let options = CatOptions::default();
        let content = toolkit.cat(&[&file_path], &options).unwrap();

        assert!(content.contains("Hello, World!"));
    }

    #[test]
    fn test_agent_toolkit_ls() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::File::create(temp_dir.path().join("file1.txt")).unwrap();
        std::fs::File::create(temp_dir.path().join("file2.txt")).unwrap();

        let toolkit = AgentToolkit::with_root(temp_dir.path().to_path_buf());
        let options = LsOptions::default();
        let result = toolkit.ls(temp_dir.path(), &options).unwrap();

        assert_eq!(result.total, 2);
    }
}
