//! Ls utility - list directory contents
//!
//! Simplified implementation for agent tools.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentResult, FileEntry, LsOptions, LsResult};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// List directory contents
pub fn ls(sandbox: &Sandbox, path: &Path, options: &LsOptions) -> AgentResult<LsResult> {
    // Validate path through sandbox
    let validated_path = sandbox.validate_read(path)?;

    // Check if path is a directory
    let metadata = fs::metadata(&validated_path)?;
    if !metadata.is_dir() {
        // If it's a file, just return that file's info
        let entry = create_file_entry(&validated_path)?;
        return Ok(LsResult {
            entries: vec![entry.clone()],
            total: 1,
            total_size: entry.size,
        });
    }

    // Read directory
    let mut entries = Vec::new();

    if options.recursive {
        collect_recursive(&validated_path, &mut entries, options, sandbox)?;
    } else {
        collect_dir(&validated_path, &mut entries, options)?;
    }

    // Sort entries
    sort_entries(&mut entries, options);

    // Calculate totals
    let total = entries.len();
    let total_size = entries.iter().map(|e| e.size).sum();

    Ok(LsResult {
        entries,
        total,
        total_size,
    })
}

/// Collect entries from a single directory
fn collect_dir(
    dir_path: &Path,
    entries: &mut Vec<FileEntry>,
    options: &LsOptions,
) -> AgentResult<()> {
    let dir_entries = fs::read_dir(dir_path)?;

    for entry_result in dir_entries {
        let entry = entry_result?;
        let path = entry.path();

        // Skip hidden files unless --all is specified
        if !options.all {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with('.') {
                        continue;
                    }
                }
            }
        }

        entries.push(create_file_entry(&path)?);
    }

    Ok(())
}

/// Collect entries recursively
fn collect_recursive(
    dir_path: &Path,
    entries: &mut Vec<FileEntry>,
    options: &LsOptions,
    sandbox: &Sandbox,
) -> AgentResult<()> {
    use std::collections::HashSet;
    let mut visited = HashSet::new();
    collect_recursive_impl(dir_path, entries, options, sandbox, &mut visited)
}

/// Internal recursive implementation with visited tracking
fn collect_recursive_impl(
    dir_path: &Path,
    entries: &mut Vec<FileEntry>,
    options: &LsOptions,
    sandbox: &Sandbox,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> AgentResult<()> {
    // Canonicalize path to handle symlinks
    let canonical = dir_path
        .canonicalize()
        .unwrap_or_else(|_| dir_path.to_path_buf());

    // Prevent infinite recursion
    if visited.contains(&canonical) {
        return Ok(());
    }
    visited.insert(canonical);

    // Store current entry count
    let start_idx = entries.len();

    collect_dir(dir_path, entries, options)?;

    // Get subdirectories from newly added entries only
    let subdirs: Vec<_> = entries[start_idx..]
        .iter()
        .filter(|e| e.is_dir)
        .map(|e| e.path.clone())
        .collect();

    // Recurse into subdirectories
    for subdir in subdirs {
        // Validate subdirectory is still in sandbox
        if sandbox.validate_read(&subdir).is_ok() {
            collect_recursive_impl(&subdir, entries, options, sandbox, visited)?;
        }
    }

    Ok(())
}

/// Create a FileEntry from a path
fn create_file_entry(path: &Path) -> AgentResult<FileEntry> {
    let metadata = fs::metadata(path)?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Get permissions (Unix-style, 0 on Windows)
    let permissions = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        }
        #[cfg(not(unix))]
        {
            0
        }
    };

    Ok(FileEntry {
        path: path.to_path_buf(),
        name,
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified,
        permissions,
    })
}

/// Sort entries based on options
fn sort_entries(entries: &mut [FileEntry], options: &LsOptions) {
    if options.sort_by_time {
        entries.sort_by(|a, b| {
            let cmp = b.modified.cmp(&a.modified); // Newest first
            if options.reverse {
                cmp.reverse()
            } else {
                cmp
            }
        });
    } else {
        // Sort by name
        entries.sort_by(|a, b| {
            let cmp = a.name.cmp(&b.name);
            if options.reverse {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }
}

/// Format file size as human-readable
pub fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }

    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut size_f = size as f64;
    let mut unit_idx = 0;

    while size_f >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}{}", size, UNITS[unit_idx])
    } else {
        format!("{:.1}{}", size_f, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SandboxConfig;
    use std::fs::File;
    use tempfile::TempDir;

    fn create_test_sandbox() -> (Sandbox, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));
        (sandbox, temp_dir)
    }

    #[test]
    fn test_ls_empty_directory() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let options = LsOptions::default();
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.entries.len(), 0);
    }

    #[test]
    fn test_ls_with_files() {
        let (sandbox, temp_dir) = create_test_sandbox();

        // Create test files
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        File::create(temp_dir.path().join("file2.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let options = LsOptions::default();
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.entries.len(), 3);
    }

    #[test]
    fn test_ls_hidden_files() {
        let (sandbox, temp_dir) = create_test_sandbox();

        // Create visible and hidden files
        File::create(temp_dir.path().join("visible.txt")).unwrap();
        File::create(temp_dir.path().join(".hidden.txt")).unwrap();

        // Without --all, should not see hidden file
        let options = LsOptions::default();
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();
        assert_eq!(result.total, 1);

        // With --all, should see both
        let options = LsOptions {
            all: true,
            ..Default::default()
        };
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();
        assert_eq!(result.total, 2);
    }

    #[test]
    fn test_ls_recursive() {
        let (sandbox, temp_dir) = create_test_sandbox();

        // Create nested structure
        fs::create_dir(temp_dir.path().join("dir1")).unwrap();
        File::create(temp_dir.path().join("dir1/file1.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("dir1/dir2")).unwrap();
        File::create(temp_dir.path().join("dir1/dir2/file2.txt")).unwrap();

        let options = LsOptions {
            recursive: true,
            ..Default::default()
        };
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();

        // Should find: dir1, dir1/file1.txt, dir1/dir2, dir1/dir2/file2.txt
        assert!(result.total >= 4);
    }

    #[test]
    fn test_ls_sort_by_name() {
        let (sandbox, temp_dir) = create_test_sandbox();

        File::create(temp_dir.path().join("zebra.txt")).unwrap();
        File::create(temp_dir.path().join("apple.txt")).unwrap();
        File::create(temp_dir.path().join("banana.txt")).unwrap();

        let options = LsOptions::default();
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();

        assert_eq!(result.entries[0].name, "apple.txt");
        assert_eq!(result.entries[1].name, "banana.txt");
        assert_eq!(result.entries[2].name, "zebra.txt");
    }

    #[test]
    fn test_ls_reverse_sort() {
        let (sandbox, temp_dir) = create_test_sandbox();

        File::create(temp_dir.path().join("a.txt")).unwrap();
        File::create(temp_dir.path().join("b.txt")).unwrap();
        File::create(temp_dir.path().join("c.txt")).unwrap();

        let options = LsOptions {
            reverse: true,
            ..Default::default()
        };
        let result = ls(&sandbox, temp_dir.path(), &options).unwrap();

        assert_eq!(result.entries[0].name, "c.txt");
        assert_eq!(result.entries[1].name, "b.txt");
        assert_eq!(result.entries[2].name, "a.txt");
    }

    #[test]
    fn test_ls_single_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("single.txt");
        File::create(&file_path).unwrap();

        let options = LsOptions::default();
        let result = ls(&sandbox, &file_path, &options).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.entries[0].name, "single.txt");
        assert!(!result.entries[0].is_dir);
    }

    #[test]
    fn test_format_size_human_readable() {
        assert_eq!(format_size(100, true), "100B");
        assert_eq!(format_size(1024, true), "1.0K");
        assert_eq!(format_size(1536, true), "1.5K");
        assert_eq!(format_size(1024 * 1024, true), "1.0M");
        assert_eq!(format_size(1024 * 1024 * 1024, true), "1.0G");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(100, false), "100");
        assert_eq!(format_size(1024, false), "1024");
        assert_eq!(format_size(1048576, false), "1048576");
    }

    #[test]
    fn test_ls_sandbox_violation() {
        let (sandbox, _temp_dir) = create_test_sandbox();
        let outside_path = Path::new("/tmp/outside");

        let options = LsOptions::default();
        let result = ls(&sandbox, outside_path, &options);

        assert!(result.is_err());
    }
}
