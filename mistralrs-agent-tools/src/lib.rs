//! Sandboxed filesystem operations for mistral.rs agent mode
//!
//! This crate provides safe, sandboxed filesystem operations for the TUI agent.
//!
//! # Features
//!
//! - **Sandbox enforcement**: All operations restricted to a sandbox root
//! - **Path traversal prevention**: Blocks attempts to escape the sandbox
//! - **Symlink safety**: Validates symlinks don't escape sandbox
//! - **Read-only protection**: Configurable read-only paths
//! - **Audit logging**: Structured logs of filesystem operations

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::{info, warn};
use walkdir::WalkDir;

/// Maximum file size for read operations (5 MiB)
const MAX_READ_SIZE: u64 = 5 * 1024 * 1024;

/// Maximum number of results for find/tree operations
const MAX_RESULTS: usize = 1000;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Root directory for all filesystem operations
    pub root: Utf8PathBuf,
    /// Read-only paths that cannot be modified
    pub readonly_paths: Vec<Utf8PathBuf>,
    /// Whether to enforce sandbox restrictions
    pub enforce: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        // Default to current directory or MISTRALRS_AGENT_SANDBOX_ROOT env var
        let root = std::env::var("MISTRALRS_AGENT_SANDBOX_ROOT")
            .ok()
            .and_then(|p| Utf8PathBuf::from_path_buf(PathBuf::from(p)).ok())
            .unwrap_or_else(|| {
                Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap_or_default())
                    .unwrap_or_else(|_| Utf8PathBuf::from("."))
            });

        // Canonicalize the root to ensure consistent path comparisons
        let root = root.canonicalize_utf8().unwrap_or(root);

        Self {
            root,
            readonly_paths: vec![
                Utf8PathBuf::from(".git"),
                Utf8PathBuf::from("target"),
                Utf8PathBuf::from("node_modules"),
            ],
            enforce: true,
        }
    }
}

/// Errors that can occur during filesystem operations
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Path '{0}' is outside the sandbox root")]
    OutsideSandbox(String),

    #[error("Path '{0}' is read-only")]
    ReadOnly(String),

    #[error("File too large: {0} bytes (max {1})")]
    FileTooLarge(u64, u64),

    #[error("Too many results: {0} (max {1})")]
    TooManyResults(usize, usize),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of a filesystem operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsResult {
    pub success: bool,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

impl FsResult {
    pub fn success(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: true,
            path: path.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(
        path: impl Into<String>,
        message: impl Into<String>,
        data: String,
    ) -> Self {
        Self {
            success: true,
            path: path.into(),
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(path: impl Into<String>, error: FsError) -> Self {
        Self {
            success: false,
            path: path.into(),
            message: error.to_string(),
            data: None,
        }
    }
}

/// Sandboxed filesystem operations
pub struct AgentTools {
    config: SandboxConfig,
}

impl AgentTools {
    /// Create a new AgentTools instance with the given configuration
    pub fn new(config: SandboxConfig) -> Self {
        info!(
            "Initializing agent filesystem tools with sandbox root: {}",
            config.root
        );
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SandboxConfig::default())
    }

    /// Get the sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Validate and normalize a path within the sandbox
    fn validate_path(&self, path: &str) -> Result<Utf8PathBuf, FsError> {
        let path = Utf8Path::new(path);

        // Convert to absolute path
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.root.join(path)
        };

        // Try to canonicalize if path exists, otherwise normalize manually
        let normalized = if absolute.exists() {
            absolute
                .canonicalize_utf8()
                .map_err(|e| FsError::InvalidPath(format!("{}: {}", path, e)))?
        } else {
            // For non-existent paths, canonicalize parent and append filename
            if let Some(parent) = absolute.parent() {
                if parent.exists() {
                    let canonical_parent = parent
                        .canonicalize_utf8()
                        .map_err(|e| FsError::InvalidPath(format!("{}: {}", path, e)))?;
                    if let Some(filename) = absolute.file_name() {
                        canonical_parent.join(filename)
                    } else {
                        absolute
                    }
                } else {
                    // Parent doesn't exist either - use absolute path
                    absolute
                }
            } else {
                absolute
            }
        };

        // Check if path is within sandbox
        if self.config.enforce && !normalized.starts_with(&self.config.root) {
            warn!("Path traversal attempt blocked: {}", normalized);
            return Err(FsError::OutsideSandbox(normalized.to_string()));
        }

        Ok(normalized)
    }

    /// Check if a path is read-only
    fn is_readonly(&self, path: &Utf8Path) -> bool {
        self.config.readonly_paths.iter().any(|ro| {
            // Check if any component of the path matches the readonly pattern
            path.components().any(|comp| comp.as_str() == ro.as_str())
        })
    }

    /// Read a file's contents
    pub fn read(&self, path: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;
        info!("Reading file: {}", validated_path);

        // Check file size
        let metadata = fs::metadata(&validated_path)?;
        if metadata.len() > MAX_READ_SIZE {
            return Err(FsError::FileTooLarge(metadata.len(), MAX_READ_SIZE));
        }

        let contents = fs::read_to_string(&validated_path)?;
        Ok(FsResult::success_with_data(
            validated_path.as_str(),
            format!("Read {} bytes", contents.len()),
            contents,
        ))
    }

    /// Write contents to a file
    pub fn write(
        &self,
        path: &str,
        content: &str,
        create: bool,
        overwrite: bool,
    ) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!(
            "Writing file: {} (create={}, overwrite={})",
            validated_path, create, overwrite
        );

        // Prepare parent directory
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Check if file exists
        if validated_path.exists() && !overwrite {
            return Err(FsError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "File exists and overwrite=false",
            )));
        }

        fs::write(&validated_path, content)?;
        Ok(FsResult::success(
            validated_path.as_str(),
            format!("Wrote {} bytes", content.len()),
        ))
    }

    /// Append contents to a file
    pub fn append(&self, path: &str, content: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Appending to file: {}", validated_path);

        use std::fs::OpenOptions;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&validated_path)?;

        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        Ok(FsResult::success(
            validated_path.as_str(),
            format!("Appended {} bytes", content.len()),
        ))
    }

    /// Delete a file
    pub fn delete(&self, path: &str) -> Result<FsResult, FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Deleting file: {}", validated_path);
        fs::remove_file(&validated_path)?;
        Ok(FsResult::success(validated_path.as_str(), "Deleted"))
    }

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> Result<bool, FsError> {
        let validated_path = self.validate_path(path)?;
        Ok(validated_path.exists())
    }

    /// Find files matching patterns
    pub fn find(&self, pattern: &str, max_depth: Option<usize>) -> Result<Vec<String>, FsError> {
        info!("Finding files with pattern: {}", pattern);

        let mut results = Vec::new();
        let walker = if let Some(depth) = max_depth {
            WalkDir::new(&self.config.root).max_depth(depth)
        } else {
            WalkDir::new(&self.config.root)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if results.len() >= MAX_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_RESULTS));
            }

            let path = entry.path();
            if let Some(path_str) = path.to_str() {
                // Simple pattern matching - contains the pattern
                if path_str.contains(pattern) {
                    results.push(path_str.to_string());
                }
            }
        }

        Ok(results)
    }

    /// List directory contents as a tree
    pub fn tree(
        &self,
        root: Option<String>,
        max_depth: Option<usize>,
    ) -> Result<Vec<String>, FsError> {
        let start_path = if let Some(r) = root {
            self.validate_path(&r)?
        } else {
            self.config.root.clone()
        };

        info!("Listing tree from: {}", start_path);

        let mut results = Vec::new();
        let walker = if let Some(depth) = max_depth {
            WalkDir::new(&start_path).max_depth(depth)
        } else {
            WalkDir::new(&start_path)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if results.len() >= MAX_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_RESULTS));
            }

            if let Some(path_str) = entry.path().to_str() {
                results.push(path_str.to_string());
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        let root = root.canonicalize_utf8().unwrap();
        let config = SandboxConfig {
            root,
            readonly_paths: vec![],
            enforce: true,
        };

        let tools = AgentTools::new(config);

        // Attempt to read outside sandbox should fail
        let result = tools.read("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_readonly_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        let root = root.canonicalize_utf8().unwrap();
        let config = SandboxConfig {
            root,
            readonly_paths: vec![Utf8PathBuf::from(".git")],
            enforce: true,
        };

        let tools = AgentTools::new(config);

        // Create .git directory
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();

        // Attempt to write to .git should fail
        let result = tools.write(".git/config", "test", true, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        let root = root.canonicalize_utf8().unwrap();
        let config = SandboxConfig {
            root,
            readonly_paths: vec![],
            enforce: true,
        };

        let tools = AgentTools::new(config);

        // Write, read, append, delete
        let write_result = tools.write("test.txt", "Hello", true, true).unwrap();
        assert!(write_result.success);

        let read_result = tools.read("test.txt").unwrap();
        assert!(read_result.success);
        assert_eq!(read_result.data.unwrap(), "Hello");

        let append_result = tools.append("test.txt", " World").unwrap();
        assert!(append_result.success);

        let read_result2 = tools.read("test.txt").unwrap();
        assert_eq!(read_result2.data.unwrap(), "Hello World");

        assert!(tools.exists("test.txt").unwrap());
        let delete_result = tools.delete("test.txt").unwrap();
        assert!(delete_result.success);
        assert!(!tools.exists("test.txt").unwrap());
    }
}
