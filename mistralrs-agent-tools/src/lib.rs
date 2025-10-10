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
pub mod catalog;
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
pub use catalog::{ToolCatalog, ToolDefinition, ToolExample};
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
