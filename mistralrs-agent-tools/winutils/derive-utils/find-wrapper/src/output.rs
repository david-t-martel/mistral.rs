//! Output formatting for find results

use anyhow::Result;
use std::io::{self, Write};
use std::time::SystemTime;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::options::{ColorMode, FileType, FindOptions};

/// Result structure for find operations
#[derive(Debug, Clone)]
pub struct FindResult {
    pub path: String,
    pub file_type: FileType,
    pub size: u64,
    pub modified: Option<SystemTime>,
    #[cfg(windows)]
    pub windows_attributes: Option<WindowsAttributes>,
    #[cfg(windows)]
    pub ntfs_streams: Option<Vec<String>>,
}

/// Windows file attributes
#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct WindowsAttributes {
    pub hidden: bool,
    pub system: bool,
    pub readonly: bool,
    pub archive: bool,
    pub directory: bool,
    pub compressed: bool,
    pub encrypted: bool,
    pub reparse_point: bool,
}

/// Output formatter for find results
pub struct OutputFormatter {
    stdout: StandardStream,
    color_choice: ColorChoice,
    null_separator: bool,
    show_attributes: bool,
    show_size: bool,
    show_time: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(options: &FindOptions) -> Self {
        let color_choice = match options.color_mode {
            ColorMode::Always => ColorChoice::Always,
            ColorMode::Never => ColorChoice::Never,
            ColorMode::Auto => {
                if atty::is(atty::Stream::Stdout) {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
        };

        Self {
            stdout: StandardStream::stdout(color_choice),
            color_choice,
            null_separator: options.null_separator,
            show_attributes: options.show_windows_attributes,
            show_size: false, // TODO @gemini: Add option for this
            show_time: false, // TODO @codex: Add option for this
        }
    }

    /// Format and output a find result
    pub fn format_entry(&mut self, result: &FindResult) -> Result<()> {
        // Set color based on file type
        self.set_color_for_file_type(result.file_type)?;

        // Output the path
        write!(self.stdout, "{}", result.path)?;

        // Reset color
        self.stdout.reset()?;

        // Add size information if requested
        if self.show_size {
            write!(self.stdout, " ({})", humansize::format_size(result.size, humansize::BINARY))?;
        }

        // Add Windows attributes if requested
        #[cfg(windows)]
        if self.show_attributes {
            if let Some(attrs) = &result.windows_attributes {
                write!(self.stdout, " {}", format_windows_attributes(attrs))?;
            }
        }

        // Add NTFS streams if present
        #[cfg(windows)]
        if let Some(streams) = &result.ntfs_streams {
            if !streams.is_empty() {
                write!(self.stdout, " [streams: {}]", streams.join(", "))?;
            }
        }

        // Output separator
        if self.null_separator {
            write!(self.stdout, "\0")?;
        } else {
            writeln!(self.stdout)?;
        }

        self.stdout.flush()?;
        Ok(())
    }

    /// Set color based on file type
    fn set_color_for_file_type(&mut self, file_type: FileType) -> Result<()> {
        if self.color_choice == ColorChoice::Never {
            return Ok(());
        }

        let mut color_spec = ColorSpec::new();

        match file_type {
            FileType::Directory => {
                color_spec.set_fg(Some(Color::Blue)).set_bold(true);
            }
            FileType::Symlink => {
                color_spec.set_fg(Some(Color::Cyan));
            }
            FileType::File => {
                // Check for executable files
                // TODO @gemini: Implement executable detection
                color_spec.set_fg(Some(Color::White));
            }
            FileType::Pipe => {
                color_spec.set_fg(Some(Color::Yellow));
            }
            FileType::Socket => {
                color_spec.set_fg(Some(Color::Magenta));
            }
            FileType::BlockDevice | FileType::CharDevice => {
                color_spec.set_fg(Some(Color::Red));
            }
        }

        self.stdout.set_color(&color_spec)?;
        Ok(())
    }
}

/// Format Windows attributes for display
#[cfg(windows)]
fn format_windows_attributes(attrs: &WindowsAttributes) -> String {
    let mut result = String::new();

    if attrs.hidden {
        result.push('H');
    }
    if attrs.system {
        result.push('S');
    }
    if attrs.readonly {
        result.push('R');
    }
    if attrs.archive {
        result.push('A');
    }
    if attrs.directory {
        result.push('D');
    }
    if attrs.compressed {
        result.push('C');
    }
    if attrs.encrypted {
        result.push('E');
    }
    if attrs.reparse_point {
        result.push('L');
    }

    if result.is_empty() {
        "-".to_string()
    } else {
        result
    }
}

/// Format file size in human-readable format
pub fn format_size(size: u64) -> String {
    humansize::format_size(size, humansize::BINARY)
}

/// Format system time for display
pub fn format_time(time: SystemTime) -> String {
    match time.elapsed() {
        Ok(elapsed) => {
            let secs = elapsed.as_secs();
            if secs < 60 {
                format!("{}s ago", secs)
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86400 {
                format!("{}h ago", secs / 3600)
            } else {
                format!("{}d ago", secs / 86400)
            }
        }
        Err(_) => "in future".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024), "1.00 KiB");
        assert_eq!(format_size(1024 * 1024), "1.00 MiB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GiB");
    }

    #[cfg(windows)]
    #[test]
    fn test_format_windows_attributes() {
        let attrs = WindowsAttributes {
            hidden: true,
            system: false,
            readonly: true,
            archive: false,
            directory: false,
            compressed: false,
            encrypted: false,
            reparse_point: false,
        };
        assert_eq!(format_windows_attributes(&attrs), "HR");

        let empty_attrs = WindowsAttributes {
            hidden: false,
            system: false,
            readonly: false,
            archive: false,
            directory: false,
            compressed: false,
            encrypted: false,
            reparse_point: false,
        };
        assert_eq!(format_windows_attributes(&empty_attrs), "-");
    }
}
