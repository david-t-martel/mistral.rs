//! Core winutils executable wrapper
//!
//! Provides infrastructure for executing winutils command-line utilities
//! with proper path handling, argument passing, and error handling.

use crate::tools::sandbox::Sandbox;
use crate::tools::shell::execute;
use crate::types::{AgentError, AgentResult, CommandOptions, CommandResult, ShellType};
use std::path::PathBuf;

/// Winutils command builder
#[derive(Debug, Clone)]
pub struct WinutilCommand {
    /// The utility name (e.g., "cut", "base64")
    pub utility: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Winutils installation path (defaults to T:\projects\coreutils\winutils\target\release)
    pub winutils_path: Option<PathBuf>,
}

impl WinutilCommand {
    /// Create a new winutils command
    pub fn new(utility: impl Into<String>) -> Self {
        Self {
            utility: utility.into(),
            args: Vec::new(),
            working_dir: None,
            winutils_path: None,
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Set winutils installation path
    pub fn winutils_path(mut self, path: PathBuf) -> Self {
        self.winutils_path = Some(path);
        self
    }

    /// Execute the command
    pub fn execute(self, sandbox: &Sandbox) -> AgentResult<CommandResult> {
        winutil_exec(sandbox, &self)
    }
}

/// Execute a winutils command
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `command` - Winutils command to execute
///
/// # Returns
/// * CommandResult with stdout, stderr, and exit status
///
/// # Example
/// ```no_run
/// use mistralrs_agent_tools::tools::winutils::{WinutilCommand, winutil_exec};
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::types::{AgentResult, SandboxConfig};
///
/// fn main() -> AgentResult<()> {
///     let sandbox = Sandbox::new(SandboxConfig::default());
///     let cmd = WinutilCommand::new("base64")
///         .arg("--encode")
///         .arg("file.txt");
///
///     let result = winutil_exec(&sandbox, &cmd)?;
///     println!("{}", result.stdout);
///     Ok(())
/// }
/// ```
pub fn winutil_exec(sandbox: &Sandbox, command: &WinutilCommand) -> AgentResult<CommandResult> {
    // Determine winutils path
    let winutils_dir = command
        .winutils_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("T:\\projects\\coreutils\\winutils\\target\\release"));

    // Build executable path
    let exe_path = winutils_dir.join(format!("{}.exe", command.utility));

    // Check if executable exists
    if !exe_path.exists() {
        return Err(AgentError::NotFound(format!(
            "Winutils executable not found: {}",
            exe_path.display()
        )));
    }

    // Build command string with quoted arguments
    let mut cmd_parts = vec![format!("\"{}\"", exe_path.display())];

    for arg in &command.args {
        // Quote arguments that contain spaces
        if arg.contains(' ') {
            cmd_parts.push(format!("\"{}\"", arg));
        } else {
            cmd_parts.push(arg.clone());
        }
    }

    let command_str = cmd_parts.join(" ");

    // Build command options
    let options = CommandOptions {
        shell: ShellType::Cmd, // Use cmd.exe for better compatibility
        working_dir: command.working_dir.clone(),
        timeout: Some(60), // 60 second default timeout
        capture_stdout: true,
        capture_stderr: true,
        env: Vec::new(),
    };

    // Execute through shell
    execute(sandbox, &command_str, &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winutil_command_builder() {
        let cmd = WinutilCommand::new("base64")
            .arg("--encode")
            .arg("file.txt")
            .working_dir(PathBuf::from("/tmp"));

        assert_eq!(cmd.utility, "base64");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], "--encode");
        assert_eq!(cmd.args[1], "file.txt");
        assert!(cmd.working_dir.is_some());
    }

    #[test]
    fn test_winutil_command_args() {
        let cmd = WinutilCommand::new("cut")
            .args(vec!["-f", "1,2,3"])
            .arg("file.txt");

        assert_eq!(cmd.args.len(), 3);
        assert_eq!(cmd.args[0], "-f");
        assert_eq!(cmd.args[1], "1,2,3");
        assert_eq!(cmd.args[2], "file.txt");
    }
}
