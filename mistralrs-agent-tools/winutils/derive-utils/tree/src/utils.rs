use anyhow::Result;
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};

use crate::windows::{AlternateDataStreams, FileAttributes, ReparsePointInfo};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_executable: bool,
    pub size: Option<u64>,
    pub modified_time: Option<String>,
    pub attributes: Option<FileAttributes>,
    pub reparse_info: Option<ReparsePointInfo>,
    pub alternate_streams: Option<AlternateDataStreams>,
}

impl FileInfo {
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = std::fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();

        let is_dir = file_type.is_dir();
        let is_symlink = file_type.is_symlink();
        let size = if is_dir { None } else { Some(metadata.len()) };

        // Format modification time
        let modified_time = metadata
            .modified()
            .ok()
            .and_then(|time| {
                let datetime: DateTime<Local> = time.into();
                Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
            });

        // Get Windows-specific information
        let attributes = if cfg!(windows) {
            FileAttributes::from_path(path).ok()
        } else {
            None
        };

        let reparse_info = if cfg!(windows) {
            ReparsePointInfo::from_path(path).unwrap_or(None)
        } else {
            None
        };

        let alternate_streams = if cfg!(windows) {
            AlternateDataStreams::from_path(path).ok()
        } else {
            None
        };

        // Check if file is executable
        let is_executable = Self::is_executable(path, &metadata);

        Ok(Self {
            path: path.to_owned(),
            is_dir,
            is_symlink,
            is_executable,
            size,
            modified_time,
            attributes,
            reparse_info,
            alternate_streams,
        })
    }

    #[cfg(windows)]
    fn is_executable(path: &Path, _metadata: &std::fs::Metadata) -> bool {
        // On Windows, check file extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "exe" | "bat" | "cmd" | "com" | "scr" | "msi"
            )
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    fn is_executable(_path: &Path, metadata: &std::fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    /// Get file extension if available
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Get file name without extension
    pub fn stem(&self) -> Option<&str> {
        self.path.file_stem().and_then(|s| s.to_str())
    }

    /// Check if this is a hidden file (Windows-specific or Unix dot files)
    pub fn is_hidden(&self) -> bool {
        // Check Windows attributes first
        if let Some(ref attrs) = self.attributes {
            if attrs.hidden {
                return true;
            }
        }

        // Check for Unix-style hidden files (starting with .)
        if let Some(name) = self.path.file_name().and_then(|n| n.to_str()) {
            name.starts_with('.')
        } else {
            false
        }
    }

    /// Check if this is a system file
    pub fn is_system(&self) -> bool {
        self.attributes.as_ref().map_or(false, |a| a.system)
    }

    /// Get human-readable file size
    pub fn size_string(&self) -> String {
        match self.size {
            Some(size) => humansize::format_size(size, humansize::BINARY),
            None => if self.is_dir { "<DIR>".to_string() } else { "0".to_string() },
        }
    }

    /// Get Windows attributes as a descriptive string
    pub fn attributes_description(&self) -> Option<String> {
        self.attributes.as_ref().map(|attrs| {
            let descriptions = attrs.to_description();
            if descriptions.is_empty() {
                "Normal".to_string()
            } else {
                descriptions.join(", ")
            }
        })
    }

    /// Check if file matches a pattern (simple glob matching)
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let filename = self.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        glob_match(pattern, filename)
    }

    /// Get the file's parent directory
    pub fn parent(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// Get depth relative to a root path
    pub fn depth_from(&self, root: &Path) -> Option<usize> {
        self.path.strip_prefix(root)
            .ok()
            .map(|relative| relative.components().count())
    }
}

/// Simple glob pattern matching
/// Supports * (match any characters) and ? (match single character)
pub fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_recursive(pattern.chars().collect(), text.chars().collect(), 0, 0)
}

fn glob_match_recursive(pattern: Vec<char>, text: Vec<char>, p_idx: usize, t_idx: usize) -> bool {
    // End of both pattern and text
    if p_idx >= pattern.len() && t_idx >= text.len() {
        return true;
    }

    // End of pattern but not text
    if p_idx >= pattern.len() {
        return false;
    }

    // Handle * wildcard
    if pattern[p_idx] == '*' {
        // Try matching zero characters
        if glob_match_recursive(pattern.clone(), text.clone(), p_idx + 1, t_idx) {
            return true;
        }

        // Try matching one or more characters
        for i in t_idx..text.len() {
            if glob_match_recursive(pattern.clone(), text.clone(), p_idx + 1, i + 1) {
                return true;
            }
        }

        return false;
    }

    // End of text but not pattern
    if t_idx >= text.len() {
        return false;
    }

    // Handle ? wildcard or exact character match
    if pattern[p_idx] == '?' || pattern[p_idx] == text[t_idx] {
        return glob_match_recursive(pattern, text, p_idx + 1, t_idx + 1);
    }

    false
}

/// Format a duration in a human-readable way
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs_f64();

    if total_secs < 1.0 {
        format!("{:.0}ms", duration.as_millis())
    } else if total_secs < 60.0 {
        format!("{:.2}s", total_secs)
    } else if total_secs < 3600.0 {
        let mins = (total_secs / 60.0) as u32;
        let secs = total_secs % 60.0;
        format!("{}m {:.1}s", mins, secs)
    } else {
        let hours = (total_secs / 3600.0) as u32;
        let mins = ((total_secs % 3600.0) / 60.0) as u32;
        format!("{}h {}m", hours, mins)
    }
}

/// Calculate directory statistics
#[derive(Debug, Default)]
pub struct DirectoryStats {
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_size: u64,
    pub max_depth: usize,
    pub largest_file: Option<(PathBuf, u64)>,
    pub file_extensions: std::collections::HashMap<String, usize>,
}

impl DirectoryStats {
    pub fn calculate(root: &Path, max_depth: Option<usize>) -> Result<Self> {
        let mut stats = Self::default();
        Self::calculate_recursive(root, 0, max_depth, &mut stats)?;
        Ok(stats)
    }

    fn calculate_recursive(
        path: &Path,
        depth: usize,
        max_depth: Option<usize>,
        stats: &mut DirectoryStats,
    ) -> Result<()> {
        if let Some(max) = max_depth {
            if depth > max {
                return Ok(());
            }
        }

        stats.max_depth = stats.max_depth.max(depth);

        let metadata = std::fs::symlink_metadata(path)?;

        if metadata.is_dir() {
            stats.total_dirs += 1;

            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    Self::calculate_recursive(&entry.path(), depth + 1, max_depth, stats)?;
                }
            }
        } else {
            stats.total_files += 1;
            let size = metadata.len();
            stats.total_size += size;

            // Track largest file
            if stats.largest_file.as_ref().map_or(true, |(_, s)| size > *s) {
                stats.largest_file = Some((path.to_owned(), size));
            }

            // Track file extensions
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                *stats.file_extensions.entry(ext).or_insert(0) += 1;
            }
        }

        Ok(())
    }

    /// Get the top N file extensions by count
    pub fn top_extensions(&self, n: usize) -> Vec<(String, usize)> {
        let mut extensions: Vec<_> = self.file_extensions.iter()
            .map(|(ext, count)| (ext.clone(), *count))
            .collect();

        extensions.sort_by(|a, b| b.1.cmp(&a.1));
        extensions.truncate(n);
        extensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("*.txt", "test.txt"));
        assert!(!glob_match("*.txt", "file.doc"));

        assert!(glob_match("test?", "test1"));
        assert!(glob_match("test?", "testa"));
        assert!(!glob_match("test?", "test12"));

        assert!(glob_match("*", "anything"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "something"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(std::time::Duration::from_secs(1)), "1.00s");
        assert_eq!(format_duration(std::time::Duration::from_secs(90)), "1m 30.0s");
        assert_eq!(format_duration(std::time::Duration::from_secs(3661)), "1h 1m");
    }
}
