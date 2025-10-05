//! Enhanced Windows wrapper for ripgrep with comprehensive path normalization.
//!
//! This wrapper intercepts all path arguments passed to ripgrep and normalizes them
//! to Windows format, supporting various path formats including WSL, Cygwin, UNC,
//! and mixed separators. It then passes through all functionality to the actual
//! ripgrep binary while adding Windows-specific enhancements.

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use winpath::{normalize_path, PathError};

/// The expected location of the actual ripgrep binary
const RG_BINARY_PATH: &str = r"C:\Users\david\.local\bin\rg.exe";

/// Configuration for the ripgrep wrapper
#[derive(Debug, Clone)]
struct RgConfig {
    /// Path to the actual ripgrep binary
    rg_binary: PathBuf,
    /// Whether to enable debug output for argument processing
    debug_args: bool,
    /// Whether to enable path normalization
    normalize_paths: bool,
    /// Whether to enable Windows-specific enhancements
    windows_enhancements: bool,
}

impl Default for RgConfig {
    fn default() -> Self {
        Self {
            rg_binary: PathBuf::from(RG_BINARY_PATH),
            debug_args: cfg!(feature = "debug-args"),
            normalize_paths: cfg!(feature = "path-normalization"),
            windows_enhancements: cfg!(feature = "windows-enhancements"),
        }
    }
}

/// Represents a ripgrep argument that may contain a path
#[derive(Debug, Clone)]
enum RgArgument {
    /// A pattern argument (first positional argument)
    Pattern(String),
    /// A path argument (file or directory to search)
    Path(String),
    /// A flag with a path value (e.g., --file, --ignore-file)
    FlagWithPath { flag: String, path: String },
    /// A regular flag without path content
    Flag(String),
    /// An unknown argument that should be passed through
    Unknown(String),
}

/// Windows-specific file system enhancements
#[derive(Debug, Clone)]
struct WindowsEnhancements {
    /// Whether to automatically handle CRLF/LF detection
    auto_crlf: bool,
    /// Whether to include Windows hidden files
    include_hidden: bool,
    /// Whether to follow junctions and symlinks
    follow_links: bool,
    /// Whether to support UNC paths
    support_unc: bool,
}

impl Default for WindowsEnhancements {
    fn default() -> Self {
        Self {
            auto_crlf: true,
            include_hidden: false,
            follow_links: false,
            support_unc: true,
        }
    }
}

/// Main wrapper application
struct RgWrapper {
    config: RgConfig,
    enhancements: WindowsEnhancements,
    original_args: Vec<String>,
    processed_args: Vec<String>,
}

impl RgWrapper {
    /// Create a new ripgrep wrapper with the given arguments
    fn new(args: Vec<String>) -> Result<Self> {
        let config = RgConfig::default();
        let enhancements = WindowsEnhancements::default();

        // Verify the ripgrep binary exists
        if !config.rg_binary.exists() {
            return Err(anyhow::anyhow!(
                "ripgrep binary not found at: {}. Please ensure ripgrep is installed.",
                config.rg_binary.display()
            ));
        }

        Ok(Self {
            config,
            enhancements,
            original_args: args,
            processed_args: Vec::new(),
        })
    }

    /// Process all arguments, normalizing paths and adding Windows enhancements
    fn process_arguments(&mut self) -> Result<()> {
        let parsed_args = self.parse_arguments()?;

        if self.config.debug_args {
            eprintln!("Debug: Original args: {:?}", self.original_args);
            eprintln!("Debug: Parsed args: {:?}", parsed_args);
        }

        for arg in parsed_args {
            match arg {
                RgArgument::Pattern(pattern) => {
                    self.processed_args.push(pattern);
                }
                RgArgument::Path(path) => {
                    let normalized = self.normalize_path_if_needed(&path)?;
                    self.processed_args.push(normalized);
                }
                RgArgument::FlagWithPath { flag, path } => {
                    let normalized = self.normalize_path_if_needed(&path)?;
                    self.processed_args.push(flag);
                    self.processed_args.push(normalized);
                }
                RgArgument::Flag(flag) => {
                    self.processed_args.push(flag);
                }
                RgArgument::Unknown(arg) => {
                    self.processed_args.push(arg);
                }
            }
        }

        // Add Windows-specific enhancements
        if self.config.windows_enhancements {
            self.add_windows_enhancements();
        }

        if self.config.debug_args {
            eprintln!("Debug: Final processed args: {:?}", self.processed_args);
        }

        Ok(())
    }

    /// Parse arguments into structured format
    fn parse_arguments(&self) -> Result<Vec<RgArgument>> {
        let mut args = Vec::new();
        let mut i = 1; // Skip program name
        let mut positional_count = 0;

        while i < self.original_args.len() {
            let arg = &self.original_args[i];

            if arg.starts_with('-') {
                // Handle flags
                if let Some((path_arg, skip_next)) = self.get_path_for_flag(arg, i)? {
                    args.push(RgArgument::FlagWithPath {
                        flag: arg.clone(),
                        path: path_arg,
                    });
                    if skip_next {
                        i += 1; // Skip the next argument as it was consumed as the path
                    }
                } else {
                    args.push(RgArgument::Flag(arg.clone()));
                }
            } else {
                // Handle positional arguments
                if positional_count == 0 {
                    // First positional is usually the pattern
                    args.push(RgArgument::Pattern(arg.clone()));
                } else {
                    // Subsequent positionals are paths
                    args.push(RgArgument::Path(arg.clone()));
                }
                positional_count += 1;
            }
            i += 1;
        }

        Ok(args)
    }

    /// Get path argument for flags that take path values
    /// Returns (path_value, should_skip_next_arg)
    fn get_path_for_flag(
        &self,
        flag: &str,
        current_index: usize,
    ) -> Result<Option<(String, bool)>> {
        // Flags that take path arguments
        let path_flags = [
            "--file", "-f",
            "--ignore-file",
            "--path-separator",
            "--pre-glob",
        ];

        // Flags with inline values (--file=path)
        if let Some(eq_pos) = flag.find('=') {
            let flag_name = &flag[..eq_pos];
            if path_flags.contains(&flag_name) {
                return Ok(Some((flag[eq_pos + 1..].to_string(), false)));
            }
        }

        // Flags with separate value
        if path_flags.contains(&flag) {
            if current_index + 1 < self.original_args.len() {
                let next_arg = &self.original_args[current_index + 1];
                return Ok(Some((next_arg.clone(), true)));
            }
        }

        Ok(None)
    }

    /// Normalize a path using the winpath library if path normalization is enabled
    fn normalize_path_if_needed(&self, path: &str) -> Result<String> {
        if !self.config.normalize_paths {
            return Ok(path.to_string());
        }

        // Check if this looks like a path (not a pattern or other argument)
        if !self.looks_like_path(path) {
            return Ok(path.to_string());
        }

        match normalize_path(path) {
            Ok(normalized) => {
                if self.config.debug_args {
                    eprintln!("Debug: Normalized '{}' -> '{}'", path, normalized);
                }
                Ok(normalized)
            }
            Err(PathError::UnsupportedFormat) => {
                // If normalization fails due to unsupported format, use original
                if self.config.debug_args {
                    eprintln!("Debug: Path '{}' not normalized (unsupported format)", path);
                }
                Ok(path.to_string())
            }
            Err(e) => {
                if self.config.debug_args {
                    eprintln!("Debug: Path normalization error for '{}': {}", path, e);
                }
                // On error, use original path
                Ok(path.to_string())
            }
        }
    }

    /// Check if a string looks like a file path
    fn looks_like_path(&self, s: &str) -> bool {
        // Skip empty strings
        if s.is_empty() {
            return false;
        }

        // Skip obvious patterns (regex-like strings)
        if s.contains('[') || s.contains(']') || s.contains('*') || s.contains('?') {
            // But allow glob patterns that look like paths
            if s.contains('/') || s.contains('\\') {
                return true;
            }
            return false;
        }

        // Check for path-like characteristics
        s.contains('/') ||
        s.contains('\\') ||
        s.starts_with('.') ||
        s.contains(':') ||
        (s.len() > 2 && Path::new(s).extension().is_some())
    }

    /// Add Windows-specific enhancements to the argument list
    fn add_windows_enhancements(&mut self) {
        if self.enhancements.auto_crlf {
            // Add CRLF handling - ripgrep handles this automatically, but we can be explicit
            if !self.processed_args.iter().any(|arg| arg.starts_with("--crlf")) {
                self.processed_args.push("--crlf".to_string());
            }
        }

        if self.enhancements.include_hidden {
            // Include hidden files on Windows
            if !self.processed_args.iter().any(|arg| arg == "--hidden") {
                self.processed_args.push("--hidden".to_string());
            }
        }

        if self.enhancements.follow_links {
            // Follow symbolic links and junctions
            if !self.processed_args.iter().any(|arg| arg == "--follow") {
                self.processed_args.push("--follow".to_string());
            }
        }
    }

    /// Execute the actual ripgrep binary with processed arguments
    fn execute(&self) -> Result<i32> {
        let mut cmd = ProcessCommand::new(&self.config.rg_binary);
        cmd.args(&self.processed_args);

        // Preserve stdio settings
        cmd.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

        if self.config.debug_args {
            eprintln!("Debug: Executing: {} {:?}", self.config.rg_binary.display(), self.processed_args);
        }

        let status = cmd.status()
            .with_context(|| format!("Failed to execute ripgrep at {}", self.config.rg_binary.display()))?;

        Ok(status.code().unwrap_or(1))
    }
}

/// Set up command line interface (mainly for help and version info)
fn build_cli() -> Command {
    Command::new("rg")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Enhanced Windows wrapper for ripgrep with comprehensive path normalization")
        .long_about(
            "This is a Windows-enhanced wrapper for ripgrep that provides:\n\
             • Automatic path normalization (WSL, Cygwin, UNC, mixed separators)\n\
             • Windows-specific file handling enhancements\n\
             • Transparent pass-through of all ripgrep functionality\n\n\
             All ripgrep arguments and options are supported. This wrapper only\n\
             enhances path handling and adds Windows-specific optimizations."
        )
        .arg(
            Arg::new("debug-wrapper")
                .long("debug-wrapper")
                .action(ArgAction::SetTrue)
                .help("Enable debug output for the wrapper (shows path normalization)")
                .hide(true)
        )
        .disable_help_flag(true)
        .disable_version_flag(true)
        .allow_external_subcommands(true)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Handle special wrapper-specific flags
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("rg-wrapper {}", env!("CARGO_PKG_VERSION"));
        println!("Enhanced Windows wrapper for ripgrep");
        println!();

        // Also show the underlying ripgrep version if possible
        match ProcessCommand::new(RG_BINARY_PATH).arg("--version").output() {
            Ok(output) => {
                if output.status.success() {
                    println!("Underlying ripgrep:");
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
            Err(_) => {
                println!("Underlying ripgrep binary not found at: {}", RG_BINARY_PATH);
            }
        }
        return Ok(());
    }

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        let mut app = build_cli();
        app.print_help()?;
        println!();
        println!();
        println!("This wrapper enhances ripgrep with Windows path normalization.");
        println!("All ripgrep options and arguments are supported.");
        println!();

        // Show underlying ripgrep help too
        match ProcessCommand::new(RG_BINARY_PATH).arg("--help").output() {
            Ok(output) => {
                if output.status.success() {
                    println!("--- Underlying ripgrep help ---");
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
            Err(_) => {
                println!("Cannot access underlying ripgrep binary at: {}", RG_BINARY_PATH);
            }
        }
        return Ok(());
    }

    // Create and run the wrapper
    let mut wrapper = RgWrapper::new(args)?;
    wrapper.process_arguments()?;
    let exit_code = wrapper.execute()?;

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_looks_like_path() {
        let wrapper = RgWrapper::new(vec!["rg".to_string()]).unwrap();

        // Should be recognized as paths
        assert!(wrapper.looks_like_path("file.txt"));
        assert!(wrapper.looks_like_path("./file.txt"));
        assert!(wrapper.looks_like_path("../file.txt"));
        assert!(wrapper.looks_like_path("/path/to/file"));
        assert!(wrapper.looks_like_path("C:\\path\\to\\file"));
        assert!(wrapper.looks_like_path("/mnt/c/users"));
        assert!(wrapper.looks_like_path("src/*.rs")); // glob pattern

        // Should NOT be recognized as paths
        assert!(!wrapper.looks_like_path("pattern"));
        assert!(!wrapper.looks_like_path("regex[a-z]+"));
        assert!(!wrapper.looks_like_path("--flag"));
        assert!(!wrapper.looks_like_path(""));
    }

    #[test]
    fn test_parse_arguments() {
        let args = vec![
            "rg".to_string(),
            "pattern".to_string(),
            "file.txt".to_string(),
            "--hidden".to_string(),
            "--file".to_string(),
            "ignore.txt".to_string(),
        ];

        let wrapper = RgWrapper::new(args).unwrap();
        let parsed = wrapper.parse_arguments().unwrap();

        assert_eq!(parsed.len(), 4);

        match &parsed[0] {
            RgArgument::Pattern(p) => assert_eq!(p, "pattern"),
            _ => panic!("Expected pattern argument"),
        }

        match &parsed[1] {
            RgArgument::Path(p) => assert_eq!(p, "file.txt"),
            _ => panic!("Expected path argument"),
        }

        match &parsed[2] {
            RgArgument::Flag(f) => assert_eq!(f, "--hidden"),
            _ => panic!("Expected flag argument"),
        }

        match &parsed[3] {
            RgArgument::FlagWithPath { flag, path } => {
                assert_eq!(flag, "--file");
                assert_eq!(path, "ignore.txt");
            }
            _ => panic!("Expected flag with path argument"),
        }
    }

    #[test]
    fn test_path_normalization() {
        let mut wrapper = RgWrapper::new(vec!["rg".to_string()]).unwrap();
        wrapper.config.normalize_paths = true;

        // Test WSL path normalization
        let normalized = wrapper.normalize_path_if_needed("/mnt/c/users/test").unwrap();
        assert_eq!(normalized, r"C:\users\test");

        // Test that non-paths are left alone
        let not_normalized = wrapper.normalize_path_if_needed("pattern[a-z]+").unwrap();
        assert_eq!(not_normalized, "pattern[a-z]+");

        // Test already normalized Windows path
        let unchanged = wrapper.normalize_path_if_needed(r"C:\Users\test").unwrap();
        assert_eq!(unchanged, r"C:\Users\test");
    }

    #[test]
    fn test_flag_with_path_parsing() {
        let args = vec![
            "rg".to_string(),
            "pattern".to_string(),
            "--file=ignore.txt".to_string(),
        ];

        let wrapper = RgWrapper::new(args).unwrap();
        let flag = &wrapper.original_args[2]; // "--file=ignore.txt"
        let path_arg = wrapper.get_path_for_flag(flag, 2).unwrap();

        assert_eq!(path_arg, Some(("ignore.txt".to_string(), false)));
    }

    #[test]
    fn test_windows_enhancements() {
        let mut wrapper = RgWrapper::new(vec!["rg".to_string(), "pattern".to_string()]).unwrap();
        wrapper.config.windows_enhancements = true;
        wrapper.enhancements.auto_crlf = true;
        wrapper.enhancements.include_hidden = true;
        wrapper.enhancements.follow_links = true;

        wrapper.process_arguments().unwrap();

        // Should contain Windows-specific flags
        assert!(wrapper.processed_args.contains(&"--crlf".to_string()));
        assert!(wrapper.processed_args.contains(&"--hidden".to_string()));
        assert!(wrapper.processed_args.contains(&"--follow".to_string()));
    }
}
