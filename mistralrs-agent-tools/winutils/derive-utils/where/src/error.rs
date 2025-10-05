// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types for the where utility

use thiserror::Error;

/// Errors that can occur during where execution
#[derive(Error, Debug)]
pub enum WhereError {
    #[error("File not found")]
    NotFound,

    #[error("Invalid pattern: {pattern}")]
    InvalidPattern { pattern: String },

    #[error("Permission denied accessing: {path}")]
    PermissionDenied { path: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {message}")]
    Path { message: String },

    #[error("Environment error: {variable}")]
    Environment { variable: String },

    #[error("Cache error: {message}")]
    Cache { message: String },

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Pattern error: {0}")]
    Pattern(#[from] glob::PatternError),
}

impl WhereError {
    /// Create a new path error
    pub fn path<S: Into<String>>(message: S) -> Self {
        Self::Path {
            message: message.into(),
        }
    }

    /// Create a new environment error
    pub fn environment<S: Into<String>>(variable: S) -> Self {
        Self::Environment {
            variable: variable.into(),
        }
    }

    /// Create a new cache error
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Create a new permission denied error
    pub fn permission_denied<S: Into<String>>(path: S) -> Self {
        Self::PermissionDenied {
            path: path.into(),
        }
    }

    /// Create a new invalid pattern error
    pub fn invalid_pattern<S: Into<String>>(pattern: S) -> Self {
        Self::InvalidPattern {
            pattern: pattern.into(),
        }
    }
}

/// Result type for where operations
pub type WhereResult<T> = Result<T, WhereError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = WhereError::path("test path error");
        assert!(matches!(err, WhereError::Path { .. }));

        let err = WhereError::environment("PATH");
        assert!(matches!(err, WhereError::Environment { .. }));

        let err = WhereError::cache("cache error");
        assert!(matches!(err, WhereError::Cache { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = WhereError::NotFound;
        assert_eq!(err.to_string(), "File not found");

        let err = WhereError::invalid_pattern("*.invalid");
        assert_eq!(err.to_string(), "Invalid pattern: *.invalid");
    }
}
