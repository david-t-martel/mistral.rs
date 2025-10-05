//! Filtering logic for find operations

use anyhow::{anyhow, Result};
use regex::Regex;
use std::time::SystemTime;
use glob::Pattern;
use crate::options::{FindOptions, FileType};

/// Name-based filtering
#[derive(Debug, Clone)]
pub struct NameFilter {
    pattern: Pattern,
    case_sensitive: bool,
}

/// File type filtering
#[derive(Debug, Clone)]
pub struct TypeFilter {
    file_type: FileType,
}

/// Size-based filtering
#[derive(Debug, Clone)]
pub struct SizeFilter {
    mode: SizeMode,
    size: u64,
}

/// Time-based filtering
#[derive(Debug, Clone)]
pub struct TimeFilter {
    mode: TimeMode,
    reference_time: SystemTime,
}

/// Size comparison mode
#[derive(Debug, Clone, PartialEq)]
pub enum SizeMode {
    Exact,
    GreaterThan,
    LessThan,
}

/// Time comparison mode
#[derive(Debug, Clone, PartialEq)]
pub enum TimeMode {
    NewerThan,
    OlderThan,
}

impl NameFilter {
    /// Create a new name filter from pattern
    pub fn new(pattern: &str, case_sensitive: bool) -> Result<Self> {
        let glob_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            // Convert to case-insensitive pattern
            pattern.to_lowercase()
        };

        let pattern = Pattern::new(&glob_pattern)
            .map_err(|e| anyhow!("Invalid name pattern: {}", e))?;

        Ok(Self {
            pattern,
            case_sensitive,
        })
    }

    /// Check if a path matches this filter
    pub fn matches(&self, path: &str) -> bool {
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path);

        let test_name = if self.case_sensitive {
            filename.to_string()
        } else {
            filename.to_lowercase()
        };

        self.pattern.matches(&test_name)
    }
}

impl TypeFilter {
    /// Create a new type filter
    pub fn new(type_char: char) -> Result<Self> {
        let file_type = match type_char {
            'f' => FileType::File,
            'd' => FileType::Directory,
            'l' => FileType::Symlink,
            'p' => FileType::Pipe,
            's' => FileType::Socket,
            'b' => FileType::BlockDevice,
            'c' => FileType::CharDevice,
            _ => return Err(anyhow!("Invalid file type: {}", type_char)),
        };

        Ok(Self { file_type })
    }

    /// Check if a file type matches this filter
    pub fn matches(&self, file_type: FileType) -> bool {
        self.file_type == file_type
    }
}

impl SizeFilter {
    /// Create a new size filter from size specification
    pub fn new(size_spec: &str) -> Result<Self> {
        let (mode, size_str) = if size_spec.starts_with('+') {
            (SizeMode::GreaterThan, &size_spec[1..])
        } else if size_spec.starts_with('-') {
            (SizeMode::LessThan, &size_spec[1..])
        } else {
            (SizeMode::Exact, size_spec)
        };

        let size = parse_size(size_str)?;

        Ok(Self { mode, size })
    }

    /// Check if a file size matches this filter
    pub fn matches(&self, file_size: u64) -> bool {
        match self.mode {
            SizeMode::Exact => file_size == self.size,
            SizeMode::GreaterThan => file_size > self.size,
            SizeMode::LessThan => file_size < self.size,
        }
    }
}

impl TimeFilter {
    /// Create a newer-than filter
    pub fn newer_than(reference_path: &std::path::Path) -> Result<Self> {
        let metadata = std::fs::metadata(reference_path)
            .map_err(|e| anyhow!("Cannot access reference file: {}", e))?;

        let reference_time = metadata.modified()
            .map_err(|e| anyhow!("Cannot get modification time: {}", e))?;

        Ok(Self {
            mode: TimeMode::NewerThan,
            reference_time,
        })
    }

    /// Create an older-than filter
    pub fn older_than(reference_path: &std::path::Path) -> Result<Self> {
        let metadata = std::fs::metadata(reference_path)
            .map_err(|e| anyhow!("Cannot access reference file: {}", e))?;

        let reference_time = metadata.modified()
            .map_err(|e| anyhow!("Cannot get modification time: {}", e))?;

        Ok(Self {
            mode: TimeMode::OlderThan,
            reference_time,
        })
    }

    /// Check if a file time matches this filter
    pub fn matches(&self, file_time: SystemTime) -> bool {
        match self.mode {
            TimeMode::NewerThan => file_time > self.reference_time,
            TimeMode::OlderThan => file_time < self.reference_time,
        }
    }
}

/// Parse size specification with suffixes
fn parse_size(size_str: &str) -> Result<u64> {
    let (number_part, suffix) = if size_str.ends_with(['k', 'K']) {
        (&size_str[..size_str.len() - 1], 1024u64)
    } else if size_str.ends_with(['m', 'M']) {
        (&size_str[..size_str.len() - 1], 1024u64.pow(2))
    } else if size_str.ends_with(['g', 'G']) {
        (&size_str[..size_str.len() - 1], 1024u64.pow(3))
    } else if size_str.ends_with(['t', 'T']) {
        (&size_str[..size_str.len() - 1], 1024u64.pow(4))
    } else {
        (size_str, 1u64)
    };

    let number: u64 = number_part
        .parse()
        .map_err(|_| anyhow!("Invalid size specification: {}", size_str))?;

    Ok(number.saturating_mul(suffix))
}

/// Create name filter from options
pub fn create_name_filter(options: &FindOptions) -> Result<Option<NameFilter>> {
    if let Some(pattern) = &options.name_pattern {
        Ok(Some(NameFilter::new(pattern, !options.ignore_case)?))
    } else if let Some(pattern) = &options.iname_pattern {
        Ok(Some(NameFilter::new(pattern, false)?))
    } else {
        Ok(None)
    }
}

/// Create type filter from options
pub fn create_type_filter(options: &FindOptions) -> Result<Option<TypeFilter>> {
    if let Some(type_str) = &options.file_type {
        if type_str.len() != 1 {
            return Err(anyhow!("File type must be a single character"));
        }
        let type_char = type_str.chars().next().unwrap();
        Ok(Some(TypeFilter::new(type_char)?))
    } else {
        Ok(None)
    }
}

/// Create size filter from options
pub fn create_size_filter(options: &FindOptions) -> Result<Option<SizeFilter>> {
    if let Some(size_spec) = &options.size_filter {
        Ok(Some(SizeFilter::new(size_spec)?))
    } else {
        Ok(None)
    }
}

/// Create time filter from options
pub fn create_time_filter(options: &FindOptions) -> Result<Option<TimeFilter>> {
    if let Some(newer_path) = &options.newer_than {
        Ok(Some(TimeFilter::newer_than(newer_path)?))
    } else if let Some(older_path) = &options.older_than {
        Ok(Some(TimeFilter::older_than(older_path)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_parsing() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("1k").unwrap(), 1024);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("1m").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1g").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_name_filter() {
        let filter = NameFilter::new("*.txt", true).unwrap();
        assert!(filter.matches("test.txt"));
        assert!(filter.matches("/path/to/test.txt"));
        assert!(!filter.matches("test.rs"));

        let case_insensitive = NameFilter::new("*.TXT", false).unwrap();
        assert!(case_insensitive.matches("test.txt"));
        assert!(case_insensitive.matches("TEST.TXT"));
    }

    #[test]
    fn test_type_filter() {
        let file_filter = TypeFilter::new('f').unwrap();
        assert!(file_filter.matches(FileType::File));
        assert!(!file_filter.matches(FileType::Directory));

        let dir_filter = TypeFilter::new('d').unwrap();
        assert!(dir_filter.matches(FileType::Directory));
        assert!(!dir_filter.matches(FileType::File));
    }

    #[test]
    fn test_size_filter() {
        let exact_filter = SizeFilter::new("1024").unwrap();
        assert!(exact_filter.matches(1024));
        assert!(!exact_filter.matches(1025));

        let greater_filter = SizeFilter::new("+1k").unwrap();
        assert!(greater_filter.matches(2048));
        assert!(!greater_filter.matches(512));

        let less_filter = SizeFilter::new("-1k").unwrap();
        assert!(less_filter.matches(512));
        assert!(!less_filter.matches(2048));
    }
}
