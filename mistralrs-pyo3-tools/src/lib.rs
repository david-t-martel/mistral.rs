//! Sandboxed filesystem operations for mistral.rs agent mode
//!
//! This crate provides safe, sandboxed filesystem operations that can be used
//! from Rust code or exposed to Python via PyO3 bindings.
//!
//! # Features
//!
//! - **Sandbox enforcement**: All operations are restricted to a sandbox root
//! - **Path traversal prevention**: Blocks attempts to escape the sandbox
//! - **Symlink safety**: Validates symlinks don't escape the sandbox
//! - **File locking**: Advisory locks for concurrent access
//! - **Audit logging**: Structured logs of all filesystem operations

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use fs4::FileExt;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, info, warn};

#[cfg(feature = "pyo3-bindings")]
use pyo3::prelude::*;

/// Maximum file size for read operations (5 MiB)
const MAX_READ_SIZE: u64 = 5 * 1024 * 1024;

/// Maximum number of results for glob/find operations
const MAX_GLOB_RESULTS: usize = 1000;

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
        // Default to current directory or MISTRALRS_AGENT_SANDBOX_ROOT
        let root = std::env::var("MISTRALRS_AGENT_SANDBOX_ROOT")
            .ok()
            .and_then(|p| Utf8PathBuf::from_path_buf(PathBuf::from(p)).ok())
            .unwrap_or_else(|| {
                Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap_or_default())
                    .unwrap_or_else(|_| Utf8PathBuf::from("."))
            });

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

    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(feature = "pyo3-bindings")]
impl From<FsError> for PyErr {
    fn from(err: FsError) -> PyErr {
        use pyo3::exceptions::*;
        match err {
            FsError::OutsideSandbox(msg) => PyPermissionError::new_err(msg),
            FsError::ReadOnly(msg) => PyPermissionError::new_err(msg),
            FsError::FileTooLarge(_, _) => PyOSError::new_err(err.to_string()),
            FsError::TooManyResults(_, _) => PyOSError::new_err(err.to_string()),
            FsError::InvalidPath(msg) => PyValueError::new_err(msg),
            FsError::Io(e) => PyIOError::new_err(e.to_string()),
            FsError::Other(msg) => PyRuntimeError::new_err(msg),
        }
    }
}

/// Sandboxed filesystem operations
pub struct FsTools {
    config: SandboxConfig,
}

impl FsTools {
    /// Create a new FsTools instance with the given configuration
    pub fn new(config: SandboxConfig) -> Self {
        info!("Initializing filesystem tools with sandbox root: {}", config.root);
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SandboxConfig::default())
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

        // Canonicalize to resolve symlinks and normalize
        let canonical = absolute
            .canonicalize_utf8()
            .map_err(|e| FsError::InvalidPath(format!("{}: {}", path, e)))?;

        // Check if path is within sandbox
        if self.config.enforce && !canonical.starts_with(&self.config.root) {
            warn!("Path traversal attempt blocked: {}", canonical);
            return Err(FsError::OutsideSandbox(canonical.to_string()));
        }

        Ok(canonical)
    }

    /// Check if a path is read-only
    fn is_readonly(&self, path: &Utf8Path) -> bool {
        self.config.readonly_paths.iter().any(|ro| {
            path.starts_with(ro) || path.ends_with(ro)
        })
    }

    /// Read a file's contents
    pub fn read(&self, path: &str) -> Result<String, FsError> {
        let validated_path = self.validate_path(path)?;
        info!("Reading file: {}", validated_path);

        // Check file size
        let metadata = fs::metadata(&validated_path)?;
        if metadata.len() > MAX_READ_SIZE {
            return Err(FsError::FileTooLarge(metadata.len(), MAX_READ_SIZE));
        }

        // Read with advisory lock
        let mut file = File::open(&validated_path)?;
        file.lock_shared()?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        file.unlock()?;
        Ok(contents)
    }

    /// Write contents to a file
    pub fn write(&self, path: &str, content: &str, create: bool, overwrite: bool) -> Result<(), FsError> {
        let validated_path = self.validate_path(path)?;

        // Check if read-only
        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Writing file: {} (create={}, overwrite={})", validated_path, create, overwrite);

        // Prepare parent directory
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Open file with appropriate options
        let mut options = OpenOptions::new();
        options.write(true);

        if create {
            options.create(true);
        }
        if overwrite {
            options.truncate(true);
        } else {
            options.create_new(!validated_path.exists());
        }

        let mut file = options.open(&validated_path)?;
        file.lock_exclusive()?;

        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        file.unlock()?;
        Ok(())
    }

    /// Append contents to a file
    pub fn append(&self, path: &str, content: &str) -> Result<(), FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Appending to file: {}", validated_path);

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&validated_path)?;

        file.lock_exclusive()?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        file.unlock()?;

        Ok(())
    }

    /// Delete a file
    pub fn delete(&self, path: &str) -> Result<(), FsError> {
        let validated_path = self.validate_path(path)?;

        if self.is_readonly(&validated_path) {
            return Err(FsError::ReadOnly(validated_path.to_string()));
        }

        info!("Deleting file: {}", validated_path);
        fs::remove_file(&validated_path)?;
        Ok(())
    }

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> Result<bool, FsError> {
        let validated_path = self.validate_path(path)?;
        Ok(validated_path.exists())
    }

    /// Find files matching a pattern
    pub fn find(&self, pattern: &str, include: Vec<String>, exclude: Vec<String>) -> Result<Vec<String>, FsError> {
        info!("Finding files with pattern: {}", pattern);

        // Build glob set for includes
        let mut include_builder = GlobSetBuilder::new();
        for pat in include {
            include_builder.add(Glob::new(&pat).map_err(|e| FsError::Other(e.to_string()))?);
        }
        let include_set = include_builder.build().map_err(|e| FsError::Other(e.to_string()))?;

        // Build glob set for excludes
        let mut exclude_builder = GlobSetBuilder::new();
        for pat in exclude {
            exclude_builder.add(Glob::new(&pat).map_err(|e| FsError::Other(e.to_string()))?);
        }
        let exclude_set = exclude_builder.build().map_err(|e| FsError::Other(e.to_string()))?;

        // Walk directory
        let mut results = Vec::new();
        for entry in WalkBuilder::new(&self.config.root)
            .hidden(false)
            .build()
        {
            if results.len() >= MAX_GLOB_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_GLOB_RESULTS));
            }

            let entry = entry.map_err(|e| FsError::Other(e.to_string()))?;
            let path = entry.path();

            // Check include/exclude patterns
            if include_set.is_match(path) && !exclude_set.is_match(path) {
                if let Some(path_str) = path.to_str() {
                    results.push(path_str.to_string());
                }
            }
        }

        Ok(results)
    }

    /// List directory contents as a tree
    pub fn tree(&self, root: Option<String>, max_depth: Option<usize>) -> Result<Vec<String>, FsError> {
        let start_path = if let Some(r) = root {
            self.validate_path(&r)?
        } else {
            self.config.root.clone()
        };

        info!("Listing tree from: {}", start_path);

        let mut results = Vec::new();
        let mut walker = WalkBuilder::new(&start_path).hidden(false);

        if let Some(depth) = max_depth {
            walker.max_depth(Some(depth));
        }

        for entry in walker.build() {
            if results.len() >= MAX_GLOB_RESULTS {
                return Err(FsError::TooManyResults(results.len(), MAX_GLOB_RESULTS));
            }

            let entry = entry.map_err(|e| FsError::Other(e.to_string()))?;
            if let Some(path_str) = entry.path().to_str() {
                results.push(path_str.to_string());
            }
        }

        Ok(results)
    }
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_read(path: String) -> PyResult<String> {
    let tools = FsTools::with_defaults();
    tools.read(&path).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_write(path: String, content: String, create: Option<bool>, overwrite: Option<bool>) -> PyResult<()> {
    let tools = FsTools::with_defaults();
    tools.write(&path, &content, create.unwrap_or(true), overwrite.unwrap_or(false))
        .map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_append(path: String, content: String) -> PyResult<()> {
    let tools = FsTools::with_defaults();
    tools.append(&path, &content).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_delete(path: String) -> PyResult<()> {
    let tools = FsTools::with_defaults();
    tools.delete(&path).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_exists(path: String) -> PyResult<bool> {
    let tools = FsTools::with_defaults();
    tools.exists(&path).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_find(pattern: String, include: Option<Vec<String>>, exclude: Option<Vec<String>>) -> PyResult<Vec<String>> {
    let tools = FsTools::with_defaults();
    tools.find(
        &pattern,
        include.unwrap_or_else(|| vec!["**/*".to_string()]),
        exclude.unwrap_or_else(|| vec!["target/**".to_string(), ".git/**".to_string()])
    ).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pyfunction]
fn fs_tree(root: Option<String>, max_depth: Option<usize>) -> PyResult<Vec<String>> {
    let tools = FsTools::with_defaults();
    tools.tree(root, max_depth).map_err(|e| e.into())
}

#[cfg(feature = "pyo3-bindings")]
#[pymodule]
fn mistralrs_pyo3_tools(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fs_read, m)?)?;
    m.add_function(wrap_pyfunction!(fs_write, m)?)?;
    m.add_function(wrap_pyfunction!(fs_append, m)?)?;
    m.add_function(wrap_pyfunction!(fs_delete, m)?)?;
    m.add_function(wrap_pyfunction!(fs_exists, m)?)?;
    m.add_function(wrap_pyfunction!(fs_find, m)?)?;
    m.add_function(wrap_pyfunction!(fs_tree, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig {
            root: Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap(),
            readonly_paths: vec![],
            enforce: true,
        };

        let tools = FsTools::new(config);

        // Attempt to read outside sandbox should fail
        let result = tools.read("/etc/passwd");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::OutsideSandbox(_)));
    }

    #[test]
    fn test_readonly_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig {
            root: Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap(),
            readonly_paths: vec![Utf8PathBuf::from(".git")],
            enforce: true,
        };

        let tools = FsTools::new(config);

        // Create .git directory
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();

        // Attempt to write to .git should fail
        let result = tools.write(".git/config", "test", true, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::ReadOnly(_)));
    }

    #[test]
    fn test_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = SandboxConfig {
            root: Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap(),
            readonly_paths: vec![],
            enforce: true,
        };

        let tools = FsTools::new(config);

        // Write, read, append, delete
        tools.write("test.txt", "Hello", true, true).unwrap();
        assert_eq!(tools.read("test.txt").unwrap(), "Hello");

        tools.append("test.txt", " World").unwrap();
        assert_eq!(tools.read("test.txt").unwrap(), "Hello World");

        assert!(tools.exists("test.txt").unwrap());
        tools.delete("test.txt").unwrap();
        assert!(!tools.exists("test.txt").unwrap());
    }
}
