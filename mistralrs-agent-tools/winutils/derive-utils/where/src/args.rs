// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Command line argument parsing for the where utility

use clap::Parser;

/// Command line arguments for the where utility
#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Patterns to search for (supports wildcards)
    #[arg(
        value_name = "PATTERN",
        help = "Executable name or pattern to search for (supports wildcards like *.exe)",
        required = true
    )]
    pub patterns: Vec<String>,

    /// Recursive search from specified directory
    #[arg(
        short = 'R',
        long = "recursive",
        value_name = "DIR",
        help = "Recursively search for files starting from specified directory"
    )]
    pub recursive_dir: Option<String>,

    /// Quiet mode - only exit codes, no output
    #[arg(
        short = 'Q',
        long = "quiet",
        help = "Quiet mode - suppress output, only return exit code"
    )]
    pub quiet: bool,

    /// Display files in full path format
    #[arg(
        short = 'F',
        long = "full",
        help = "Display files in full path format"
    )]
    pub full_path: bool,

    /// Display file size and modification time
    #[arg(
        short = 'T',
        long = "time",
        help = "Display file size and modification time"
    )]
    pub show_time: bool,
}

impl Args {
    /// Validate the arguments for consistency
    pub fn validate(&self) -> Result<(), String> {
        if self.patterns.is_empty() {
            return Err("At least one pattern must be provided".to_string());
        }

        if let Some(dir) = &self.recursive_dir {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                return Err(format!("Directory '{}' does not exist", dir));
            }
            if !path.is_dir() {
                return Err(format!("'{}' is not a directory", dir));
            }
        }

        Ok(())
    }

    /// Check if we should search recursively
    pub fn is_recursive(&self) -> bool {
        self.recursive_dir.is_some()
    }

    /// Get the starting directory for recursive search
    pub fn get_search_root(&self) -> Option<&str> {
        self.recursive_dir.as_deref()
    }

    /// Check if output should be suppressed
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Check if full path format should be used
    pub fn use_full_path(&self) -> bool {
        self.full_path
    }

    /// Check if time information should be shown
    pub fn show_time_info(&self) -> bool {
        self.show_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_patterns() {
        let args = Args {
            patterns: vec![],
            recursive_dir: None,
            quiet: false,
            full_path: false,
            show_time: false,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_nonexistent_directory() {
        let args = Args {
            patterns: vec!["test".to_string()],
            recursive_dir: Some("/nonexistent/directory".to_string()),
            quiet: false,
            full_path: false,
            show_time: false,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_valid_args() {
        let args = Args {
            patterns: vec!["test".to_string()],
            recursive_dir: None,
            quiet: false,
            full_path: false,
            show_time: false,
        };

        assert!(args.validate().is_ok());
    }
}
