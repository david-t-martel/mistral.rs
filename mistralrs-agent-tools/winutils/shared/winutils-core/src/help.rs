//! Enhanced Help System for Windows Utilities
//!
//! Provides a unified help framework with examples, common use cases,
//! Windows-specific notes, and comprehensive documentation.

use crate::{WinUtilsError, WinUtilsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Represents a single example with description and command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub description: String,
    pub command: String,
    pub output: Option<String>,
    pub notes: Option<String>,
    pub windows_specific: bool,
}

impl Example {
    /// Create a new example
    pub fn new<S: Into<String>>(description: S, command: S) -> Self {
        Self {
            description: description.into(),
            command: command.into(),
            output: None,
            notes: None,
            windows_specific: false,
        }
    }

    /// Add expected output to the example
    pub fn with_output<S: Into<String>>(mut self, output: S) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Add notes to the example
    pub fn with_notes<S: Into<String>>(mut self, notes: S) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Mark this example as Windows-specific
    pub fn windows_specific(mut self) -> Self {
        self.windows_specific = true;
        self
    }
}

/// Collection of examples organized by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleSet {
    pub basic: Vec<Example>,
    pub advanced: Vec<Example>,
    pub windows_specific: Vec<Example>,
    pub common_use_cases: Vec<Example>,
    pub troubleshooting: Vec<Example>,
}

impl ExampleSet {
    /// Create a new empty example set
    pub fn new() -> Self {
        Self {
            basic: Vec::new(),
            advanced: Vec::new(),
            windows_specific: Vec::new(),
            common_use_cases: Vec::new(),
            troubleshooting: Vec::new(),
        }
    }

    /// Add a basic example
    pub fn add_basic(mut self, example: Example) -> Self {
        self.basic.push(example);
        self
    }

    /// Add an advanced example
    pub fn add_advanced(mut self, example: Example) -> Self {
        self.advanced.push(example);
        self
    }

    /// Add a Windows-specific example
    pub fn add_windows_specific(mut self, example: Example) -> Self {
        self.windows_specific.push(example);
        self
    }

    /// Add a common use case example
    pub fn add_common_use_case(mut self, example: Example) -> Self {
        self.common_use_cases.push(example);
        self
    }

    /// Add a troubleshooting example
    pub fn add_troubleshooting(mut self, example: Example) -> Self {
        self.troubleshooting.push(example);
        self
    }

    /// Get all examples as a flat vector
    pub fn all_examples(&self) -> Vec<&Example> {
        let mut all = Vec::new();
        all.extend(&self.basic);
        all.extend(&self.advanced);
        all.extend(&self.windows_specific);
        all.extend(&self.common_use_cases);
        all.extend(&self.troubleshooting);
        all
    }
}

impl Default for ExampleSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Windows-specific documentation and notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsNotes {
    pub path_handling: Vec<String>,
    pub permissions: Vec<String>,
    pub line_endings: Vec<String>,
    pub file_attributes: Vec<String>,
    pub performance: Vec<String>,
    pub compatibility: Vec<String>,
}

impl WindowsNotes {
    /// Create new Windows notes with common defaults
    pub fn new() -> Self {
        Self {
            path_handling: vec![
                "Supports both forward slashes (/) and backslashes (\\) in paths".to_string(),
                "Handles UNC paths (\\\\server\\share) and long paths (>260 characters)".to_string(),
                "Git Bash path normalization is automatically applied".to_string(),
            ],
            permissions: vec![
                "Windows permissions are mapped from Unix-style permissions".to_string(),
                "ACL support available with --windows-acl flag where applicable".to_string(),
            ],
            line_endings: vec![
                "Automatically detects and handles both CRLF and LF line endings".to_string(),
                "Use --unix-line-endings to force LF output".to_string(),
            ],
            file_attributes: vec![
                "Supports Windows file attributes (Hidden, System, Archive, ReadOnly)".to_string(),
                "Use --show-attributes to display Windows-specific attributes".to_string(),
            ],
            performance: vec![
                "Optimized for Windows filesystem characteristics".to_string(),
                "Uses native Windows APIs for better performance".to_string(),
            ],
            compatibility: vec![
                "Maintains GNU coreutils compatibility where possible".to_string(),
                "Windows-specific extensions clearly marked".to_string(),
            ],
        }
    }

    /// Add a path handling note
    pub fn add_path_note<S: Into<String>>(mut self, note: S) -> Self {
        self.path_handling.push(note.into());
        self
    }

    /// Add a permissions note
    pub fn add_permissions_note<S: Into<String>>(mut self, note: S) -> Self {
        self.permissions.push(note.into());
        self
    }

    /// Add a line endings note
    pub fn add_line_endings_note<S: Into<String>>(mut self, note: S) -> Self {
        self.line_endings.push(note.into());
        self
    }

    /// Add a file attributes note
    pub fn add_file_attributes_note<S: Into<String>>(mut self, note: S) -> Self {
        self.file_attributes.push(note.into());
        self
    }

    /// Add a performance note
    pub fn add_performance_note<S: Into<String>>(mut self, note: S) -> Self {
        self.performance.push(note.into());
        self
    }

    /// Add a compatibility note
    pub fn add_compatibility_note<S: Into<String>>(mut self, note: S) -> Self {
        self.compatibility.push(note.into());
        self
    }
}

impl Default for WindowsNotes {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive help information for a utility
#[derive(Debug, Clone)]
pub struct EnhancedHelp {
    pub utility_name: String,
    pub short_description: String,
    pub long_description: String,
    pub examples: ExampleSet,
    pub windows_notes: WindowsNotes,
    pub see_also: Vec<String>,
    pub reporting_bugs: String,
    pub author: String,
    pub copyright: String,
    pub custom_sections: HashMap<String, Vec<String>>,
}

impl EnhancedHelp {
    /// Create new enhanced help for a utility
    pub fn new<S: Into<String>>(
        utility_name: S,
        short_description: S,
        long_description: S,
    ) -> Self {
        Self {
            utility_name: utility_name.into(),
            short_description: short_description.into(),
            long_description: long_description.into(),
            examples: ExampleSet::new(),
            windows_notes: WindowsNotes::new(),
            see_also: Vec::new(),
            reporting_bugs: "Report bugs to: https://github.com/uutils/coreutils/issues".to_string(),
            author: "winutils contributors".to_string(),
            copyright: "Copyright © 2024 winutils contributors. License MIT OR Apache-2.0.".to_string(),
            custom_sections: HashMap::new(),
        }
    }

    /// Set examples
    pub fn with_examples(mut self, examples: ExampleSet) -> Self {
        self.examples = examples;
        self
    }

    /// Set Windows notes
    pub fn with_windows_notes(mut self, notes: WindowsNotes) -> Self {
        self.windows_notes = notes;
        self
    }

    /// Add a "see also" reference
    pub fn add_see_also<S: Into<String>>(mut self, reference: S) -> Self {
        self.see_also.push(reference.into());
        self
    }

    /// Add a custom section
    pub fn add_custom_section<S: Into<String>>(
        mut self,
        title: S,
        content: Vec<String>,
    ) -> Self {
        self.custom_sections.insert(title.into(), content);
        self
    }

    /// Set the author
    pub fn with_author<S: Into<String>>(mut self, author: S) -> Self {
        self.author = author.into();
        self
    }

    /// Set the bug reporting information
    pub fn with_bug_reporting<S: Into<String>>(mut self, reporting: S) -> Self {
        self.reporting_bugs = reporting.into();
        self
    }
}

/// Help system that manages and displays enhanced help information
pub struct HelpSystem {
    help: EnhancedHelp,
    color_choice: ColorChoice,
}

impl HelpSystem {
    /// Create a new help system
    pub fn new(help: EnhancedHelp) -> Self {
        Self {
            help,
            color_choice: ColorChoice::Auto,
        }
    }

    /// Set color output preference
    pub fn with_color_choice(mut self, choice: ColorChoice) -> Self {
        self.color_choice = choice;
        self
    }

    /// Show standard help (brief)
    pub fn show_help(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        // Header
        self.write_header(&mut stdout)?;

        // Short description
        writeln!(stdout, "{}", self.help.short_description)?;
        writeln!(stdout)?;

        // Usage examples (basic only)
        if !self.help.examples.basic.is_empty() {
            self.write_section_header(&mut stdout, "BASIC EXAMPLES")?;
            for example in &self.help.examples.basic[..std::cmp::min(3, self.help.examples.basic.len())] {
                self.write_example(&mut stdout, example)?;
            }
            writeln!(stdout)?;
        }

        // Footer
        writeln!(stdout, "Use --help-full for comprehensive help with all examples and Windows-specific information.")?;

        Ok(())
    }

    /// Show comprehensive help with all sections
    pub fn show_full_help(&self) -> WinUtilsResult<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        // Header
        self.write_header(&mut stdout)?;

        // Descriptions
        writeln!(stdout, "{}", self.help.short_description)?;
        writeln!(stdout)?;
        writeln!(stdout, "{}", self.help.long_description)?;
        writeln!(stdout)?;

        // Examples sections
        self.write_examples_section(&mut stdout, "BASIC EXAMPLES", &self.help.examples.basic)?;
        self.write_examples_section(&mut stdout, "COMMON USE CASES", &self.help.examples.common_use_cases)?;
        self.write_examples_section(&mut stdout, "ADVANCED EXAMPLES", &self.help.examples.advanced)?;
        self.write_examples_section(&mut stdout, "WINDOWS-SPECIFIC EXAMPLES", &self.help.examples.windows_specific)?;
        self.write_examples_section(&mut stdout, "TROUBLESHOOTING", &self.help.examples.troubleshooting)?;

        // Windows notes
        self.write_windows_notes(&mut stdout)?;

        // Custom sections
        for (title, content) in &self.help.custom_sections {
            self.write_section_header(&mut stdout, title)?;
            for line in content {
                writeln!(stdout, "  {}", line)?;
            }
            writeln!(stdout)?;
        }

        // See also
        if !self.help.see_also.is_empty() {
            self.write_section_header(&mut stdout, "SEE ALSO")?;
            writeln!(stdout, "  {}", self.help.see_also.join(", "))?;
            writeln!(stdout)?;
        }

        // Footer
        self.write_footer(&mut stdout)?;

        Ok(())
    }

    /// Generate man page content
    #[cfg(feature = "man-pages")]
    pub fn generate_man_page(&self) -> WinUtilsResult<String> {
        use roff::{Roff, PAGE_BREAK};

        let mut roff = Roff::new();

        // Title
        roff = roff.control("TH", vec![
            &self.help.utility_name.to_uppercase(),
            "1",
            &chrono::Utc::now().format("%B %Y").to_string(),
            "winutils",
            "User Commands"
        ]);

        // Name section
        roff = roff.control("SH", vec!["NAME"]);
        roff = roff.text(vec![
            &format!("{} - {}", self.help.utility_name, self.help.short_description)
        ]);

        // Description
        roff = roff.control("SH", vec!["DESCRIPTION"]);
        roff = roff.text(vec![&self.help.long_description]);

        // Examples
        if !self.help.examples.all_examples().is_empty() {
            roff = roff.control("SH", vec!["EXAMPLES"]);
            for example in self.help.examples.all_examples() {
                roff = roff.control("TP", vec![]);
                roff = roff.text(vec![&example.description]);
                roff = roff.control("br", vec![]);
                roff = roff.text(vec![&format!(".B {}", example.command)]);
                if let Some(ref output) = example.output {
                    roff = roff.control("br", vec![]);
                    roff = roff.text(vec![output]);
                }
            }
        }

        // Author
        roff = roff.control("SH", vec!["AUTHOR"]);
        roff = roff.text(vec![&self.help.author]);

        // Copyright
        roff = roff.control("SH", vec!["COPYRIGHT"]);
        roff = roff.text(vec![&self.help.copyright]);

        // Bug reporting
        roff = roff.control("SH", vec!["REPORTING BUGS"]);
        roff = roff.text(vec![&self.help.reporting_bugs]);

        Ok(roff.render())
    }

    fn write_header(&self, stdout: &mut StandardStream) -> WinUtilsResult<()> {
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
        writeln!(stdout, "{} - {}", self.help.utility_name.to_uppercase(), self.help.short_description)?;
        stdout.reset()?;
        Ok(())
    }

    fn write_section_header(&self, stdout: &mut StandardStream, title: &str) -> WinUtilsResult<()> {
        stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Blue)))?;
        writeln!(stdout, "{}", title)?;
        stdout.reset()?;
        Ok(())
    }

    fn write_example(&self, stdout: &mut StandardStream, example: &Example) -> WinUtilsResult<()> {
        // Description
        stdout.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(stdout, "  {}", example.description)?;
        stdout.reset()?;

        // Command
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        writeln!(stdout, "    {}", example.command)?;
        stdout.reset()?;

        // Output (if provided)
        if let Some(ref output) = example.output {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            for line in output.lines() {
                writeln!(stdout, "    {}", line)?;
            }
            stdout.reset()?;
        }

        // Notes (if provided)
        if let Some(ref notes) = example.notes {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
            writeln!(stdout, "    Note: {}", notes)?;
            stdout.reset()?;
        }

        // Windows-specific marker
        if example.windows_specific {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
            writeln!(stdout, "    [Windows-specific]")?;
            stdout.reset()?;
        }

        writeln!(stdout)?;
        Ok(())
    }

    fn write_examples_section(&self, stdout: &mut StandardStream, title: &str, examples: &[Example]) -> WinUtilsResult<()> {
        if !examples.is_empty() {
            self.write_section_header(stdout, title)?;
            for example in examples {
                self.write_example(stdout, example)?;
            }
            writeln!(stdout)?;
        }
        Ok(())
    }

    fn write_windows_notes(&self, stdout: &mut StandardStream) -> WinUtilsResult<()> {
        self.write_section_header(stdout, "WINDOWS-SPECIFIC NOTES")?;

        self.write_note_category(stdout, "Path Handling", &self.help.windows_notes.path_handling)?;
        self.write_note_category(stdout, "Permissions", &self.help.windows_notes.permissions)?;
        self.write_note_category(stdout, "Line Endings", &self.help.windows_notes.line_endings)?;
        self.write_note_category(stdout, "File Attributes", &self.help.windows_notes.file_attributes)?;
        self.write_note_category(stdout, "Performance", &self.help.windows_notes.performance)?;
        self.write_note_category(stdout, "Compatibility", &self.help.windows_notes.compatibility)?;

        writeln!(stdout)?;
        Ok(())
    }

    fn write_note_category(&self, stdout: &mut StandardStream, title: &str, notes: &[String]) -> WinUtilsResult<()> {
        if !notes.is_empty() {
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Magenta)))?;
            writeln!(stdout, "  {}:", title)?;
            stdout.reset()?;

            for note in notes {
                writeln!(stdout, "    • {}", note)?;
            }
            writeln!(stdout)?;
        }
        Ok(())
    }

    fn write_footer(&self, stdout: &mut StandardStream) -> WinUtilsResult<()> {
        self.write_section_header(stdout, "AUTHOR")?;
        writeln!(stdout, "  {}", self.help.author)?;
        writeln!(stdout)?;

        self.write_section_header(stdout, "REPORTING BUGS")?;
        writeln!(stdout, "  {}", self.help.reporting_bugs)?;
        writeln!(stdout)?;

        self.write_section_header(stdout, "COPYRIGHT")?;
        writeln!(stdout, "  {}", self.help.copyright)?;

        Ok(())
    }
}

impl fmt::Write for StandardStream {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use std::io::Write;
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_creation() {
        let example = Example::new("List files", "ls -la")
            .with_output("total 8\ndrwxr-xr-x 2 user user 4096 Jan 1 12:00 .")
            .with_notes("Shows detailed file information")
            .windows_specific();

        assert_eq!(example.description, "List files");
        assert_eq!(example.command, "ls -la");
        assert!(example.windows_specific);
        assert!(example.output.is_some());
        assert!(example.notes.is_some());
    }

    #[test]
    fn test_example_set() {
        let mut examples = ExampleSet::new();
        examples = examples.add_basic(Example::new("Basic usage", "ls"));
        examples = examples.add_advanced(Example::new("Advanced usage", "ls -laR"));

        assert_eq!(examples.basic.len(), 1);
        assert_eq!(examples.advanced.len(), 1);
        assert_eq!(examples.all_examples().len(), 2);
    }

    #[test]
    fn test_enhanced_help() {
        let help = EnhancedHelp::new(
            "test-util",
            "A test utility",
            "This is a test utility for demonstration purposes.",
        )
        .add_see_also("ls(1)")
        .with_author("Test Author");

        assert_eq!(help.utility_name, "test-util");
        assert_eq!(help.see_also.len(), 1);
        assert_eq!(help.author, "Test Author");
    }
}
