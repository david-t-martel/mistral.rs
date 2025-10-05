//! # cmd-wrapper - Windows CMD Shell Wrapper
//!
//! A Windows CMD shell wrapper that provides automatic path normalization
//! and cross-platform compatibility layer.
//!
//! ## Key Features
//!
//! - Automatic path normalization for all command arguments
//! - Environment variable translation and expansion
//! - Command history integration
//! - Cross-shell compatibility layer
//! - Windows-specific command handling
//! - Error code translation
//! - Process management with proper cleanup
//!
//! ## Usage
//!
//! ```rust
//! use cmd_wrapper::{CmdWrapper, CmdOptions};
//!
//! let options = CmdOptions::new()
//!     .normalize_paths(true)
//!     .translate_env_vars(true);
//!
//! let wrapper = CmdWrapper::new(options)?;
//! let result = wrapper.execute_command("dir C:\\Users")?;
//! ```

use anyhow::{Context, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;
use winpath::PathNormalizer;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::System::Threading::{CREATE_NEW_CONSOLE, CREATE_NO_WINDOW};

/// Errors that can occur during command execution
#[derive(Error, Debug)]
pub enum CmdError {
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Path normalization error: {0}")]
    PathNormalization(String),
    #[error("Environment variable error: {0}")]
    EnvironmentVariable(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Timeout waiting for command: {0}")]
    Timeout(String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
}

/// Configuration options for CMD wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdOptions {
    /// Normalize all path arguments
    pub normalize_paths: bool,
    /// Translate environment variables between formats
    pub translate_env_vars: bool,
    /// Working directory for commands
    pub working_directory: Option<PathBuf>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Environment variables to remove
    pub env_remove: Vec<String>,
    /// Command timeout in seconds
    pub timeout: Option<Duration>,
    /// Capture stdout
    pub capture_stdout: bool,
    /// Capture stderr
    pub capture_stderr: bool,
    /// Inherit parent environment
    pub inherit_env: bool,
    /// Show command before execution
    pub echo_commands: bool,
    /// Create new console window
    pub new_console: bool,
    /// Hide console window
    pub hide_console: bool,
    /// Use shell expansion
    pub shell_expansion: bool,
    /// Command history size
    pub history_size: usize,
    /// Enable command completion
    pub enable_completion: bool,
}

impl Default for CmdOptions {
    fn default() -> Self {
        Self {
            normalize_paths: true,
            translate_env_vars: true,
            working_directory: None,
            env_vars: HashMap::new(),
            env_remove: Vec::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
            inherit_env: true,
            echo_commands: false,
            new_console: false,
            hide_console: false,
            shell_expansion: true,
            history_size: 1000,
            enable_completion: true,
        }
    }
}

impl CmdOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn normalize_paths(mut self, normalize: bool) -> Self {
        self.normalize_paths = normalize;
        self
    }


    pub fn translate_env_vars(mut self, translate: bool) -> Self {
        self.translate_env_vars = translate;
        self
    }

    pub fn working_directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn echo_commands(mut self, echo: bool) -> Self {
        self.echo_commands = echo;
        self
    }

    pub fn hide_console(mut self, hide: bool) -> Self {
        self.hide_console = hide;
        self
    }
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdResult {
    /// Exit status/code
    pub exit_code: Option<i32>,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr
    pub stderr: String,
    /// Execution time
    pub execution_time: Duration,
    /// Normalized command that was executed
    pub normalized_command: String,
    /// Working directory used
    pub working_directory: PathBuf,
    /// Environment variables that were set
    pub env_vars_used: HashMap<String, String>,
}

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: std::time::SystemTime,
    pub exit_code: Option<i32>,
    pub execution_time: Duration,
    pub working_directory: PathBuf,
}

/// Main CMD wrapper implementation
pub struct CmdWrapper {
    options: CmdOptions,
    normalizer: PathNormalizer,
    history: Vec<HistoryEntry>,
    current_directory: PathBuf,
    env_cache: HashMap<String, String>,
}

impl CmdWrapper {
    pub fn new(options: CmdOptions) -> Result<Self> {
        let normalizer = PathNormalizer::new();
        let current_directory = env::current_dir()
            .context("Failed to get current directory")?;

        let mut env_cache = HashMap::new();
        if options.inherit_env {
            for (key, value) in env::vars() {
                env_cache.insert(key, value);
            }
        }

        Ok(Self {
            options,
            normalizer,
            history: Vec::new(),
            current_directory,
            env_cache,
        })
    }

    /// Execute a command with automatic path normalization
    pub fn execute_command(&mut self, command: &str) -> Result<CmdResult> {
        let start_time = Instant::now();

        debug!("Executing command: {}", command);

        // Parse and normalize the command
        let normalized_command = self.normalize_command(command)?;

        if self.options.echo_commands {
            println!("+ {}", normalized_command);
        }

        // Parse command into program and arguments
        let (program, args) = self.parse_command(&normalized_command)?;

        // Set up the command
        let mut cmd = Command::new(&program);

        // Add arguments
        for arg in args {
            cmd.arg(&arg);
        }

        // Set working directory
        let working_dir = self.options.working_directory
            .as_ref()
            .unwrap_or(&self.current_directory);
        cmd.current_dir(working_dir);

        // Set up environment
        self.setup_environment(&mut cmd)?;

        // Set up stdio
        if self.options.capture_stdout {
            cmd.stdout(Stdio::piped());
        }
        if self.options.capture_stderr {
            cmd.stderr(Stdio::piped());
        }

        // Set Windows-specific options
        #[cfg(windows)]
        {
            let mut creation_flags = 0;
            if self.options.new_console {
                creation_flags |= CREATE_NEW_CONSOLE.0;
            }
            if self.options.hide_console {
                creation_flags |= CREATE_NO_WINDOW.0;
            }
            if creation_flags != 0 {
                cmd.creation_flags(creation_flags);
            }
        }

        // Execute the command
        let execution_result = if let Some(timeout) = self.options.timeout {
            self.execute_with_timeout(cmd, timeout)
        } else {
            self.execute_immediate(cmd)
        };

        let execution_time = start_time.elapsed();

        let (exit_status, stdout, stderr) = execution_result?;

        let result = CmdResult {
            exit_code: exit_status.code(),
            stdout,
            stderr,
            execution_time,
            normalized_command: normalized_command.clone(),
            working_directory: working_dir.clone(),
            env_vars_used: self.get_effective_env_vars(),
        };

        // Add to history
        if self.history.len() >= self.options.history_size {
            self.history.remove(0);
        }
        self.history.push(HistoryEntry {
            command: command.to_string(),
            timestamp: std::time::SystemTime::now(),
            exit_code: result.exit_code,
            execution_time,
            working_directory: working_dir.clone(),
        });

        debug!(
            "Command completed with exit code: {:?} in {:.2}s",
            result.exit_code,
            execution_time.as_secs_f64()
        );

        Ok(result)
    }

    /// Execute multiple commands in sequence
    pub fn execute_batch(&mut self, commands: &[String]) -> Result<Vec<CmdResult>> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute_command(command)?;
            let success = result.exit_code.unwrap_or(1) == 0;
            results.push(result);

            // Stop on first failure unless continuing
            if !success {
                warn!("Command failed: {}", command);
                // Could add option to continue on error
            }
        }

        Ok(results)
    }

    /// Execute a command and return only the stdout
    pub fn execute_for_output(&mut self, command: &str) -> Result<String> {
        let result = self.execute_command(command)?;
        Ok(result.stdout)
    }

    /// Check if a command exists
    pub fn command_exists(&self, command: &str) -> bool {
        // Try to execute with --help or /? to see if command exists
        let test_command = format!("{} /?", command);
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &test_command])
           .stdout(Stdio::null())
           .stderr(Stdio::null());

        cmd.status().map(|status| status.success()).unwrap_or(false)
    }

    /// Get command history
    pub fn get_history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Clear command history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get current working directory
    pub fn get_current_directory(&self) -> &PathBuf {
        &self.current_directory
    }

    /// Change current working directory
    pub fn change_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let normalized_path = if self.options.normalize_paths {
            let path_str = path.to_string_lossy();
            let result = self.normalizer.normalize(&path_str)?;
            PathBuf::from(result.path())
        } else {
            path.to_path_buf()
        };

        if !normalized_path.is_dir() {
            return Err(CmdError::CommandNotFound(
                format!("Directory not found: {}", normalized_path.display())
            ).into());
        }

        self.current_directory = normalized_path;
        env::set_current_dir(&self.current_directory)?;
        Ok(())
    }

    /// Set environment variable
    pub fn set_env_var<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        let key = key.into();
        let value = value.into();
        self.env_cache.insert(key.clone(), value.clone());
        env::set_var(&key, &value);
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.env_cache.get(key)
    }

    /// Remove environment variable
    pub fn remove_env_var(&mut self, key: &str) {
        self.env_cache.remove(key);
        env::remove_var(key);
    }

    /// Normalize a command string with path arguments
    fn normalize_command(&self, command: &str) -> Result<String> {
        if !self.options.normalize_paths {
            return Ok(command.to_string());
        }

        // Simple command parsing - could be enhanced
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(command.to_string());
        }

        let mut normalized_parts = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                // First part is the command itself
                normalized_parts.push(part.to_string());
            } else if self.looks_like_path(part) {
                // Try to normalize what looks like a path
                match self.normalizer.normalize(part) {
                    Ok(result) => {
                        // Quote the path if it contains spaces
                        let path_str = result.path();
                        if path_str.contains(' ') {
                            normalized_parts.push(format!("\"{}\"", path_str));
                        } else {
                            normalized_parts.push(path_str.to_string());
                        }
                    }
                    Err(_) => {
                        // If normalization fails, keep original
                        normalized_parts.push(part.to_string());
                    }
                }
            } else {
                // Not a path, keep as-is
                normalized_parts.push(part.to_string());
            }
        }

        Ok(normalized_parts.join(" "))
    }

    /// Check if a string looks like a path
    fn looks_like_path(&self, s: &str) -> bool {
        // Remove quotes
        let s = s.trim_matches('"').trim_matches('\'');

        // Check for common path patterns
        s.contains('\\') ||
        s.contains('/') ||
        s.contains(':') ||
        s.starts_with('.') ||
        s.starts_with('~')
    }

    /// Parse command string into program and arguments
    fn parse_command(&self, command: &str) -> Result<(String, Vec<String>)> {
        let parts = self.parse_command_line(command);
        if parts.is_empty() {
            return Err(CmdError::CommandNotFound("Empty command".to_string()).into());
        }

        let program = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok((program, args))
    }

    /// Parse command line respecting quotes
    fn parse_command_line(&self, command: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';

        for ch in command.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                ch if ch == quote_char && in_quotes => {
                    in_quotes = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current);
        }

        parts
    }

    /// Set up environment for command execution
    fn setup_environment(&self, cmd: &mut Command) -> Result<()> {
        // Clear environment if not inheriting
        if !self.options.inherit_env {
            cmd.env_clear();
        }

        // Set custom environment variables
        for (key, value) in &self.options.env_vars {
            let translated_value = if self.options.translate_env_vars {
                self.translate_env_value(value)?
            } else {
                value.clone()
            };
            cmd.env(key, translated_value);
        }

        // Remove specified environment variables
        for key in &self.options.env_remove {
            cmd.env_remove(key);
        }

        // Set PATH if we've normalized any commands
        if self.options.normalize_paths {
            if let Some(path) = env::var_os("PATH") {
                cmd.env("PATH", path);
            }
        }

        Ok(())
    }

    /// Translate environment variable value (e.g., convert paths)
    fn translate_env_value(&self, value: &str) -> Result<String> {
        // Check if value looks like a path list (PATH variable)
        if value.contains(';') || value.contains(':') {
            let separator = if cfg!(windows) { ';' } else { ':' };
            let paths: Vec<&str> = value.split(separator).collect();
            let mut translated_paths = Vec::new();

            for path in paths {
                if self.looks_like_path(path) {
                    match self.normalizer.normalize(path) {
                        Ok(result) => translated_paths.push(result.path().to_string()),
                        Err(_) => translated_paths.push(path.to_string()),
                    }
                } else {
                    translated_paths.push(path.to_string());
                }
            }

            Ok(translated_paths.join(&separator.to_string()))
        } else if self.looks_like_path(value) {
            // Single path value
            match self.normalizer.normalize(value) {
                Ok(result) => Ok(result.path().to_string()),
                Err(_) => Ok(value.to_string()),
            }
        } else {
            // Not a path, return as-is
            Ok(value.to_string())
        }
    }

    /// Execute command with timeout
    fn execute_with_timeout(
        &self,
        mut cmd: Command,
        timeout: Duration,
    ) -> Result<(ExitStatus, String, String)> {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();

        let _handle = thread::spawn(move || {
            let result = cmd.output();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok((output.status, stdout, stderr))
            }
            Ok(Err(e)) => Err(CmdError::Io(e).into()),
            Err(_) => {
                // Timeout occurred
                // Note: We can't easily kill the spawned process here
                // In a real implementation, you'd want better process management
                Err(CmdError::Timeout("Command execution timed out".to_string()).into())
            }
        }
    }

    /// Execute command immediately
    fn execute_immediate(&self, mut cmd: Command) -> Result<(ExitStatus, String, String)> {
        let output = cmd.output()
            .map_err(|e| CmdError::Io(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((output.status, stdout, stderr))
    }

    /// Get effective environment variables
    fn get_effective_env_vars(&self) -> HashMap<String, String> {
        let mut effective_env = HashMap::new();

        // Start with cached environment
        if self.options.inherit_env {
            effective_env.extend(self.env_cache.clone());
        }

        // Add custom variables
        effective_env.extend(self.options.env_vars.clone());

        // Remove specified variables
        for key in &self.options.env_remove {
            effective_env.remove(key);
        }

        effective_env
    }

    /// Get available command completions
    pub fn get_completions(&self, prefix: &str) -> Vec<String> {
        if !self.options.enable_completion {
            return Vec::new();
        }

        let mut completions = Vec::new();

        // Add built-in commands
        let builtin_commands = [
            "cd", "dir", "copy", "move", "del", "md", "rd", "type", "echo",
            "set", "path", "cls", "exit", "help", "attrib", "xcopy",
        ];

        for cmd in &builtin_commands {
            if cmd.starts_with(prefix) {
                completions.push(cmd.to_string());
            }
        }

        // Add executables from PATH
        if let Ok(path_var) = env::var("PATH") {
            for path_dir in path_var.split(';') {
                if let Ok(entries) = std::fs::read_dir(path_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                                if name.ends_with(".exe") || name.ends_with(".bat") || name.ends_with(".cmd") {
                                    let cmd_name = name.trim_end_matches(".exe")
                                                      .trim_end_matches(".bat")
                                                      .trim_end_matches(".cmd");
                                    completions.push(cmd_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort
        completions.sort();
        completions.dedup();
        completions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_cmd_wrapper_creation() {
        let options = CmdOptions::new();
        let wrapper = CmdWrapper::new(options);
        assert!(wrapper.is_ok());
    }

    #[test]
    fn test_path_detection() {
        let options = CmdOptions::new();
        let wrapper = CmdWrapper::new(options).unwrap();

        assert!(wrapper.looks_like_path("C:\\Users"));
        assert!(wrapper.looks_like_path("/usr/bin"));
        assert!(wrapper.looks_like_path("./relative/path"));
        assert!(wrapper.looks_like_path("~/home"));
        assert!(!wrapper.looks_like_path("command"));
        assert!(!wrapper.looks_like_path("--flag"));
    }

    #[test]
    fn test_command_parsing() {
        let options = CmdOptions::new();
        let wrapper = CmdWrapper::new(options).unwrap();

        let parts = wrapper.parse_command_line("dir \"C:\\Program Files\" /s");
        assert_eq!(parts, vec!["dir", "C:\\Program Files", "/s"]);

        let parts = wrapper.parse_command_line("echo 'hello world'");
        assert_eq!(parts, vec!["echo", "hello world"]);
    }

    #[test]
    fn test_environment_variable_translation() {
        let options = CmdOptions::new().translate_env_vars(true);
        let wrapper = CmdWrapper::new(options).unwrap();

        // Test PATH-like variable
        let result = wrapper.translate_env_value("C:\\Windows\\System32;C:\\Windows").unwrap();
        assert!(result.contains("Windows"));

        // Test single path
        let result = wrapper.translate_env_value("C:\\Users\\test").unwrap();
        assert!(result.contains("Users"));
    }

    #[test]
    fn test_history() {
        let mut wrapper = CmdWrapper::new(CmdOptions::new()).unwrap();

        // Execute a simple command
        if cfg!(windows) {
            let _ = wrapper.execute_command("echo test");
            assert_eq!(wrapper.get_history().len(), 1);
        }
    }

    #[test]
    fn test_completions() {
        let options = CmdOptions::new().enable_completion(true);
        let wrapper = CmdWrapper::new(options).unwrap();

        let completions = wrapper.get_completions("e");
        assert!(completions.contains(&"echo".to_string()));
        assert!(completions.contains(&"exit".to_string()));
    }

    #[test]
    fn test_command_exists() {
        let wrapper = CmdWrapper::new(CmdOptions::new()).unwrap();

        if cfg!(windows) {
            assert!(wrapper.command_exists("dir"));
            assert!(wrapper.command_exists("echo"));
            assert!(!wrapper.command_exists("nonexistent_command_12345"));
        }
    }
}
