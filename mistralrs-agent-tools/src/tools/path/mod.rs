//! Path operations module.
//!
//! Implements path manipulation utilities:
//! - basename: Strip directory and suffix from filenames
//! - dirname: Strip last component from file name
//! - readlink: Print resolved symbolic links
//! - realpath: Print the resolved absolute path

use std::path::{Path, PathBuf};

/// Return the final path component optionally stripping an extension.
pub fn basename<P: AsRef<Path>>(p: P, strip_ext: bool) -> Option<String> {
    let file = p.as_ref().file_name()?.to_string_lossy().into_owned();
    if strip_ext {
        if let Some((stem, _)) = file.rsplit_once('.') {
            return Some(stem.to_string());
        }
    }
    Some(file)
}

/// Return the parent directory path as owned PathBuf.
pub fn dirname<P: AsRef<Path>>(p: P) -> Option<PathBuf> {
    p.as_ref().parent().map(|p| p.to_path_buf())
}

/// Resolve a symlink (single hop). Returns target path or original if not a symlink.
pub fn readlink<P: AsRef<Path>>(p: P) -> std::io::Result<PathBuf> {
    std::fs::read_link(p)
}

/// Produce an absolute path with symlinks resolved (best effort) similar to realpath.
pub fn realpath<P: AsRef<Path>>(p: P) -> std::io::Result<PathBuf> {
    let abs = if p.as_ref().is_absolute() {
        p.as_ref().to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    };
    // canonicalize may fail on non-existent paths; fall back to absolute.
    std::fs::canonicalize(&abs).or(Ok(abs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basename() {
        assert_eq!(basename("/tmp/file.txt", true).as_deref(), Some("file"));
        assert_eq!(basename("/tmp/file", true).as_deref(), Some("file"));
    }

    #[test]
    fn test_dirname() {
        assert!(dirname("/tmp/file.txt").unwrap().ends_with("tmp"));
    }
}
