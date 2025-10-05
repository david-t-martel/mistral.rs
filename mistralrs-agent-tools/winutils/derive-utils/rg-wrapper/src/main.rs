//! # rg - Fast Grep with Windows Path Integration
//!
//! A high-performance text search utility that combines the speed of ripgrep with
//! comprehensive Windows path normalization and cross-platform compatibility.

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use env_logger;
use log::{debug, info};
use rg_wrapper::{
    EncodingOptions, LineEndingMode, OutputFormatter, SearchOptions, TextSearcher
};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use termcolor::ColorChoice;

#[derive(Parser)]
#[command(
    name = "rg",
    version = env!("CARGO_PKG_VERSION"),
    about = "Fast grep replacement with Windows path normalization",
    long_about = "A fast and user-friendly alternative to 'grep' with built-in support for
Windows, Git Bash, WSL, and Cygwin path formats. Provides blazing fast text search
with smart encoding detection and parallel processing.",
    after_help = "EXAMPLES:
    rg pattern
    rg -i case_insensitive_pattern
    rg -n -C 3 pattern                # Show line numbers with 3 lines of context
    rg -t rust \"TODO|FIXME\"           # Search in Rust files for TODO or FIXME
    rg -e pattern1 -e pattern2        # Multiple patterns
    rg -v pattern                     # Invert match (show non-matching lines)
    rg --json pattern > results.json  # JSON output
    rg --files-with-matches pattern   # Only show file names with matches
    rg --to gitbash pattern C:\\\\src     # Output paths in Git Bash format"
)]
struct Args {
    /// Search pattern(s)
    #[arg(value_name = "PATTERN")]
    patterns: Vec<String>,

    /// Search paths (defaults to current directory)
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Additional patterns to search for
    #[arg(short = 'e', long = "regexp")]
    extra_patterns: Vec<String>,

    /// Use regex patterns (default for compatibility)
    #[arg(long = "regex")]
    regex: bool,

    /// Disable regex patterns (literal search)
    #[arg(short = 'F', long = "fixed-strings")]
    fixed_strings: bool,

    /// Case-insensitive matching
    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,

    /// Case-sensitive matching (overrides ignore-case)
    #[arg(short = 's', long = "case-sensitive")]
    case_sensitive: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    word_regexp: bool,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    line_number: bool,

    /// Don't show line numbers
    #[arg(short = 'N', long = "no-line-number")]
    no_line_number: bool,

    /// Show column numbers
    #[arg(long = "column")]
    column: bool,

    /// Lines of context before matches
    #[arg(short = 'B', long = "before-context")]
    before_context: Option<usize>,

    /// Lines of context after matches
    #[arg(short = 'A', long = "after-context")]
    after_context: Option<usize>,

    /// Lines of context before and after matches
    #[arg(short = 'C', long = "context")]
    context: Option<usize>,

    /// Include binary files in search
    #[arg(short = 'a', long = "text")]
    text: bool,

    /// Include hidden files and directories
    #[arg(long = "hidden")]
    hidden: bool,

    /// Follow symbolic links
    #[arg(long = "follow")]
    follow: bool,

    /// Maximum search depth
    #[arg(long = "max-depth")]
    max_depth: Option<usize>,

    /// File types to include
    #[arg(short = 't', long = "type")]
    file_types: Vec<String>,

    /// File types to exclude
    #[arg(short = 'T', long = "type-not")]
    exclude_types: Vec<String>,

    /// File extensions to include
    #[arg(short = 'g', long = "glob")]
    globs: Vec<String>,

    /// Maximum file size to search
    #[arg(long = "max-filesize")]
    max_filesize: Option<String>,

    /// Number of search threads
    #[arg(short = 'j', long = "threads")]
    threads: Option<usize>,

    /// Color output
    #[arg(long = "color", value_enum, default_value = "auto")]
    color: ColorChoiceArg,

    /// Show only file names with matches
    #[arg(short = 'l', long = "files-with-matches")]
    files_with_matches: bool,

    /// Show only file names without matches
    #[arg(long = "files-without-match")]
    files_without_match: bool,

    /// Count matches per file
    #[arg(short = 'c', long = "count")]
    count: bool,

    /// Count total matches across all files
    #[arg(long = "count-matches")]
    count_matches: bool,

    /// Maximum number of matches per file
    #[arg(short = 'm', long = "max-count")]
    max_count: Option<usize>,

    /// Invert matching (show non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    invert_match: bool,

    /// Quiet mode (suppress normal output)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Suppress file names in output
    #[arg(short = 'h', long = "no-filename")]
    no_filename: bool,

    /// Force file names in output
    #[arg(short = 'H', long = "with-filename")]
    with_filename: bool,

    /// Don't respect .gitignore files
    #[arg(long = "no-ignore")]
    no_ignore: bool,

    /// Output format
    #[arg(long = "format", value_enum)]
    format: Option<OutputFormatArg>,

    /// JSON output format
    #[arg(long = "json")]
    json: bool,

    /// Output path format context
    #[arg(long = "to", value_enum)]
    output_context: Option<ContextArg>,

    /// File encoding
    #[arg(long = "encoding")]
    encoding: Option<String>,

    /// Line ending mode
    #[arg(long = "crlf")]
    crlf: bool,

    /// Verbose output
    #[arg(short = 'V', long = "verbose")]
    verbose: bool,

    /// Show version
    #[arg(long = "version")]
    version: bool,

    /// Show statistics
    #[arg(long = "stats")]
    stats: bool,
}

#[derive(Clone, ValueEnum)]
enum ColorChoiceArg {
    Auto,
    Always,
    Never,
}

impl From<ColorChoiceArg> for ColorChoice {
    fn from(choice: ColorChoiceArg) -> Self {
        match choice {
            ColorChoiceArg::Auto => ColorChoice::Auto,
            ColorChoiceArg::Always => ColorChoice::Always,
            ColorChoiceArg::Never => ColorChoice::Never,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum OutputFormatArg {
    Default,
    Json,
    Csv,
}

#[derive(Clone, ValueEnum)]
enum ContextArg {
    Windows,
    Gitbash,
    Wsl,
    Cygwin,
    Auto,
}


fn main() -> Result<()> {
    let args = Args::parse();

    if args.version {
        println!("rg-wrapper {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Initialize logging
    if !args.quiet {
        let mut builder = env_logger::Builder::new();
        if args.verbose {
            builder.filter_level(log::LevelFilter::Debug);
        } else {
            builder.filter_level(log::LevelFilter::Info);
        }
        builder.init();
    }

    // Validate arguments
    if args.patterns.is_empty() && args.extra_patterns.is_empty() {
        eprintln!("Error: No search pattern provided");
        std::process::exit(1);
    }

    // Build search options
    let mut options = SearchOptions::new();

    // Combine all patterns
    let mut all_patterns = args.patterns.clone();
    all_patterns.extend(args.extra_patterns);

    // For multiple patterns, create a regex OR pattern
    let pattern = if all_patterns.len() == 1 {
        all_patterns[0].clone()
    } else {
        // Join patterns with OR operator
        format!("({})", all_patterns.join("|"))
    };

    options = options.pattern(pattern);

    // Set regex mode
    if args.fixed_strings {
        options = options.regex(false);
    } else {
        options = options.regex(!args.fixed_strings || args.regex);
    }

    // Set case sensitivity
    if args.case_sensitive {
        options = options.case_insensitive(false);
    } else if args.ignore_case {
        options = options.case_insensitive(true);
    }

    // Set word matching
    if args.word_regexp {
        options = options.whole_words(true);
    }

    // Set context lines
    if let Some(context) = args.context {
        options = options.context_lines(context);
    } else {
        if let Some(before) = args.before_context {
            options = options.context_before(before);
        }
        if let Some(after) = args.after_context {
            options = options.context_after(after);
        }
    }

    // Set binary file handling
    if args.text {
        options = options.include_binary(true);
    }

    // Set file type filters
    if !args.file_types.is_empty() {
        options.file_types = args.file_types.into_iter().collect();
    }

    // Set file extension filters from globs
    if !args.globs.is_empty() {
        let extensions: HashSet<String> = args.globs.iter()
            .filter_map(|glob| {
                if glob.starts_with("*.") {
                    Some(glob[2..].to_string())
                } else {
                    None
                }
            })
            .collect();
        options.file_extensions = extensions;
    }

    // Set file size limit
    if let Some(size_str) = args.max_filesize {
        if let Ok(size) = parse_file_size(&size_str) {
            options = options.max_file_size(size);
        } else {
            eprintln!("Warning: Invalid file size format: {}", size_str);
        }
    }

    // Set thread count
    if let Some(threads) = args.threads {
        options = options.threads(threads);
    }

    // Set display options
    options.color = args.color.into();

    if args.no_line_number {
        options.show_line_numbers = false;
    } else if args.line_number {
        options.show_line_numbers = true;
    }

    if args.column {
        options.show_column_numbers = true;
    }

    if args.no_filename {
        options.show_file_names = false;
    } else if args.with_filename {
        options.show_file_names = true;
    }

    options.files_with_matches = args.files_with_matches;
    options.files_without_matches = args.files_without_match;
    options.count_matches = args.count;

    if let Some(max_count) = args.max_count {
        options.max_matches_per_file = Some(max_count);
    }

    options.invert_match = args.invert_match;

    // Set path options
    options.include_hidden = args.hidden;
    options.follow_links = args.follow;
    options.max_depth = args.max_depth;
    options.respect_gitignore = !args.no_ignore;

    if let Some(context) = args.output_context {
        options = options.output_context(context.into());
    }

    // Set encoding options
    if let Some(encoding_name) = args.encoding {
        let mut encoding_opts = EncodingOptions::default();
        encoding_opts.default_encoding = encoding_name;
        encoding_opts.auto_detect = false;
        options.encoding_options = encoding_opts;
    }

    // Set line ending mode
    if args.crlf {
        options.line_ending_mode = LineEndingMode::Windows;
    }

    // Create searcher
    let searcher = TextSearcher::new(options.clone())
        .context("Failed to create text searcher")?;

    // Determine search paths
    let search_paths = if args.paths.is_empty() {
        vec![std::env::current_dir().context("Failed to get current directory")?]
    } else {
        args.paths
    };

    // Perform search
    let start_time = std::time::Instant::now();
    let mut total_matches = 0;
    let mut total_files = 0;

    for search_path in &search_paths {
        debug!("Searching in: {:?}", search_path);

        if search_path.is_file() {
            // Search single file
            match searcher.search_file(search_path) {
                Ok(result) => {
                    total_files += 1;
                    total_matches += result.total_matches;
                    output_file_result(&result, &args, &options)?;
                }
                Err(e) => {
                    if !args.quiet {
                        eprintln!("Error searching file {:?}: {}", search_path, e);
                    }
                }
            }
        } else {
            // Search directory
            match searcher.search_directory(search_path) {
                Ok(results) => {
                    total_files += results.len();
                    for result in &results {
                        total_matches += result.total_matches;
                        output_file_result(result, &args, &options)?;
                    }
                }
                Err(e) => {
                    if !args.quiet {
                        eprintln!("Error searching directory {:?}: {}", search_path, e);
                    }
                }
            }
        }
    }

    // Handle count-only output
    if args.count_matches {
        println!("{}", total_matches);
    }

    // Show statistics if requested
    if args.stats && !args.quiet {
        let elapsed = start_time.elapsed();
        eprintln!(
            "Searched {} files with {} matches in {:.2}s",
            total_files,
            total_matches,
            elapsed.as_secs_f64()
        );
    }

    // Exit with appropriate code
    if total_matches == 0 && !args.files_without_match {
        std::process::exit(1);
    }

    Ok(())
}

fn output_file_result(
    result: &rg_wrapper::FileSearchResult,
    args: &Args,
    options: &SearchOptions,
) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Handle special output modes
    if args.files_with_matches {
        if result.total_matches > 0 {
            writeln!(handle, "{}", result.file_path.display())?;
        }
        return Ok(());
    }

    if args.files_without_match {
        if result.total_matches == 0 {
            writeln!(handle, "{}", result.file_path.display())?;
        }
        return Ok(());
    }

    if args.count {
        if options.show_file_names {
            write!(handle, "{}:", result.file_path.display())?;
        }
        writeln!(handle, "{}", result.total_matches)?;
        return Ok(());
    }

    // Handle JSON output
    if args.json || args.format.as_ref() == Some(&OutputFormatArg::Json) {
        let json = serde_json::to_string(result)?;
        writeln!(handle, "{}", json)?;
        return Ok(());
    }

    // Standard output
    if !args.quiet && result.total_matches > 0 {
        let formatter = OutputFormatter::new(options);
        for match_result in &result.matches {
            formatter.format_result(match_result, &mut handle)?;
        }
    }

    Ok(())
}

fn parse_file_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_lowercase();

    // Parse unit suffix
    let (number_part, multiplier) = if size_str.ends_with("b") {
        (&size_str[..size_str.len()-1], 1)
    } else if size_str.ends_with("k") || size_str.ends_with("kb") {
        let end = if size_str.ends_with("kb") { 2 } else { 1 };
        (&size_str[..size_str.len()-end], 1_024)
    } else if size_str.ends_with("m") || size_str.ends_with("mb") {
        let end = if size_str.ends_with("mb") { 2 } else { 1 };
        (&size_str[..size_str.len()-end], 1_024 * 1_024)
    } else if size_str.ends_with("g") || size_str.ends_with("gb") {
        let end = if size_str.ends_with("gb") { 2 } else { 1 };
        (&size_str[..size_str.len()-end], 1_024 * 1_024 * 1_024)
    } else {
        (size_str.as_str(), 1)
    };

    let number: u64 = number_part.parse()
        .with_context(|| format!("Invalid number in file size: {}", number_part))?;

    Ok(number * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_size() {
        assert_eq!(parse_file_size("100").unwrap(), 100);
        assert_eq!(parse_file_size("1k").unwrap(), 1024);
        assert_eq!(parse_file_size("1kb").unwrap(), 1024);
        assert_eq!(parse_file_size("1m").unwrap(), 1024 * 1024);
        assert_eq!(parse_file_size("1mb").unwrap(), 1024 * 1024);
        assert_eq!(parse_file_size("1g").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_file_size("1gb").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_pattern_combination() {
        // Test that multiple patterns are combined correctly
        let patterns = vec!["TODO".to_string(), "FIXME".to_string()];
        let combined = if patterns.len() == 1 {
            patterns[0].clone()
        } else {
            format!("({})", patterns.join("|"))
        };
        assert_eq!(combined, "(TODO|FIXME)");
    }
}
