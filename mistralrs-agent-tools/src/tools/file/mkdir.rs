//! Mkdir utility - create directories
//!
//! Creates one or more directories with optional parent directory creation.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::fs;
use std::path::Path;

/// Options for mkdir operation
#[derive(Debug, Clone, Default)]
pub struct MkdirOptions {
    /// Create parent directories as needed (-p, --parents)
    pub parents: bool,
    /// Set file mode (permissions) - Unix only
    pub mode: Option<u32>,
    /// Verbose output
    pub verbose: bool,
}

/// Result of mkdir operation
#[derive(Debug, Clone)]
pub struct MkdirResult {
    /// Paths that were created
    pub created: Vec<String>,
    /// Number of directories created
    pub count: usize,
}

/// Create directories
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Paths to create
/// * `options` - Mkdir options
///
/// # Returns
/// Result containing created directory paths
///
/// # Errors
/// Returns error if:
/// - Path is outside sandbox
/// - Directory already exists (unless parents=true)
/// - Permission denied
/// - Invalid path
pub fn mkdir(
    sandbox: &Sandbox,
    paths: &[&Path],
    options: &MkdirOptions,
) -> AgentResult<MkdirResult> {
    if paths.is_empty() {
        return Err(AgentError::validation("No paths specified for mkdir"));
    }

    let mut created = Vec::new();

    for path in paths {
        // Validate path through sandbox (write permission required)
        let validated_path = sandbox.validate_write(path)?;

        // Check if directory already exists
        if validated_path.exists() {
            if !options.parents {
                return Err(AgentError::io(format!(
                    "Directory already exists: {}",
                    validated_path.display()
                )));
            }
            // With --parents flag, ignore existing directories
            continue;
        }

        // Create directory (with or without parents)
        if options.parents {
            fs::create_dir_all(&validated_path).map_err(|e| {
                AgentError::io(format!(
                    "Failed to create directory {} with parents: {}",
                    validated_path.display(),
                    e
                ))
            })?;
        } else {
            fs::create_dir(&validated_path).map_err(|e| {
                AgentError::io(format!(
                    "Failed to create directory {}: {}",
                    validated_path.display(),
                    e
                ))
            })?;
        }

        // Set permissions if specified (Unix only)
        #[cfg(unix)]
        if let Some(mode) = options.mode {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(mode);
            fs::set_permissions(&validated_path, permissions).map_err(|e| {
                AgentError::io(format!(
                    "Failed to set permissions on {}: {}",
                    validated_path.display(),
                    e
                ))
            })?;
        }

        let path_str = validated_path
            .to_str()
            .ok_or_else(|| AgentError::validation("Path contains invalid UTF-8"))?
            .to_string();

        if options.verbose {
            eprintln!("mkdir: created directory '{}'", path_str);
        }

        created.push(path_str);
    }

    Ok(MkdirResult {
        count: created.len(),
        created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::sandbox::{Sandbox, SandboxConfig};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_mkdir_single() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let new_dir = temp_dir.path().join("test_dir");
        let result = mkdir(&sandbox, &[&new_dir], &MkdirOptions::default());

        assert!(result.is_ok());
        let result = result.expect("mkdir failed");
        assert_eq!(result.count, 1);
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_mkdir_multiple() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        let dir3 = temp_dir.path().join("dir3");

        let result = mkdir(&sandbox, &[&dir1, &dir2, &dir3], &MkdirOptions::default());

        assert!(result.is_ok());
        let result = result.expect("mkdir failed");
        assert_eq!(result.count, 3);
        assert!(dir1.exists() && dir1.is_dir());
        assert!(dir2.exists() && dir2.is_dir());
        assert!(dir3.exists() && dir3.is_dir());
    }

    #[test]
    fn test_mkdir_with_parents() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let nested_dir = temp_dir.path().join("parent").join("child").join("grandchild");

        let options = MkdirOptions {
            parents: true,
            ..Default::default()
        };

        let result = mkdir(&sandbox, &[&nested_dir], &options);

        assert!(result.is_ok());
        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());
        assert!(temp_dir.path().join("parent").exists());
        assert!(temp_dir.path().join("parent").join("child").exists());
    }

    #[test]
    fn test_mkdir_without_parents_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let nested_dir = temp_dir.path().join("parent").join("child");

        let result = mkdir(&sandbox, &[&nested_dir], &MkdirOptions::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_mkdir_existing_with_parents() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let dir = temp_dir.path().join("existing");
        fs::create_dir(&dir).expect("Failed to create test dir");

        let options = MkdirOptions {
            parents: true,
            ..Default::default()
        };

        // Should succeed with --parents flag
        let result = mkdir(&sandbox, &[&dir], &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mkdir_existing_without_parents() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let dir = temp_dir.path().join("existing");
        fs::create_dir(&dir).expect("Failed to create test dir");

        // Should fail without --parents flag
        let result = mkdir(&sandbox, &[&dir], &MkdirOptions::default());
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_mkdir_with_mode() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let dir = temp_dir.path().join("mode_test");

        let options = MkdirOptions {
            mode: Some(0o755),
            ..Default::default()
        };

        let result = mkdir(&sandbox, &[&dir], &options);
        assert!(result.is_ok());
        assert!(dir.exists());

        // Check permissions
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&dir).expect("Failed to get metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }

    #[test]
    fn test_mkdir_outside_sandbox() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let outside_dir = PathBuf::from("/tmp/outside_sandbox");

        let result = mkdir(&sandbox, &[&outside_dir], &MkdirOptions::default());
        assert!(result.is_err());
    }
}
