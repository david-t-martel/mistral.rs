//! # Enhanced Grep Utility for Windows
//!
//! A high-performance grep replacement that combines ripgrep's SIMD-accelerated search
//! with GNU grep compatibility and universal Windows path support via winpath integration.
//!
//! ## Features
//! - ripgrep-like performance with SIMD acceleration
//! - GNU grep command-line compatibility
//! - Universal path normalization (DOS, WSL, Cygwin, Git Bash)
//! - Windows-specific text encodings (UTF-16LE, ANSI code pages)
//! - Proper CRLF line ending handling
//! - Binary file detection and handling
//! - Multi-pattern searching with Aho-Corasick
//! - Gitignore-style filtering support

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};
use std::io::{self, Write, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use rayon::prelude::*;
use winpath::{normalize_path, PathNormalizer};
use grep_searcher::{SearcherBuilder, BinaryDetection, LineStep};
use grep_matcher::{Matcher, Match};
use grep_regex::RegexMatcher;
use grep_printer::{StandardBuilder, Standard, PrinterPath};

mod config;
mod matchers;
mod output;
mod search;
mod encodings;
mod patterns;

use config::*;
use matchers::*;
use output::*;
use search::*;
use encodings::*;
use patterns::*;

/// Main entry point for the enhanced grep utility
fn main() -> Result<()> {
    // Initialize error handling
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("grep: fatal error: {}", panic_info);
        std::process::exit(2);
    }));

    let app = build_cli();
    let matches = app.get_matches();

    // Parse configuration from command line arguments
    let config = GrepConfig::from_matches(&matches)?;

    // Initialize path normalizer for winpath integration
    let normalizer = Arc::new(PathNormalizer::new());

    // Execute grep operation
    execute_grep(config, normalizer)
}

/// Build the command-line interface with GNU grep compatibility
fn build_cli() -> Command {
    Command::new("grep")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Search text patterns with enhanced Windows support")
        .long_about("A high-performance grep replacement with GNU grep compatibility,\n\
                     universal Windows path support (DOS/WSL/Cygwin/Git Bash), and\n\
                     ripgrep-like SIMD-accelerated search for optimal performance.")
        .arg(Arg::new("pattern")
            .help("Search pattern (regex or literal string)")
            .required(true)
            .value_name("PATTERN"))
        .arg(Arg::new("files")
            .help("Files to search (default: stdin)")
            .action(ArgAction::Append)
            .value_name("FILE"))
        // Pattern options
        .arg(Arg::new("ignore-case")
            .short('i')
            .long("ignore-case")
            .help("Case-insensitive matching")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("word-regexp")
            .short('w')
            .long("word-regexp")
            .help("Match whole words only")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("line-regexp")
            .short('x')
            .long("line-regexp")
            .help("Match whole lines only")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("fixed-strings")
            .short('F')
            .long("fixed-strings")
            .help("Treat pattern as literal string")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("basic-regexp")
            .short('G')
            .long("basic-regexp")
            .help("Use basic regular expressions")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("extended-regexp")
            .short('E')
            .long("extended-regexp")
            .help("Use extended regular expressions")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("perl-regexp")
            .short('P')
            .long("perl-regexp")
            .help("Use Perl-compatible regular expressions")
            .action(ArgAction::SetTrue))
        // Output format options
        .arg(Arg::new("line-number")
            .short('n')
            .long("line-number")
            .help("Show line numbers")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("count")
            .short('c')
            .long("count")
            .help("Show count of matching lines")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("files-with-matches")
            .short('l')
            .long("files-with-matches")
            .help("Show only names of files with matches")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("files-without-match")
            .short('L')
            .long("files-without-match")
            .help("Show only names of files without matches")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("with-filename")
            .short('H')
            .long("with-filename")
            .help("Show filename with matches")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("no-filename")
            .short('h')
            .long("no-filename")
            .help("Don't show filename with matches")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("only-matching")
            .short('o')
            .long("only-matching")
            .help("Show only matching parts of lines")
            .action(ArgAction::SetTrue))
        // Context options
        .arg(Arg::new("before-context")
            .short('B')
            .long("before-context")
            .help("Lines of context before matches")
            .value_name("NUM"))
        .arg(Arg::new("after-context")
            .short('A')
            .long("after-context")
            .help("Lines of context after matches")
            .value_name("NUM"))
        .arg(Arg::new("context")
            .short('C')
            .long("context")
            .help("Lines of context before and after matches")
            .value_name("NUM"))
        // Search behavior
        .arg(Arg::new("recursive")
            .short('r')
            .long("recursive")
            .help("Search directories recursively")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("dereference-recursive")
            .short('R')
            .long("dereference-recursive")
            .help("Search directories recursively, following symlinks")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("include")
            .long("include")
            .help("Include files matching pattern")
            .value_name("PATTERN")
            .action(ArgAction::Append))
        .arg(Arg::new("exclude")
            .long("exclude")
            .help("Exclude files matching pattern")
            .value_name("PATTERN")
            .action(ArgAction::Append))
        .arg(Arg::new("exclude-dir")
            .long("exclude-dir")
            .help("Exclude directories matching pattern")
            .value_name("PATTERN")
            .action(ArgAction::Append))
        // Selection and inversion
        .arg(Arg::new("invert-match")
            .short('v')
            .long("invert-match")
            .help("Invert match (show non-matching lines)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("max-count")
            .short('m')
            .long("max-count")
            .help("Stop after NUM matches")
            .value_name("NUM"))
        // Binary and encoding options
        .arg(Arg::new("binary-files")
            .long("binary-files")
            .help("How to handle binary files: binary, text, without-match")
            .value_name("TYPE")
            .default_value("binary"))
        .arg(Arg::new("text")
            .short('a')
            .long("text")
            .help("Treat all files as text")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("null-data")
            .short('z')
            .long("null-data")
            .help("Lines are terminated by null character")
            .action(ArgAction::SetTrue))
        // Performance options
        .arg(Arg::new("parallel")
            .short('j')
            .long("parallel")
            .help("Number of parallel threads")
            .value_name("THREADS"))
        .arg(Arg::new("mmap")
            .long("mmap")
            .help("Use memory mapping for large files")
            .action(ArgAction::SetTrue))
        // Color and output
        .arg(Arg::new("color")
            .long("color")
            .help("When to use color: auto, always, never")
            .value_name("WHEN")
            .default_value("auto"))
        .arg(Arg::new("null")
            .short('Z')
            .long("null")
            .help("Output null character after filenames")
            .action(ArgAction::SetTrue))
        // Windows-specific options
        .arg(Arg::new("windows-encoding")
            .long("windows-encoding")
            .help("Handle Windows text encodings (UTF-16LE, ANSI)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("crlf")
            .long("crlf")
            .help("Handle CRLF line endings")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("encoding")
            .long("encoding")
            .help("Text encoding: utf8, utf16le, utf16be, latin1, auto")
            .value_name("ENCODING")
            .default_value("auto"))
        // Path format options
        .arg(Arg::new("path-format")
            .long("path-format")
            .help("Output path format: windows, unix, native, auto")
            .value_name("FORMAT")
            .default_value("auto"))
        // Additional pattern files
        .arg(Arg::new("file")
            .short('f')
            .long("file")
            .help("Read patterns from file")
            .value_name("FILE")
            .action(ArgAction::Append))
        // Advanced options
        .arg(Arg::new("byte-offset")
            .short('b')
            .long("byte-offset")
            .help("Show byte offset of matches")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("quiet")
            .short('q')
            .long("quiet")
            .help("Quiet mode - exit with status only")
            .action(ArgAction::SetTrue))
}

/// Execute the grep operation with the given configuration
fn execute_grep(config: GrepConfig, normalizer: Arc<PathNormalizer>) -> Result<()> {
    // Build the pattern matcher
    let matcher = build_matcher(&config)?;

    // Build the searcher
    let searcher = build_searcher(&config)?;

    // Build the printer
    let printer = build_printer(&config, normalizer.clone())?;

    // Set up parallel execution if enabled
    if let Some(thread_count) = config.parallel_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build_global()
            .context("Failed to initialize thread pool")?;
    }

    // Normalize and process input files
    let input_files = if config.files.is_empty() {
        vec![InputFile::Stdin]
    } else {
        config.files.iter()
            .map(|path| normalize_input_file(path, &normalizer))
            .collect::<Result<Vec<_>>>()?
    };

    // Execute search
    let mut total_matches = 0u64;
    let mut found_match = false;

    if config.recursive {
        // Recursive search
        total_matches = search_recursive(&config, &matcher, &searcher, &printer, &normalizer)?;
    } else {
        // Search individual files
        for input_file in input_files {
            let matches = search_file(&input_file, &config, &matcher, &searcher, &printer)?;
            total_matches += matches;
            if matches > 0 {
                found_match = true;
            }
        }
    }

    // Handle count-only mode
    if config.count_only && !config.recursive {
        println!("{}", total_matches);
    }

    // Set exit code
    if config.quiet {
        std::process::exit(if found_match { 0 } else { 1 });
    } else if !found_match && !config.invert_match {
        std::process::exit(1);
    }

    Ok(())
}

/// Build pattern matcher based on configuration
fn build_matcher(config: &GrepConfig) -> Result<Box<dyn Matcher>> {
    let mut builder = RegexMatcher::new();

    // Configure case sensitivity
    if config.ignore_case {
        builder.case_insensitive(true);
    }

    // Configure pattern type
    if config.fixed_strings {
        builder.fixed_strings(true);
    }

    if config.word_regexp {
        builder.word(true);
    }

    if config.line_regexp {
        builder.line_terminator(Some(b'\n'));
    }

    // Handle multi-line patterns
    if config.null_data {
        builder.line_terminator(Some(b'\0'));
    }

    // Build pattern from various sources
    let pattern = build_pattern(config)?;

    let matcher = builder.build(&pattern)
        .context("Failed to compile pattern")?;

    Ok(Box::new(matcher))
}

/// Build searcher based on configuration
fn build_searcher(config: &GrepConfig) -> Result<grep_searcher::Searcher> {
    let mut builder = SearcherBuilder::new();

    // Configure binary detection
    if config.text_mode {
        builder.binary_detection(BinaryDetection::none());
    } else {
        match config.binary_files.as_str() {
            "binary" => builder.binary_detection(BinaryDetection::quit(b'\x00')),
            "text" => builder.binary_detection(BinaryDetection::none()),
            "without-match" => builder.binary_detection(BinaryDetection::quit(b'\x00')),
            _ => return Err(anyhow::anyhow!("Invalid binary-files option")),
        };
    }

    // Configure memory mapping
    if config.use_mmap {
        builder.memory_map(grep_searcher::MemoryMapChoice::auto());
    }

    // Configure line terminator
    if config.null_data {
        builder.line_terminator(grep_searcher::LineTerminator::byte(b'\0'));
    } else if config.handle_crlf {
        builder.line_terminator(grep_searcher::LineTerminator::crlf());
    }

    // Set encoding if specified
    if let Some(encoding) = detect_encoding(&config.encoding)? {
        builder.encoding(Some(encoding));
    }

    Ok(builder.build())
}

/// Build printer based on configuration
fn build_printer(config: &GrepConfig, normalizer: Arc<PathNormalizer>) -> Result<Box<dyn GrepPrinter>> {
    let color_choice = match config.color_mode {
        ColorMode::Always => termcolor::ColorChoice::Always,
        ColorMode::Never => termcolor::ColorChoice::Never,
        ColorMode::Auto => {
            if atty::is(atty::Stream::Stdout) {
                termcolor::ColorChoice::Auto
            } else {
                termcolor::ColorChoice::Never
            }
        }
    };

    let printer = EnhancedPrinter::new(config, color_choice, normalizer)?;
    Ok(Box::new(printer))
}

/// Normalize input file path using winpath
fn normalize_input_file(path: &str, normalizer: &PathNormalizer) -> Result<InputFile> {
    if path == "-" {
        Ok(InputFile::Stdin)
    } else {
        let normalized = normalizer
            .normalize(path)
            .context("Failed to normalize input file path")?;
        Ok(InputFile::Path(PathBuf::from(normalized.path())))
    }
}

/// Input file enumeration
#[derive(Debug, Clone)]
enum InputFile {
    Stdin,
    Path(PathBuf),
}

impl InputFile {
    fn path(&self) -> Option<&Path> {
        match self {
            InputFile::Stdin => None,
            InputFile::Path(path) => Some(path),
        }
    }
}
