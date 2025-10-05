//! Core shell command execution engine
//!
//! Provides safe shell command execution with sandbox integration,
//! timeout handling, and cross-platform shell support.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult, CommandOptions, CommandResult};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Execute a shell command
///
/// # Arguments
/// * `sandbox` - Sandbox for working directory validation
/// * `command` - Command string to execute
/// * `options` - Command execution options
///
/// # Returns
/// * CommandResult with status, stdout, stderr, and execution time
///
/// # Safety
/// This function executes arbitrary shell commands within the sandbox constraints.
/// The working directory must be within the sandbox if specified.
pub fn execute(
    sandbox: &Sandbox,
    command: &str,
    options: &CommandOptions,
) -> AgentResult<CommandResult> {
    if command.is_empty() {
        return Err(AgentError::InvalidInput("Empty command".to_string()));
    }

    // Validate working directory if specified
    let working_dir = if let Some(ref dir) = options.working_dir {
        sandbox.validate_read(dir)?;
        Some(dir.clone())
    } else {
        None
    };

    let start = Instant::now();

    // Build command
    let mut cmd = Command::new(options.shell.executable());
    cmd.arg(options.shell.command_flag()).arg(command);

    // Set working directory
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables
    for (key, value) in &options.env {
        cmd.env(key, value);
    }

    // Configure stdio
    if options.capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null());
    }

    if options.capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::null());
    }

    // Execute with timeout if specified
    let output = if let Some(timeout_secs) = options.timeout {
        let child = cmd
            .spawn()
            .map_err(|e| AgentError::IoError(format!("Failed to spawn command: {}", e)))?;

        wait_with_timeout(child, Duration::from_secs(timeout_secs))?
    } else {
        cmd.output()
            .map_err(|e| AgentError::IoError(format!("Failed to execute command: {}", e)))?
    };

    let duration = start.elapsed();

    // Extract output
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    Ok(CommandResult {
        status,
        stdout,
        stderr,
        duration_ms: duration.as_millis() as u64,
    })
}

/// Wait for child process with timeout
fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> AgentResult<std::process::Output> {
    use std::io::Read;
    use std::thread;

    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished
                let stdout = if let Some(mut stdout_handle) = child.stdout.take() {
                    let mut buf = Vec::new();
                    stdout_handle.read_to_end(&mut buf).map_err(|e| {
                        AgentError::IoError(format!("Failed to read stdout: {}", e))
                    })?;
                    buf
                } else {
                    Vec::new()
                };

                let stderr = if let Some(mut stderr_handle) = child.stderr.take() {
                    let mut buf = Vec::new();
                    stderr_handle.read_to_end(&mut buf).map_err(|e| {
                        AgentError::IoError(format!("Failed to read stderr: {}", e))
                    })?;
                    buf
                } else {
                    Vec::new()
                };

                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Process still running
                if start.elapsed() >= timeout {
                    // Timeout - kill process
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AgentError::IoError(format!(
                        "Command timed out after {} seconds",
                        timeout.as_secs()
                    )));
                }

                // Sleep briefly before checking again
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(AgentError::IoError(format!(
                    "Failed to wait for child: {}",
                    e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SandboxConfig, ShellType};
    use tempfile::TempDir;

    #[test]
    fn test_execute_basic() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        #[cfg(windows)]
        let command = "echo hello";
        #[cfg(not(windows))]
        let command = "echo hello";

        let options = CommandOptions {
            working_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let result = execute(&sandbox, command, &options).unwrap();

        assert_eq!(result.status, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_execute_with_env() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        #[cfg(windows)]
        let command = "powershell -Command \"Write-Host $env:TEST_VAR\"";
        #[cfg(not(windows))]
        let command = "echo $TEST_VAR";

        let options = CommandOptions {
            env: vec![("TEST_VAR".to_string(), "test_value".to_string())],
            shell: ShellType::PowerShell,
            ..Default::default()
        };

        let result = execute(&sandbox, command, &options).unwrap();

        assert_eq!(result.status, 0);
        assert!(result.stdout.contains("test_value"));
    }

    #[test]
    fn test_execute_error() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        #[cfg(windows)]
        let command = "cmd.exe /c exit 1";
        #[cfg(not(windows))]
        let command = "exit 1";

        let options = CommandOptions::default();

        let result = execute(&sandbox, command, &options).unwrap();

        assert_eq!(result.status, 1);
    }

    #[test]
    fn test_execute_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        #[cfg(windows)]
        let command = "timeout /t 10 /nobreak";
        #[cfg(not(windows))]
        let command = "sleep 10";

        let options = CommandOptions {
            timeout: Some(1), // 1 second timeout
            ..Default::default()
        };

        let result = execute(&sandbox, command, &options);

        assert!(result.is_err());
        if let Err(AgentError::IoError(msg)) = result {
            assert!(msg.contains("timed out"));
        } else {
            panic!("Expected timeout error");
        }
    }

    #[test]
    fn test_execute_sandbox_violation() {
        let temp_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        let options = CommandOptions {
            working_dir: Some(outside_dir.path().to_path_buf()),
            ..Default::default()
        };

        let result = execute(&sandbox, "echo test", &options);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::SandboxViolation(_)
        ));
    }

    #[test]
    #[cfg(windows)]
    fn test_execute_powershell() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        let options = CommandOptions {
            shell: ShellType::PowerShell,
            ..Default::default()
        };

        let result = execute(&sandbox, "Get-Date", &options).unwrap();

        assert_eq!(result.status, 0);
        assert!(!result.stdout.is_empty());
    }

    #[test]
    #[cfg(windows)]
    fn test_execute_cmd() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        let options = CommandOptions {
            shell: ShellType::Cmd,
            ..Default::default()
        };

        let result = execute(&sandbox, "ver", &options).unwrap();

        assert_eq!(result.status, 0);
        assert!(!result.stdout.is_empty());
    }
}
