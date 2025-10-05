//! Error handling for WinUtils Core
//!
//! Provides comprehensive error types for all enhanced functionality.

use std::fmt;
use thiserror::Error;

/// Result type for WinUtils operations
pub type WinUtilsResult<T> = Result<T, WinUtilsError>;

/// Comprehensive error types for WinUtils functionality
#[derive(Error, Debug)]
pub enum WinUtilsError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Git repository operation failed
    #[cfg(feature = "version")]
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// Windows API operation failed
    #[cfg(feature = "windows-enhanced")]
    #[error("Windows API error: {message}")]
    WindowsApi { message: String },

    /// Path operation failed
    #[error("Path error: {0}")]
    Path(String),

    /// Help system error
    #[error("Help system error: {0}")]
    Help(String),

    /// Version system error
    #[error("Version system error: {0}")]
    Version(String),

    /// Testing framework error
    #[cfg(feature = "testing")]
    #[error("Testing error: {0}")]
    Testing(String),

    /// Diagnostics error
    #[cfg(feature = "diagnostics")]
    #[error("Diagnostics error: {0}")]
    Diagnostics(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network operation failed (for update checking)
    #[error("Network error: {0}")]
    Network(String),

    /// Permission denied
    #[error("Permission denied: {operation} requires {required_permission}")]
    PermissionDenied {
        operation: String,
        required_permission: String,
    },

    /// Feature not available
    #[error("Feature '{feature}' is not available on this platform or not compiled in")]
    FeatureNotAvailable { feature: String },

    /// Validation failed
    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },

    /// Generic error with context
    #[error("Error in {context}: {message}")]
    Generic { context: String, message: String },
}

impl WinUtilsError {
    /// Create a new path error
    pub fn path<S: Into<String>>(message: S) -> Self {
        Self::Path(message.into())
    }

    /// Create a new help system error
    pub fn help<S: Into<String>>(message: S) -> Self {
        Self::Help(message.into())
    }

    /// Create a new version system error
    pub fn version<S: Into<String>>(message: S) -> Self {
        Self::Version(message.into())
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config(message.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network(message.into())
    }

    /// Create a permission denied error
    pub fn permission_denied<S: Into<String>>(operation: S, required_permission: S) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            required_permission: required_permission.into(),
        }
    }

    /// Create a feature not available error
    pub fn feature_not_available<S: Into<String>>(feature: S) -> Self {
        Self::FeatureNotAvailable {
            feature: feature.into(),
        }
    }

    /// Create a validation failed error
    pub fn validation_failed<S: Into<String>>(reason: S) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }

    /// Create a generic error with context
    pub fn generic<S: Into<String>>(context: S, message: S) -> Self {
        Self::Generic {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Create a Windows API error
    #[cfg(feature = "windows-enhanced")]
    pub fn windows_api<S: Into<String>>(message: S) -> Self {
        Self::WindowsApi {
            message: message.into(),
        }
    }

    /// Create a testing error
    #[cfg(feature = "testing")]
    pub fn testing<S: Into<String>>(message: S) -> Self {
        Self::Testing(message.into())
    }

    /// Create a diagnostics error
    #[cfg(feature = "diagnostics")]
    pub fn diagnostics<S: Into<String>>(message: S) -> Self {
        Self::Diagnostics(message.into())
    }

    /// Check if this error indicates a recoverable condition
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Io(_) => true,
            Self::Network(_) => true,
            Self::WindowsApi { .. } => true,
            Self::Path(_) => false,
            Self::PermissionDenied { .. } => false,
            Self::FeatureNotAvailable { .. } => false,
            Self::ValidationFailed { .. } => false,
            _ => true,
        }
    }

    /// Get error category for logging/reporting
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Serde(_) => "serialization",
            #[cfg(feature = "version")]
            Self::Git(_) => "git",
            #[cfg(feature = "windows-enhanced")]
            Self::WindowsApi { .. } => "windows-api",
            Self::Path(_) => "path",
            Self::Help(_) => "help",
            Self::Version(_) => "version",
            #[cfg(feature = "testing")]
            Self::Testing(_) => "testing",
            #[cfg(feature = "diagnostics")]
            Self::Diagnostics(_) => "diagnostics",
            Self::Config(_) => "config",
            Self::Network(_) => "network",
            Self::PermissionDenied { .. } => "permission",
            Self::FeatureNotAvailable { .. } => "feature",
            Self::ValidationFailed { .. } => "validation",
            Self::Generic { .. } => "generic",
        }
    }
}

/// Trait for providing additional context to errors
pub trait ErrorContext<T> {
    /// Add context to the error
    fn with_context<F>(self, f: F) -> WinUtilsResult<T>
    where
        F: FnOnce() -> String;

    /// Add context with a static string
    fn with_context_str(self, context: &'static str) -> WinUtilsResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<WinUtilsError>,
{
    fn with_context<F>(self, f: F) -> WinUtilsResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let original = e.into();
            WinUtilsError::generic(f(), original.to_string())
        })
    }

    fn with_context_str(self, context: &'static str) -> WinUtilsResult<T> {
        self.with_context(|| context.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = WinUtilsError::path("Invalid path");
        assert_eq!(err.category(), "path");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_context() {
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));

        let with_context = result.with_context_str("Reading configuration file");
        assert!(with_context.is_err());
    }

    #[test]
    fn test_permission_denied() {
        let err = WinUtilsError::permission_denied("write file", "administrator privileges");
        match err {
            WinUtilsError::PermissionDenied { operation, required_permission } => {
                assert_eq!(operation, "write file");
                assert_eq!(required_permission, "administrator privileges");
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }
}
