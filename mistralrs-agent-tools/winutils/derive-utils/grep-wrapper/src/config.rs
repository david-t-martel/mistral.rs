//! Configuration and command-line parsing for grep wrapper

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use std::path::PathBuf;

/// Main configuration for grep operation
#[derive(Debug, Clone)]
pub struct GrepConfig {
    pub pattern: String,
    pub files: Vec<String>,
    // Pattern options
    pub ignore_case: bool,
    pub word_regexp: bool,
    pub line_regexp: bool,
    pub fixed_strings: bool,
    pub basic_regexp: bool,
    pub extended_regexp: bool,
    pub perl_regexp: bool,
    // Output options
    pub line_number: bool,
    pub count_only: bool,
    pub files_with_matches: bool,
    pub files_without_match: bool,
    pub with_filename: bool,
    pub no_filename: bool,
    pub only_matching: bool,
    // Context options
    pub before_context: usize,
    pub after_context: usize,
    // Search behavior
    pub recursive: bool,
    pub follow_symlinks: bool,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub exclude_dirs: Vec<String>,
    // Selection and inversion
    pub invert_match: bool,
    pub max_count: Option<usize>,
    // Binary and encoding
    pub binary_files: String,
    pub text_mode: bool,
    pub null_data: bool,
    pub encoding: String,
    pub windows_encoding: bool,
    pub handle_crlf: bool,
    // Performance
    pub parallel_threads: Option<usize>,
    pub use_mmap: bool,
    // Output format
    pub color_mode: ColorMode,
    pub null_separator: bool,
    pub byte_offset: bool,
    pub quiet: bool,
    pub path_format: PathFormat,
    // Pattern files
    pub pattern_files: Vec<PathBuf>,
}

/// Color output mode
#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Path output format
#[derive(Debug, Clone, PartialEq)]
pub enum PathFormat {
    Windows,
    Unix,
    Native,
    Auto,
}

impl GrepConfig {
    /// Parse configuration from command-line matches
    pub fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let pattern = matches
            .get_one::<String>("pattern")
            .ok_or_else(|| anyhow!("Pattern is required"))?
            .clone();

        let files = matches
            .get_many::<String>("files")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        let parallel_threads = matches
            .get_one::<String>("parallel")
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid thread count"))?;

        let color_mode = match matches.get_one::<String>("color").map(|s| s.as_str()) {
            Some("always") => ColorMode::Always,
            Some("never") => ColorMode::Never,
            Some("auto") | None => ColorMode::Auto,
            Some(other) => return Err(anyhow!("Invalid color mode: {}", other)),
        };

        let path_format = match matches.get_one::<String>("path-format").map(|s| s.as_str()) {
            Some("windows") => PathFormat::Windows,
            Some("unix") => PathFormat::Unix,
            Some("native") => PathFormat::Native,
            Some("auto") | None => PathFormat::Auto,
            Some(other) => return Err(anyhow!("Invalid path format: {}", other)),
        };

        let before_context = matches
            .get_one::<String>("before-context")
            .or_else(|| matches.get_one::<String>("context"))
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid before-context value"))?
            .unwrap_or(0);

        let after_context = matches
            .get_one::<String>("after-context")
            .or_else(|| matches.get_one::<String>("context"))
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid after-context value"))?
            .unwrap_or(0);

        let max_count = matches
            .get_one::<String>("max-count")
            .map(|s| s.parse::<usize>())
            .transpose()
            .map_err(|_| anyhow!("Invalid max-count value"))?;

        let include_patterns = matches
            .get_many::<String>("include")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        let exclude_patterns = matches
            .get_many::<String>("exclude")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        let exclude_dirs = matches
            .get_many::<String>("exclude-dir")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        let pattern_files = matches
            .get_many::<String>("file")
            .map(|values| values.map(PathBuf::from).collect())
            .unwrap_or_default();

        let binary_files = matches
            .get_one::<String>("binary-files")
            .cloned()
            .unwrap_or_else(|| "binary".to_string());

        let encoding = matches
            .get_one::<String>("encoding")
            .cloned()
            .unwrap_or_else(|| "auto".to_string());

        // Handle filename display logic
        let (with_filename, no_filename) = if matches.get_flag("with-filename") {
            (true, false)
        } else if matches.get_flag("no-filename") {
            (false, true)
        } else {
            // Auto-detect based on number of files
            let auto_with_filename = files.len() > 1 || matches.get_flag("recursive");
            (auto_with_filename, !auto_with_filename)
        };

        Ok(GrepConfig {
            pattern,
            files,
            // Pattern options
            ignore_case: matches.get_flag("ignore-case"),
            word_regexp: matches.get_flag("word-regexp"),
            line_regexp: matches.get_flag("line-regexp"),
            fixed_strings: matches.get_flag("fixed-strings"),
            basic_regexp: matches.get_flag("basic-regexp"),
            extended_regexp: matches.get_flag("extended-regexp"),
            perl_regexp: matches.get_flag("perl-regexp"),
            // Output options
            line_number: matches.get_flag("line-number"),
            count_only: matches.get_flag("count"),
            files_with_matches: matches.get_flag("files-with-matches"),
            files_without_match: matches.get_flag("files-without-match"),
            with_filename,
            no_filename,
            only_matching: matches.get_flag("only-matching"),
            // Context options
            before_context,
            after_context,
            // Search behavior
            recursive: matches.get_flag("recursive") || matches.get_flag("dereference-recursive"),
            follow_symlinks: matches.get_flag("dereference-recursive"),
            include_patterns,
            exclude_patterns,
            exclude_dirs,
            // Selection and inversion
            invert_match: matches.get_flag("invert-match"),
            max_count,
            // Binary and encoding
            binary_files,
            text_mode: matches.get_flag("text"),
            null_data: matches.get_flag("null-data"),
            encoding,
            windows_encoding: matches.get_flag("windows-encoding"),
            handle_crlf: matches.get_flag("crlf"),
            // Performance
            parallel_threads,
            use_mmap: matches.get_flag("mmap"),
            // Output format
            color_mode,
            null_separator: matches.get_flag("null"),
            byte_offset: matches.get_flag("byte-offset"),
            quiet: matches.get_flag("quiet"),
            path_format,
            // Pattern files
            pattern_files,
        })
    }

    /// Check if this is a simple search (single file, no special options)
    pub fn is_simple_search(&self) -> bool {
        self.files.len() <= 1
            && !self.recursive
            && self.before_context == 0
            && self.after_context == 0
            && !self.count_only
            && !self.files_with_matches
            && !self.files_without_match
            && self.max_count.is_none()
    }

    /// Get effective thread count
    pub fn effective_thread_count(&self) -> usize {
        self.parallel_threads.unwrap_or_else(num_cpus::get)
    }

    /// Should we search recursively?
    pub fn should_search_recursively(&self) -> bool {
        self.recursive || (!self.files.is_empty() && self.files.iter().any(|f| {
            std::path::Path::new(f).is_dir()
        }))
    }

    /// Should we show filename in output?
    pub fn should_show_filename(&self) -> bool {
        self.with_filename && !self.no_filename
    }
}

impl Default for GrepConfig {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            files: Vec::new(),
            // Pattern options
            ignore_case: false,
            word_regexp: false,
            line_regexp: false,
            fixed_strings: false,
            basic_regexp: false,
            extended_regexp: false,
            perl_regexp: false,
            // Output options
            line_number: false,
            count_only: false,
            files_with_matches: false,
            files_without_match: false,
            with_filename: false,
            no_filename: true,
            only_matching: false,
            // Context options
            before_context: 0,
            after_context: 0,
            // Search behavior
            recursive: false,
            follow_symlinks: false,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            exclude_dirs: Vec::new(),
            // Selection and inversion
            invert_match: false,
            max_count: None,
            // Binary and encoding
            binary_files: "binary".to_string(),
            text_mode: false,
            null_data: false,
            encoding: "auto".to_string(),
            windows_encoding: false,
            handle_crlf: false,
            // Performance
            parallel_threads: None,
            use_mmap: false,
            // Output format
            color_mode: ColorMode::Auto,
            null_separator: false,
            byte_offset: false,
            quiet: false,
            path_format: PathFormat::Auto,
            // Pattern files
            pattern_files: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;

    fn create_test_command() -> Command {
        Command::new("grep")
            .arg(clap::Arg::new("pattern").required(true))
            .arg(clap::Arg::new("files").action(clap::ArgAction::Append))
            .arg(clap::Arg::new("ignore-case").short('i').action(clap::ArgAction::SetTrue))
            .arg(clap::Arg::new("count").short('c').action(clap::ArgAction::SetTrue))
    }

    #[test]
    fn test_basic_config_parsing() {
        let cmd = create_test_command();
        let matches = cmd.try_get_matches_from(vec!["grep", "pattern", "file.txt"]).unwrap();
        let config = GrepConfig::from_matches(&matches).unwrap();

        assert_eq!(config.pattern, "pattern");
        assert_eq!(config.files, vec!["file.txt"]);
        assert!(!config.ignore_case);
        assert!(!config.count_only);
    }

    #[test]
    fn test_case_insensitive_config() {
        let cmd = create_test_command();
        let matches = cmd.try_get_matches_from(vec!["grep", "-i", "pattern"]).unwrap();
        let config = GrepConfig::from_matches(&matches).unwrap();

        assert!(config.ignore_case);
    }

    #[test]
    fn test_count_mode_config() {
        let cmd = create_test_command();
        let matches = cmd.try_get_matches_from(vec!["grep", "-c", "pattern"]).unwrap();
        let config = GrepConfig::from_matches(&matches).unwrap();

        assert!(config.count_only);
    }
}
