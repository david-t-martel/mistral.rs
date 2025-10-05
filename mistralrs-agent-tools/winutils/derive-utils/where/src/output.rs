// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Output formatting for the where utility

use crate::args::Args;
use crate::error::WhereResult;
use chrono::{DateTime, Local};
use humansize::{format_size, WINDOWS};
use std::fs::Metadata;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// A search result containing file information
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Full path to the file
    pub path: PathBuf,
    /// File metadata (if available and requested)
    pub metadata: Option<Metadata>,
    /// Which pattern matched this file
    pub matched_pattern: String,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(path: PathBuf, matched_pattern: String) -> Self {
        Self {
            path,
            metadata: None,
            matched_pattern,
        }
    }

    /// Create a search result with metadata
    pub fn with_metadata(path: PathBuf, matched_pattern: String, metadata: Metadata) -> Self {
        Self {
            path,
            metadata: Some(metadata),
            matched_pattern,
        }
    }

    /// Get the file size if metadata is available
    pub fn file_size(&self) -> Option<u64> {
        self.metadata.as_ref().map(|m| m.len())
    }

    /// Get the modification time if metadata is available
    pub fn modified_time(&self) -> Option<DateTime<Local>> {
        self.metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| DateTime::from(t))
    }

    /// Check if this is an executable file based on its extension
    pub fn is_executable(&self) -> bool {
        crate::pathext::has_executable_extension(
            &self.path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    }
}

/// Output formatter for search results
pub struct OutputFormatter {
    /// Output stream
    stream: StandardStream,
    /// Arguments controlling output format
    args: Args,
    /// Whether to use colors
    use_colors: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(args: Args) -> Self {
        let color_choice = if args.quiet {
            ColorChoice::Never
        } else {
            ColorChoice::Auto
        };

        Self {
            stream: StandardStream::stdout(color_choice),
            use_colors: color_choice != ColorChoice::Never,
            args,
        }
    }

    /// Format and print search results
    pub fn print_results(&mut self, results: &[SearchResult]) -> WhereResult<()> {
        if self.args.quiet {
            return Ok(());
        }

        for result in results {
            self.print_result(result)?;
        }

        Ok(())
    }

    /// Print a single search result
    fn print_result(&mut self, result: &SearchResult) -> WhereResult<()> {
        if self.args.show_time && result.metadata.is_some() {
            self.print_detailed_result(result)
        } else {
            self.print_simple_result(result)
        }
    }

    /// Print a simple result (just the path)
    fn print_simple_result(&mut self, result: &SearchResult) -> WhereResult<()> {
        let path_str = if self.args.full_path {
            self.format_full_path(&result.path)
        } else {
            self.format_relative_path(&result.path)
        };

        if self.use_colors && result.is_executable() {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        }

        writeln!(self.stream, "{}", path_str)?;

        if self.use_colors {
            self.stream.reset()?;
        }

        Ok(())
    }

    /// Print a detailed result with size and time information
    fn print_detailed_result(&mut self, result: &SearchResult) -> WhereResult<()> {
        let path_str = if self.args.full_path {
            self.format_full_path(&result.path)
        } else {
            self.format_relative_path(&result.path)
        };

        // Format file size
        let size_str = result.file_size()
            .map(|size| format_size(size, WINDOWS))
            .unwrap_or_else(|| "Unknown".to_string());

        // Format modification time
        let time_str = result.modified_time()
            .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Color the path if it's executable
        if self.use_colors && result.is_executable() {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        }

        write!(self.stream, "{:<40}", path_str)?;

        if self.use_colors {
            self.stream.reset()?;
        }

        // Print size and time
        if self.use_colors {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        }

        write!(self.stream, " {:>10}", size_str)?;

        if self.use_colors {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        }

        writeln!(self.stream, " {}", time_str)?;

        if self.use_colors {
            self.stream.reset()?;
        }

        Ok(())
    }

    /// Format a path as a full path
    fn format_full_path(&self, path: &Path) -> String {
        dunce::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
            .to_string()
    }

    /// Format a path as a relative path (if possible)
    fn format_relative_path(&self, path: &Path) -> String {
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(relative) = path.strip_prefix(&current_dir) {
                return relative.display().to_string();
            }
        }
        path.display().to_string()
    }

    /// Print summary information
    pub fn print_summary(&mut self, results: &[SearchResult], patterns: &[String]) -> WhereResult<()> {
        if self.args.quiet || results.is_empty() {
            return Ok(());
        }

        if self.use_colors {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        }

        let pattern_str = if patterns.len() == 1 {
            patterns[0].clone()
        } else {
            format!("{} patterns", patterns.len())
        };

        writeln!(
            self.stream,
            "\nFound {} file(s) matching {}",
            results.len(),
            pattern_str
        )?;

        if self.use_colors {
            self.stream.reset()?;
        }

        Ok(())
    }

    /// Print error message
    pub fn print_error(&mut self, error: &str) -> WhereResult<()> {
        if self.args.quiet {
            return Ok(());
        }

        if self.use_colors {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        }

        writeln!(self.stream, "where: {}", error)?;

        if self.use_colors {
            self.stream.reset()?;
        }

        Ok(())
    }

    /// Flush the output stream
    pub fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_search_result_creation() {
        let path = PathBuf::from("test.exe");
        let result = SearchResult::new(path.clone(), "*.exe".to_string());

        assert_eq!(result.path, path);
        assert_eq!(result.matched_pattern, "*.exe");
        assert!(result.metadata.is_none());
    }

    #[test]
    fn test_search_result_executable_detection() {
        let exe_result = SearchResult::new(PathBuf::from("test.exe"), "*.exe".to_string());
        assert!(exe_result.is_executable());

        let txt_result = SearchResult::new(PathBuf::from("test.txt"), "*.txt".to_string());
        assert!(!txt_result.is_executable());
    }

    #[test]
    fn test_output_formatter_creation() {
        let args = Args {
            patterns: vec!["test".to_string()],
            recursive_dir: None,
            quiet: false,
            full_path: false,
            show_time: false,
        };

        let formatter = OutputFormatter::new(args);
        assert!(!formatter.args.quiet);
    }

    #[test]
    fn test_search_result_with_metadata() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.exe");
        fs::write(&file_path, "test content").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let result = SearchResult::with_metadata(
            file_path,
            "*.exe".to_string(),
            metadata
        );

        assert!(result.file_size().is_some());
        assert!(result.modified_time().is_some());
    }
}
