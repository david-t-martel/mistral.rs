//! Sandbox enforcement for agent tools.
//!
//! Provides security boundary checking and path validation to ensure
//! all file system operations stay within allowed boundaries.

use crate::pathlib::{is_absolute, normalize_path};
use crate::types::{AgentError, AgentResult, SandboxConfig};
use std::path::{Path, PathBuf};

/// Sandbox enforcer that validates all file system operations
#[derive(Debug, Clone)]
pub struct Sandbox {
    config: SandboxConfig,
}

impl Sandbox {
    /// Creates a new sandbox with the given configuration
    pub fn new(mut config: SandboxConfig) -> Self {
        // Canonicalize the root to ensure consistent comparisons
        if let Ok(canonical) = config.root.canonicalize() {
            config.root = canonical;
        }
        Self { config }
    }

    /// Validates a path for read operations
    pub fn validate_read(&self, path: &Path) -> AgentResult<PathBuf> {
        let normalized = self.normalize_and_canonicalize(path)?;

        // Check if path is within sandbox
        if !self.is_within_sandbox(&normalized) {
            if self.config.allow_read_outside {
                // Allowed to read outside, but still validate the path exists
                if !normalized.exists() {
                    return Err(AgentError::NotFound(format!(
                        "Path does not exist: {}",
                        normalized.display()
                    )));
                }
                return Ok(normalized);
            } else {
                return Err(AgentError::SandboxViolation(format!(
                    "Path outside sandbox: {}",
                    normalized.display()
                )));
            }
        }

        Ok(normalized)
    }

    /// Validates a path for write operations (stricter than read)
    pub fn validate_write(&self, path: &Path) -> AgentResult<PathBuf> {
        let normalized = self.normalize_and_canonicalize(path)?;

        // Write operations MUST be within sandbox
        if !self.is_within_sandbox(&normalized) {
            return Err(AgentError::SandboxViolation(format!(
                "Write operation outside sandbox: {}",
                normalized.display()
            )));
        }

        Ok(normalized)
    }

    /// Validates multiple paths for read operations
    pub fn validate_reads(&self, paths: &[PathBuf]) -> AgentResult<Vec<PathBuf>> {
        if paths.len() > self.config.max_batch_size {
            return Err(AgentError::InvalidInput(format!(
                "Batch size {} exceeds maximum {}",
                paths.len(),
                self.config.max_batch_size
            )));
        }

        paths.iter().map(|p| self.validate_read(p)).collect()
    }

    /// Validates a file size for read operations
    pub fn validate_file_size(&self, path: &Path) -> AgentResult<u64> {
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();

        if size > self.config.max_read_size as u64 {
            return Err(AgentError::InvalidInput(format!(
                "File size {} exceeds maximum {}",
                size, self.config.max_read_size
            )));
        }

        Ok(size)
    }

    /// Checks if path is within sandbox
    fn is_within_sandbox(&self, path: &Path) -> bool {
        path.starts_with(&self.config.root)
    }

    /// Normalizes and canonicalizes a path
    fn normalize_and_canonicalize(&self, path: &Path) -> AgentResult<PathBuf> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AgentError::PathError("Path contains invalid UTF-8".to_string()))?;

        // First normalize the path (handle WSL, Git Bash, etc.)
        let normalized_str = normalize_path(path_str)?;
        let normalized = PathBuf::from(normalized_str);

        // If path is relative, make it absolute relative to sandbox root
        let absolute = if !is_absolute(path_str) {
            self.config.root.join(&normalized)
        } else {
            normalized
        };

        // Canonicalize to resolve symlinks and .. components
        // Note: This will fail if the path doesn't exist, which is fine for write validation
        match absolute.canonicalize() {
            Ok(canonical) => Ok(canonical),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // For write operations, the file might not exist yet
                // Try to canonicalize the parent directory instead
                if let Some(parent) = absolute.parent() {
                    let canonical_parent = parent.canonicalize().map_err(|_| {
                        AgentError::PathError(format!(
                            "Parent directory does not exist: {}",
                            parent.display()
                        ))
                    })?;

                    if let Some(filename) = absolute.file_name() {
                        Ok(canonical_parent.join(filename))
                    } else {
                        Ok(canonical_parent)
                    }
                } else {
                    // No parent means this is a root directory
                    Ok(absolute)
                }
            }
            Err(e) => Err(AgentError::from(e)),
        }
    }

    /// Gets the sandbox root
    pub fn root(&self) -> &Path {
        &self.config.root
    }

    /// Gets the full config
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

/// Checks if a filename is safe (doesn't contain path traversal)
pub fn is_safe_filename(filename: &str) -> bool {
    !filename.contains("..") && !filename.contains('/') && !filename.contains('\\')
}

/// Validates that a path doesn't attempt traversal
pub fn validate_no_traversal(path: &Path) -> AgentResult<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| AgentError::PathError("Path contains invalid UTF-8".to_string()))?;

    if path_str.contains("..") {
        return Err(AgentError::SandboxViolation(
            "Path contains traversal component '..'".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn create_test_sandbox() -> Sandbox {
        let temp_dir = env::temp_dir().join("agent_tools_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        Sandbox::new(SandboxConfig::new(temp_dir))
    }

    #[test]
    fn test_safe_filename() {
        assert!(is_safe_filename("file.txt"));
        assert!(is_safe_filename("my-document.pdf"));
        assert!(!is_safe_filename("..\\file.txt"));
        assert!(!is_safe_filename("../file.txt"));
        assert!(!is_safe_filename("dir/file.txt"));
    }

    #[test]
    fn test_validate_no_traversal() {
        assert!(validate_no_traversal(Path::new("file.txt")).is_ok());
        assert!(validate_no_traversal(Path::new("dir/file.txt")).is_ok());
        assert!(validate_no_traversal(Path::new("../file.txt")).is_err());
        assert!(validate_no_traversal(Path::new("dir/../file.txt")).is_err());
    }

    #[test]
    fn test_sandbox_within_bounds() {
        let sandbox = create_test_sandbox();
        let test_file = sandbox.root().join("test.txt");

        // Create test file
        std::fs::write(&test_file, b"test").unwrap();

        // Should succeed - file is within sandbox
        assert!(sandbox.validate_read(&test_file).is_ok());
        assert!(sandbox.validate_write(&test_file).is_ok());

        // Clean up
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_sandbox_outside_bounds() {
        let sandbox = create_test_sandbox();
        let outside_file = PathBuf::from("/tmp/outside.txt");

        // Should fail for read without allow_read_outside
        assert!(sandbox.validate_read(&outside_file).is_err());

        // Should always fail for write
        assert!(sandbox.validate_write(&outside_file).is_err());
    }

    #[test]
    fn test_sandbox_allow_read_outside() {
        let config = SandboxConfig::new(env::temp_dir()).allow_read_outside(true);
        let sandbox = Sandbox::new(config);

        // Create a file outside sandbox
        let outside_file = env::temp_dir()
            .parent()
            .unwrap()
            .join("test_read_outside.txt");
        std::fs::write(&outside_file, b"test").unwrap();

        // Should succeed for read with allow_read_outside
        assert!(sandbox.validate_read(&outside_file).is_ok());

        // Should still fail for write
        assert!(sandbox.validate_write(&outside_file).is_err());

        // Clean up
        std::fs::remove_file(&outside_file).ok();
    }

    #[test]
    fn test_relative_path_handling() {
        let sandbox = create_test_sandbox();
        let relative_path = Path::new("relative/file.txt");

        // Relative paths should be resolved relative to sandbox root
        let resolved = sandbox.validate_write(relative_path);
        assert!(resolved.is_ok());

        if let Ok(path) = resolved {
            assert!(path.starts_with(sandbox.root()));
        }
    }
}
