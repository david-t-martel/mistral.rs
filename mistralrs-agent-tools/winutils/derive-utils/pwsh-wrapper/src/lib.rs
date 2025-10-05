//! # pwsh-wrapper - PowerShell Wrapper with Path Normalization
//!
//! A PowerShell wrapper that provides automatic path normalization and
//! cross-platform compatibility for PowerShell commands.

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;
use winpath::PathNormalizer;

#[derive(Error, Debug)]
pub enum PwshError {
    #[error("PowerShell execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Path normalization error: {0}")]
    PathNormalization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PowerShell not found")]
    PowerShellNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwshOptions {
    pub normalize_paths: bool,
    pub working_directory: Option<PathBuf>,
    pub env_vars: HashMap<String, String>,
    pub timeout: Option<Duration>,
    pub capture_output: bool,
    pub echo_commands: bool,
    pub execution_policy: Option<String>,
    pub no_profile: bool,
    pub no_logo: bool,
}

impl Default for PwshOptions {
    fn default() -> Self {
        Self {
            normalize_paths: true,
            working_directory: None,
            env_vars: HashMap::new(),
            timeout: None,
            capture_output: true,
            echo_commands: false,
            execution_policy: Some("Bypass".to_string()),
            no_profile: true,
            no_logo: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwshResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub normalized_command: String,
}

pub struct PwshWrapper {
    options: PwshOptions,
    normalizer: PathNormalizer,
    powershell_path: PathBuf,
}

impl PwshWrapper {
    pub fn new(options: PwshOptions) -> Result<Self> {
        let normalizer = PathNormalizer::new();
        let powershell_path = Self::find_powershell()?;

        Ok(Self {
            options,
            normalizer,
            powershell_path,
        })
    }

    fn find_powershell() -> Result<PathBuf> {
        // Try to find PowerShell in common locations
        let candidates = [
            "pwsh",  // PowerShell Core
            "powershell", // Windows PowerShell
            "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        ];

        for candidate in &candidates {
            if let Ok(path) = which::which(candidate) {
                return Ok(path);
            }
        }

        Err(PwshError::PowerShellNotFound.into())
    }

    pub fn execute_command(&self, command: &str) -> Result<PwshResult> {
        let start_time = Instant::now();

        debug!("Executing PowerShell command: {}", command);

        let normalized_command = self.normalize_command(command)?;

        if self.options.echo_commands {
            println!("+ {}", normalized_command);
        }

        let mut cmd = Command::new(&self.powershell_path);

        // Add PowerShell arguments
        if self.options.no_logo {
            cmd.arg("-NoLogo");
        }
        if self.options.no_profile {
            cmd.arg("-NoProfile");
        }
        if let Some(policy) = &self.options.execution_policy {
            cmd.args(["-ExecutionPolicy", policy]);
        }

        cmd.args(["-Command", &normalized_command]);

        // Set working directory
        if let Some(dir) = &self.options.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &self.options.env_vars {
            cmd.env(key, value);
        }

        // Set up stdio
        if self.options.capture_output {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        }

        // Execute
        let output = cmd.output()
            .map_err(|e| PwshError::Io(e))?;

        let execution_time = start_time.elapsed();

        let result = PwshResult {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            normalized_command,
        };

        Ok(result)
    }

    fn normalize_command(&self, command: &str) -> Result<String> {
        if !self.options.normalize_paths {
            return Ok(command.to_string());
        }

        // Basic PowerShell path normalization
        // This is simplified - real implementation would need PowerShell AST parsing
        let normalized = command.replace("\\", "/");
        Ok(normalized)
    }
}
