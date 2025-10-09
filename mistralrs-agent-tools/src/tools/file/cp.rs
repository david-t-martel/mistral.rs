//! Cp utility - copy files and directories
//!
//! Copies files and directories with various options.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Options for cp operation
#[derive(Debug, Clone, Default)]
pub struct CpOptions {
    /// Copy directories recursively (-r, -R, --recursive)
    pub recursive: bool,
    /// Force overwrite existing files (-f, --force)
    pub force: bool,
    /// Interactive mode - prompt before overwrite (-i, --interactive)
    pub interactive: bool,
    /// Preserve file attributes (mode, ownership, timestamps) (-p, --preserve)
    pub preserve: bool,
    /// Create hard links instead of copying (-l, --link)
    pub link: bool,
    /// Create symbolic links instead of copying (-s, --symbolic-link)
    pub symbolic_link: bool,
    /// Update - copy only when source is newer (-u, --update)
    pub update: bool,
    /// Verbose output (-v, --verbose)
    pub verbose: bool,
}

/// Result of cp operation
#[derive(Debug, Clone)]
pub struct CpResult {
    /// Paths that were copied (destination paths)
    pub copied: Vec<String>,
    /// Number of files copied
    pub count: usize,
    /// Total bytes copied
    pub bytes_copied: u64,
}

/// Copy files or directories
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `sources` - Source paths to copy
/// * `dest` - Destination path
/// * `options` - Copy options
///
/// # Returns
/// Result containing copied file information
///
/// # Errors
/// Returns error if:
/// - Path is outside sandbox
/// - Source doesn't exist
/// - Permission denied
/// - Invalid path
/// - Cannot copy directory without recursive flag
pub fn cp(
    sandbox: &Sandbox,
    sources: &[&Path],
    dest: &Path,
    options: &CpOptions,
) -> AgentResult<CpResult> {
    if sources.is_empty() {
        return Err(AgentError::validation("No source paths specified for cp"));
    }

    // Validate destination path
    let validated_dest = sandbox.validate_write(dest)?;

    let mut copied = Vec::new();
    let mut bytes_copied = 0u64;

    // Determine if destination is a directory
    let dest_is_dir = validated_dest.exists() && validated_dest.is_dir();

    // If multiple sources, destination must be a directory
    if sources.len() > 1 && !dest_is_dir {
        return Err(AgentError::validation(
            "When copying multiple sources, destination must be a directory",
        ));
    }

    for source in sources {
        // Validate source path
        let validated_source = sandbox.validate_read(source)?;

        if !validated_source.exists() {
            return Err(AgentError::io(format!(
                "Source does not exist: {}",
                validated_source.display()
            )));
        }

        // Determine final destination path
        let final_dest = if dest_is_dir {
            let file_name = validated_source
                .file_name()
                .ok_or_else(|| AgentError::validation("Invalid source path"))?;
            validated_dest.join(file_name)
        } else {
            validated_dest.clone()
        };

        // Check if we should skip (update mode)
        if options.update && should_skip_update(&validated_source, &final_dest)? {
            continue;
        }

        // Check if destination exists and handle accordingly
        if final_dest.exists() && !options.force {
            if !options.interactive {
                return Err(AgentError::io(format!(
                    "Destination exists and force flag not set: {}",
                    final_dest.display()
                )));
            }
            // In a real implementation, we'd prompt here
            // For agent tools, we'll skip in interactive mode without force
            continue;
        }

        // Perform the copy
        let bytes = if options.symbolic_link {
            create_symlink(&validated_source, &final_dest)?;
            0
        } else if options.link {
            create_hardlink(&validated_source, &final_dest)?;
            0
        } else if validated_source.is_dir() {
            if !options.recursive {
                return Err(AgentError::validation(format!(
                    "Cannot copy directory {} without --recursive flag",
                    validated_source.display()
                )));
            }
            copy_dir_recursive(&validated_source, &final_dest, options)?
        } else {
            copy_file(&validated_source, &final_dest, options)?
        };

        bytes_copied += bytes;

        if options.verbose {
            eprintln!(
                "cp: copied '{}' -> '{}'",
                validated_source.display(),
                final_dest.display()
            );
        }

        let dest_str = final_dest
            .to_str()
            .ok_or_else(|| AgentError::validation("Destination path contains invalid UTF-8"))?
            .to_string();

        copied.push(dest_str);
    }

    Ok(CpResult {
        count: copied.len(),
        bytes_copied,
        copied,
    })
}

/// Copy a single file
fn copy_file(source: &Path, dest: &Path, options: &CpOptions) -> AgentResult<u64> {
    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                AgentError::io(format!(
                    "Failed to create parent directory for {}: {}",
                    dest.display(),
                    e
                ))
            })?;
        }
    }

    // Copy the file
    let bytes = fs::copy(source, dest).map_err(|e| {
        AgentError::io(format!(
            "Failed to copy {} to {}: {}",
            source.display(),
            dest.display(),
            e
        ))
    })?;

    // Preserve attributes if requested
    if options.preserve {
        preserve_attributes(source, dest)?;
    }

    Ok(bytes)
}

/// Copy a directory recursively
fn copy_dir_recursive(source: &Path, dest: &Path, options: &CpOptions) -> AgentResult<u64> {
    // Create destination directory
    fs::create_dir_all(dest).map_err(|e| {
        AgentError::io(format!(
            "Failed to create directory {}: {}",
            dest.display(),
            e
        ))
    })?;

    let mut total_bytes = 0u64;

    // Iterate through directory entries
    for entry_result in fs::read_dir(source).map_err(|e| {
        AgentError::io(format!(
            "Failed to read directory {}: {}",
            source.display(),
            e
        ))
    })? {
        let entry = entry_result.map_err(|e| {
            AgentError::io(format!("Failed to read directory entry: {}", e))
        })?;

        let source_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        if source_path.is_dir() {
            total_bytes += copy_dir_recursive(&source_path, &dest_path, options)?;
        } else {
            total_bytes += copy_file(&source_path, &dest_path, options)?;
        }
    }

    // Preserve directory attributes if requested
    if options.preserve {
        preserve_attributes(source, dest)?;
    }

    Ok(total_bytes)
}

/// Preserve file attributes (mode, timestamps)
fn preserve_attributes(source: &Path, dest: &Path) -> AgentResult<()> {
    let source_metadata = fs::metadata(source).map_err(|e| {
        AgentError::io(format!(
            "Failed to get metadata for {}: {}",
            source.display(),
            e
        ))
    })?;

    // Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = source_metadata.permissions();
        fs::set_permissions(dest, permissions).map_err(|e| {
            AgentError::io(format!(
                "Failed to set permissions on {}: {}",
                dest.display(),
                e
            ))
        })?;
    }

    // Note: Preserving timestamps requires platform-specific code
    // Production implementation should use the `filetime` crate

    Ok(())
}

/// Create a symbolic link
fn create_symlink(source: &Path, dest: &Path) -> AgentResult<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, dest).map_err(|e| {
            AgentError::io(format!(
                "Failed to create symbolic link from {} to {}: {}",
                source.display(),
                dest.display(),
                e
            ))
        })?;
    }

    #[cfg(windows)]
    {
        if source.is_dir() {
            std::os::windows::fs::symlink_dir(source, dest).map_err(|e| {
                AgentError::io(format!(
                    "Failed to create directory symbolic link from {} to {}: {}",
                    source.display(),
                    dest.display(),
                    e
                ))
            })?;
        } else {
            std::os::windows::fs::symlink_file(source, dest).map_err(|e| {
                AgentError::io(format!(
                    "Failed to create file symbolic link from {} to {}: {}",
                    source.display(),
                    dest.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

/// Create a hard link
fn create_hardlink(source: &Path, dest: &Path) -> AgentResult<()> {
    fs::hard_link(source, dest).map_err(|e| {
        AgentError::io(format!(
            "Failed to create hard link from {} to {}: {}",
            source.display(),
            dest.display(),
            e
        ))
    })?;

    Ok(())
}

/// Check if file should be skipped in update mode
fn should_skip_update(source: &Path, dest: &Path) -> AgentResult<bool> {
    if !dest.exists() {
        return Ok(false);
    }

    let source_metadata = fs::metadata(source).map_err(|e| {
        AgentError::io(format!(
            "Failed to get metadata for {}: {}",
            source.display(),
            e
        ))
    })?;

    let dest_metadata = fs::metadata(dest).map_err(|e| {
        AgentError::io(format!(
            "Failed to get metadata for {}: {}",
            dest.display(),
            e
        ))
    })?;

    let source_mtime = source_metadata.modified().ok();
    let dest_mtime = dest_metadata.modified().ok();

    match (source_mtime, dest_mtime) {
        (Some(src), Some(dst)) => Ok(src <= dst), // Skip if source is not newer
        _ => Ok(false), // If we can't determine, don't skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::sandbox::{Sandbox, SandboxConfig};
    use tempfile::TempDir;

    #[test]
    fn test_cp_single_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "test content").expect("Failed to create source file");

        let result = cp(&sandbox, &[&source], &dest, &CpOptions::default());

        assert!(result.is_ok());
        let result = result.expect("cp failed");
        assert_eq!(result.count, 1);
        assert!(result.bytes_copied > 0);
        assert!(dest.exists());

        let content = fs::read_to_string(&dest).expect("Failed to read dest file");
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_cp_directory_recursive() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let source_dir = temp_dir.path().join("source_dir");
        let dest_dir = temp_dir.path().join("dest_dir");

        fs::create_dir(&source_dir).expect("Failed to create source dir");
        fs::write(source_dir.join("file1.txt"), "content1").expect("Failed to create file");
        fs::write(source_dir.join("file2.txt"), "content2").expect("Failed to create file");

        let options = CpOptions {
            recursive: true,
            ..Default::default()
        };

        let result = cp(&sandbox, &[&source_dir], &dest_dir, &options);

        assert!(result.is_ok());
        assert!(dest_dir.exists());
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
    }

    #[test]
    fn test_cp_directory_without_recursive_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let source_dir = temp_dir.path().join("source_dir");
        let dest_dir = temp_dir.path().join("dest_dir");

        fs::create_dir(&source_dir).expect("Failed to create source dir");

        let result = cp(&sandbox, &[&source_dir], &dest_dir, &CpOptions::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_cp_multiple_to_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let dest_dir = temp_dir.path().join("dest_dir");

        fs::write(&file1, "content1").expect("Failed to create file1");
        fs::write(&file2, "content2").expect("Failed to create file2");
        fs::create_dir(&dest_dir).expect("Failed to create dest dir");

        let result = cp(&sandbox, &[&file1, &file2], &dest_dir, &CpOptions::default());

        assert!(result.is_ok());
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
    }

    #[test]
    fn test_cp_force_overwrite() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "new content").expect("Failed to create source");
        fs::write(&dest, "old content").expect("Failed to create dest");

        let options = CpOptions {
            force: true,
            ..Default::default()
        };

        let result = cp(&sandbox, &[&source], &dest, &options);

        assert!(result.is_ok());

        let content = fs::read_to_string(&dest).expect("Failed to read dest");
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_cp_without_force_fails_if_exists() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));

        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "content").expect("Failed to create source");
        fs::write(&dest, "existing").expect("Failed to create dest");

        let result = cp(&sandbox, &[&source], &dest, &CpOptions::default());

        assert!(result.is_err());
    }
}
