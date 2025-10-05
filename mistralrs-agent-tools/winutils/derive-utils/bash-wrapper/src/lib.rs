//! # bash-wrapper - Bash Shell Wrapper with Path Normalization
//!
//! A Bash wrapper that provides automatic path normalization for Git Bash,
//! WSL, and other Unix-like environments on Windows.

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
pub enum BashError {
    #[error("Bash execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Path normalization error: {0}")]
    PathNormalization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Bash not found")]
    BashNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashOptions {
    pub normalize_paths: bool,
    pub working_directory: Option<PathBuf>,
    pub env_vars: HashMap<String, String>,
    pub timeout: Option<Duration>,
    pub capture_output: bool,
    pub echo_commands: bool,
    pub login_shell: bool,
    pub interactive: bool,
    pub source_profile: bool,
}

impl Default for BashOptions {
    fn default() -> Self {
        Self {
            normalize_paths: true,
            working_directory: None,
            env_vars: HashMap::new(),
            timeout: None,
            capture_output: true,
            echo_commands: false,
            login_shell: false,
            interactive: false,
            source_profile: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub normalized_command: String,
}

pub struct BashWrapper {
    options: BashOptions,
    normalizer: PathNormalizer,
    bash_path: PathBuf,
}

impl BashWrapper {
    pub fn new(options: BashOptions) -> Result<Self> {
        let normalizer = PathNormalizer::new();
        let bash_path = Self::find_bash()?;

        Ok(Self {
            options,
            normalizer,
            bash_path,
        })
    }

    fn find_bash() -> Result<PathBuf> {
        let candidates = [
            "bash",
            "/usr/bin/bash",
            "/bin/bash",
            "C:\\Program Files\\Git\\usr\\bin\\bash.exe",
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\msys64\\usr\\bin\\bash.exe",
        ];

        for candidate in &candidates {
            if let Ok(path) = which::which(candidate) {
                return Ok(path);
            }
        }

        Err(BashError::BashNotFound.into())
    }

    pub fn execute_command(&self, command: &str) -> Result<BashResult> {
        let start_time = Instant::now();

        debug!("Executing Bash command: {}", command);

        let normalized_command = self.normalize_command(command)?;

        if self.options.echo_commands {
            println!("+ {}", normalized_command);
        }

        let mut cmd = Command::new(&self.bash_path);

        // Add bash arguments
        if self.options.login_shell {
            cmd.arg("-l");
        }
        if self.options.interactive {
            cmd.arg("-i");
        }

        cmd.args(["-c", &normalized_command]);

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
            .map_err(|e| BashError::Io(e))?;

        let execution_time = start_time.elapsed();

        let result = BashResult {
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

        // Basic bash command normalization
        // Convert Windows paths to Git Bash format
        let mut normalized = command.to_string();

        // Simple regex replacement for common Windows paths
        // This would be enhanced in a real implementation
        if normalized.contains("C:\\") {
            normalized = normalized.replace("C:\\", "/c/");
            normalized = normalized.replace("\\", "/");
        }

        Ok(normalized)
    }
}
