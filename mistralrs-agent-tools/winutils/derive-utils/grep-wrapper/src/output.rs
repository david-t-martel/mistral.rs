//! Output formatting and printing for grep results

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use grep_searcher::SinkMatch;
use grep_printer::{PrinterPath, Sink};
use winpath::PathNormalizer;

use crate::config::{GrepConfig, PathFormat, ColorMode};

/// Trait for grep output printing
pub trait GrepPrinter {
    /// Create a sink for search results
    fn sink<'a>(
        &'a self,
        path: Option<&'a Path>,
        match_count: &'a mut u64,
        line_count: &'a mut u64,
        config: &'a GrepConfig,
    ) -> Result<Box<dyn Sink + 'a>>;
}

/// Enhanced printer with Windows path support
pub struct EnhancedPrinter {
    stdout: StandardStream,
    config: GrepConfig,
    normalizer: Arc<PathNormalizer>,
    color_choice: ColorChoice,
}

impl EnhancedPrinter {
    /// Create a new enhanced printer
    pub fn new(
        config: &GrepConfig,
        color_choice: ColorChoice,
        normalizer: Arc<PathNormalizer>,
    ) -> Result<Self> {
        Ok(Self {
            stdout: StandardStream::stdout(color_choice),
            config: config.clone(),
            normalizer,
            color_choice,
        })
    }

    /// Format path according to configuration
    fn format_path(&self, path: &Path) -> String {
        match &self.config.path_format {
            PathFormat::Windows => {
                self.normalizer.normalize(path.to_string_lossy().as_ref())
                    .map(|result| result.into_owned())
                    .unwrap_or_else(|_| path.display().to_string())
            }
            PathFormat::Unix => {
                path.to_string_lossy().replace('\\', "/")
            }
            PathFormat::Native => {
                path.display().to_string()
            }
            PathFormat::Auto => {
                if cfg!(windows) {
                    self.normalizer.normalize(path.to_string_lossy().as_ref())
                        .map(|result| result.into_owned())
                        .unwrap_or_else(|_| path.display().to_string())
                } else {
                    path.display().to_string()
                }
            }
        }
    }
}

impl GrepPrinter for EnhancedPrinter {
    fn sink<'a>(
        &'a self,
        path: Option<&'a Path>,
        match_count: &'a mut u64,
        line_count: &'a mut u64,
        config: &'a GrepConfig,
    ) -> Result<Box<dyn Sink + 'a>> {
        Ok(Box::new(EnhancedSink {
            printer: self,
            path,
            match_count,
            line_count,
            config,
            current_match_count: 0,
        }))
    }
}

/// Sink implementation for enhanced output
struct EnhancedSink<'a> {
    printer: &'a EnhancedPrinter,
    path: Option<&'a Path>,
    match_count: &'a mut u64,
    line_count: &'a mut u64,
    config: &'a GrepConfig,
    current_match_count: u64,
}

impl<'a> Sink for EnhancedSink<'a> {
    type Error = anyhow::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        *self.match_count += 1;
        self.current_match_count += 1;

        // Check max count limit
        if let Some(max_count) = self.config.max_count {
            if self.current_match_count >= max_count as u64 {
                return Ok(false); // Stop searching
            }
        }

        // Handle different output modes
        if self.config.count_only {
            // Count mode - don't print matches, just count
            return Ok(true);
        }

        if self.config.files_with_matches {
            // Files with matches mode - print filename and stop
            if let Some(path) = self.path {
                let formatted_path = self.printer.format_path(path);
                if self.config.null_separator {
                    print!("{}\0", formatted_path);
                } else {
                    println!("{}", formatted_path);
                }
            }
            return Ok(false); // Stop after first match
        }

        if self.config.files_without_match {
            // Files without match mode - this will be handled in context_break
            return Ok(true);
        }

        // Regular output mode
        self.print_match(mat)?;

        Ok(true)
    }

    fn context_break(&mut self, _searcher: &grep_searcher::Searcher) -> Result<(), Self::Error> {
        // Handle files without match mode
        if self.config.files_without_match && self.current_match_count == 0 {
            if let Some(path) = self.path {
                let formatted_path = self.printer.format_path(path);
                if self.config.null_separator {
                    print!("{}\0", formatted_path);
                } else {
                    println!("{}", formatted_path);
                }
            }
        }

        Ok(())
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        _context: &grep_searcher::SinkContext<'_>,
    ) -> Result<(), Self::Error> {
        // Context lines handling would go here
        // For now, we'll implement basic context support
        Ok(())
    }
}

impl<'a> EnhancedSink<'a> {
    /// Print a match with proper formatting
    fn print_match(&mut self, mat: &SinkMatch<'_>) -> Result<()> {
        let mut stdout = &mut self.printer.stdout;

        // Print filename if needed
        if self.config.should_show_filename() {
            if let Some(path) = self.path {
                let formatted_path = self.printer.format_path(path);

                // Set filename color
                if self.printer.color_choice != ColorChoice::Never {
                    let mut color_spec = ColorSpec::new();
                    color_spec.set_fg(Some(Color::Magenta)).set_bold(true);
                    stdout.set_color(&color_spec)?;
                }

                print!("{}", formatted_path);
                stdout.reset()?;

                if self.config.null_separator {
                    print!("\0");
                } else {
                    print!(":");
                }
            }
        }

        // Print line number if needed
        if self.config.line_number {
            if self.printer.color_choice != ColorChoice::Never {
                let mut color_spec = ColorSpec::new();
                color_spec.set_fg(Some(Color::Green)).set_bold(true);
                stdout.set_color(&color_spec)?;
            }

            print!("{}", mat.line_number().unwrap_or(0));
            stdout.reset()?;

            if self.config.null_separator {
                print!("\0");
            } else {
                print!(":");
            }
        }

        // Print byte offset if needed
        if self.config.byte_offset {
            if self.printer.color_choice != ColorChoice::Never {
                let mut color_spec = ColorSpec::new();
                color_spec.set_fg(Some(Color::Cyan));
                stdout.set_color(&color_spec)?;
            }

            print!("{}", mat.absolute_byte_offset());
            stdout.reset()?;

            if self.config.null_separator {
                print!("\0");
            } else {
                print!(":");
            }
        }

        // Print the match
        if self.config.only_matching {
            // Only print the matching part
            self.print_only_matches(mat)?;
        } else {
            // Print the entire line with highlighted matches
            self.print_line_with_matches(mat)?;
        }

        if self.config.null_separator {
            print!("\0");
        } else {
            println!();
        }

        Ok(())
    }

    /// Print only the matching parts of a line
    fn print_only_matches(&mut self, mat: &SinkMatch<'_>) -> Result<()> {
        let line = mat.bytes();
        let matches = mat.submatches();

        for submatch in matches.iter() {
            if let Some(match_bytes) = submatch.as_bytes() {
                // Highlight the match
                if self.printer.color_choice != ColorChoice::Never {
                    let mut color_spec = ColorSpec::new();
                    color_spec.set_fg(Some(Color::Red)).set_bold(true);
                    self.printer.stdout.set_color(&color_spec)?;
                }

                // Convert bytes to string, handling potential encoding issues
                let match_str = String::from_utf8_lossy(match_bytes);
                print!("{}", match_str);
                self.printer.stdout.reset()?;

                if self.config.null_separator {
                    print!("\0");
                } else {
                    println!();
                }
            }
        }

        Ok(())
    }

    /// Print a line with highlighted matches
    fn print_line_with_matches(&mut self, mat: &SinkMatch<'_>) -> Result<()> {
        let line = mat.bytes();
        let matches = mat.submatches();

        if matches.is_empty() {
            // No submatches, print the entire line
            let line_str = String::from_utf8_lossy(line);
            print!("{}", line_str.trim_end());
            return Ok(());
        }

        let mut last_end = 0;
        let line_str = String::from_utf8_lossy(line);

        for submatch in matches.iter() {
            let start = submatch.start();
            let end = submatch.end();

            // Print text before the match
            print!("{}", &line_str[last_end..start]);

            // Highlight the match
            if self.printer.color_choice != ColorChoice::Never {
                let mut color_spec = ColorSpec::new();
                color_spec.set_fg(Some(Color::Red)).set_bold(true);
                self.printer.stdout.set_color(&color_spec)?;
            }

            print!("{}", &line_str[start..end]);
            self.printer.stdout.reset()?;

            last_end = end;
        }

        // Print remaining text after the last match
        print!("{}", &line_str[last_end..].trim_end());

        Ok(())
    }
}

/// Simple counting sink for count-only mode
pub struct CountingSink {
    pub count: u64,
}

impl CountingSink {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

impl Sink for CountingSink {
    type Error = anyhow::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        _mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        self.count += 1;
        Ok(true)
    }

    fn context_break(&mut self, _searcher: &grep_searcher::Searcher) -> Result<(), Self::Error> {
        Ok(())
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        _context: &grep_searcher::SinkContext<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_path_formatting() {
        let normalizer = Arc::new(PathNormalizer::new());
        let config = GrepConfig {
            path_format: PathFormat::Unix,
            ..Default::default()
        };

        let printer = EnhancedPrinter::new(&config, ColorChoice::Never, normalizer).unwrap();

        let path = Path::new(r"C:\Users\Test\file.txt");
        let formatted = printer.format_path(path);

        assert!(formatted.contains("/"));
        assert!(!formatted.contains("\\"));
    }
}
