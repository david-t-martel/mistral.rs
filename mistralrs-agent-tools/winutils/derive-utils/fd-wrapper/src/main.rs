//! # fd - Fast Find with Windows Path Integration
//!
//! A high-performance file search utility that combines the speed of fd with
//! comprehensive Windows path normalization and cross-platform compatibility.

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use env_logger;
use fd_wrapper::{
    EntryType, FileSearcher, SearchOptions, SizeFilter, TimeFilter
};
use log::{debug, info};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Parser)]
#[command(
    name = "fd",
    version = env!("CARGO_PKG_VERSION"),
    about = "Fast find replacement with Windows path normalization",
    long_about = "A fast and user-friendly alternative to 'find' with built-in support for
Windows, Git Bash, WSL, and Cygwin path formats. Provides blazing fast file and directory
search with smart ignore patterns and parallel processing.",
    after_help = "EXAMPLES:
    fd pattern
    fd -t f -e rs *.rs
    fd -H secret                    # Include hidden files
    fd -E target -t d build         # Exclude 'target', find directories named 'build'
    fd -S +1m                       # Files larger than 1MB
    fd --changed-within 1d          # Files modified within last day
    fd -x ls -la                    # Execute 'ls -la' on each result
    fd --to gitbash C:\\\\Users\\\\test   # Output in Git Bash format"
)]
struct Args {
    /// Search pattern (supports glob patterns)
    #[arg(value_name = "PATTERN")]
    pattern: Option<String>,

    /// Search paths (defaults to current directory)
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Entry type to search for
    #[arg(short = 't', long = "type", value_enum)]
    entry_type: Vec<EntryTypeArg>,

    /// File extension to include
    #[arg(short = 'e', long = "extension")]
    extensions: Vec<String>,

    /// Use regex instead of glob patterns
    #[arg(short = 'r', long = "regex")]
    regex: bool,

    /// Case-sensitive matching
    #[arg(short = 's', long = "case-sensitive")]
    case_sensitive: bool,

    /// Include hidden files and directories
    #[arg(short = 'H', long = "hidden")]
    hidden: bool,

    /// Follow symbolic links
    #[arg(short = 'L', long = "follow")]
    follow: bool,

    /// Maximum search depth
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Minimum search depth
    #[arg(long = "min-depth")]
    min_depth: Option<usize>,

    /// Exclude patterns (can be used multiple times)
    #[arg(short = 'E', long = "exclude")]
    exclude: Vec<String>,

    /// Don't respect .gitignore files
    #[arg(long = "no-ignore")]
    no_ignore: bool,

    /// Don't respect .fdignore files
    #[arg(long = "no-ignore-parent")]
    no_ignore_parent: bool,

    /// Search absolute paths
    #[arg(short = 'a', long = "absolute-path")]
    absolute_path: bool,

    /// Number of search threads
    #[arg(short = 'j', long = "threads")]
    threads: Option<usize>,

    /// Size constraints
    #[arg(short = 'S', long = "size")]
    size: Vec<String>,

    /// Files changed within time duration
    #[arg(long = "changed-within")]
    changed_within: Option<String>,

    /// Files changed before time duration
    #[arg(long = "changed-before")]
    changed_before: Option<String>,

    /// Search for executable files only
    #[arg(short = 'X', long = "executable")]
    executable: bool,

    /// Output path format context
    #[arg(long = "to", value_enum)]
    output_context: Option<ContextArg>,

    /// Output format
    #[arg(short = 'F', long = "format", value_enum, default_value = "path")]
    format: OutputFormat,

    /// Print null character after each result
    #[arg(short = '0', long = "print0")]
    print0: bool,

    /// Count total number of matches
    #[arg(short = 'c', long = "count")]
    count: bool,

    /// Show only first match
    #[arg(short = '1', long = "max-results")]
    max_results: Option<usize>,

    /// Execute command for each result
    #[arg(short = 'x', long = "exec")]
    exec: Vec<String>,

    /// Show statistics
    #[arg(long = "stats")]
    stats: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Quiet mode
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Show version
    #[arg(short = 'V', long = "version")]
    version: bool,
}

#[derive(Clone, ValueEnum)]
enum EntryTypeArg {
    #[value(alias = "f")]
    File,
    #[value(alias = "d")]
    Directory,
    #[value(alias = "l")]
    Symlink,
}

impl From<EntryTypeArg> for EntryType {
    fn from(arg: EntryTypeArg) -> Self {
        match arg {
            EntryTypeArg::File => EntryType::File,
            EntryTypeArg::Directory => EntryType::Directory,
            EntryTypeArg::Symlink => EntryType::Symlink,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum ContextArg {
    Windows,
    Gitbash,
    Wsl,
    Cygwin,
    Auto,
}


#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Just the path
    Path,
    /// Detailed information
    Long,
    /// JSON output
    Json,
    /// Tab-separated values
    Tsv,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.version {
        println!("fd-wrapper {}", env!("CARGO_PKG_VERSION"));
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

    // Build search options
    let mut options = SearchOptions::new();

    // Set pattern
    if let Some(pattern) = args.pattern {
        options = options.pattern(pattern);
    }

    // Configure search behavior
    options = options
        .regex(args.regex)
        .case_sensitive(args.case_sensitive)
        .include_hidden(args.hidden)
        .follow_links(args.follow);

    // Set depth limits
    if let Some(max_depth) = args.max_depth {
        options = options.max_depth(max_depth);
    }
    if let Some(min_depth) = args.min_depth {
        options = options.min_depth(min_depth);
    }

    // Set entry types
    if !args.entry_type.is_empty() {
        let types: HashSet<EntryType> = args.entry_type.into_iter()
            .map(|t| t.into())
            .collect();
        options = options.entry_types(types);
    }

    // Set file extensions
    if !args.extensions.is_empty() {
        let extensions: HashSet<String> = args.extensions.into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
        options = options.extensions(extensions);
    }

    // Set size constraints
    if !args.size.is_empty() {
        let size_filter = parse_size_constraints(&args.size)?;
        options = options.size_filter(size_filter);
    }

    // Set time constraints
    let time_filter = parse_time_constraints(&args.changed_within, &args.changed_before)?;
    options = options.time_filter(time_filter);

    // Set thread count
    if let Some(threads) = args.threads {
        options = options.threads(threads);
    }

    // Set output context
    if let Some(context) = args.output_context {
        options = options.output_context(context.into());
    }

    // Set executable filter
    if args.executable {
        options = options.executable_only(true);
    }

    // Set ignore patterns
    options.ignore_patterns.extend(args.exclude);
    options.respect_gitignore = !args.no_ignore;
    options.respect_fdignore = !args.no_ignore_parent;

    // Create searcher
    let searcher = FileSearcher::new(options)
        .context("Failed to create file searcher")?;

    // Determine search paths
    let search_paths = if args.paths.is_empty() {
        vec![std::env::current_dir().context("Failed to get current directory")?]
    } else {
        args.paths
    };

    // Perform search
    let start_time = std::time::Instant::now();
    let mut total_results = 0;

    for search_path in &search_paths {
        debug!("Searching in: {:?}", search_path);

        if args.count {
            let count = searcher.count(search_path)
                .with_context(|| format!("Failed to count in {:?}", search_path))?;
            total_results += count;
        } else {
            let results = searcher.search(search_path)
                .with_context(|| format!("Failed to search in {:?}", search_path))?;

            total_results += results.len();

            // Apply max results limit
            let results_to_show = if let Some(max) = args.max_results {
                &results[..results.len().min(max)]
            } else {
                &results
            };

            // Output results
            for result in results_to_show {
                match args.format {
                    OutputFormat::Path => {
                        if args.absolute_path {
                            let abs_path = result.path.canonicalize()
                                .unwrap_or_else(|_| result.path.clone());
                            print_path(&abs_path, args.print0);
                        } else {
                            print_path(&result.path, args.print0);
                        }
                    }
                    OutputFormat::Long => {
                        print_detailed_result(result, args.print0)?;
                    }
                    OutputFormat::Json => {
                        let json = serde_json::to_string(result)?;
                        if args.print0 {
                            print!("{}\0", json);
                        } else {
                            println!("{}", json);
                        }
                    }
                    OutputFormat::Tsv => {
                        print_tsv_result(result, args.print0)?;
                    }
                }

                // Execute command if specified
                if !args.exec.is_empty() {
                    execute_command(&args.exec, &result.path)?;
                }

                // Early exit if max results reached
                if let Some(max) = args.max_results {
                    if total_results >= max {
                        break;
                    }
                }
            }
        }
    }

    // Show count or statistics
    if args.count {
        println!("{}", total_results);
    }

    if args.stats {
        let elapsed = start_time.elapsed();
        if !args.quiet {
            eprintln!("Found {} results in {:.2}s", total_results, elapsed.as_secs_f64());
        }
    }

    Ok(())
}

fn parse_size_constraints(size_specs: &[String]) -> Result<SizeFilter> {
    let mut filter = SizeFilter::new();

    for spec in size_specs {
        let (op, value_str) = if spec.starts_with('+') {
            ("min", &spec[1..])
        } else if spec.starts_with('-') {
            ("max", &spec[1..])
        } else {
            ("exact", spec.as_str())
        };

        let size = parse_size(value_str)
            .with_context(|| format!("Invalid size specification: {}", spec))?;

        match op {
            "min" => filter = filter.min(size),
            "max" => filter = filter.max(size),
            "exact" => {
                filter = filter.min(size);
                filter = filter.max(size);
            }
            _ => unreachable!(),
        }
    }

    Ok(filter)
}

fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim();

    // Parse unit suffix
    let (number_part, multiplier) = if size_str.ends_with("b") || size_str.ends_with("B") {
        (&size_str[..size_str.len()-1], 1)
    } else if size_str.ends_with("k") || size_str.ends_with("K") {
        (&size_str[..size_str.len()-1], 1_024)
    } else if size_str.ends_with("m") || size_str.ends_with("M") {
        (&size_str[..size_str.len()-1], 1_024 * 1_024)
    } else if size_str.ends_with("g") || size_str.ends_with("G") {
        (&size_str[..size_str.len()-1], 1_024 * 1_024 * 1_024)
    } else if size_str.ends_with("t") || size_str.ends_with("T") {
        (&size_str[..size_str.len()-1], 1_024_u64.pow(4))
    } else {
        (size_str, 1)
    };

    let number: u64 = number_part.parse()
        .with_context(|| format!("Invalid number: {}", number_part))?;

    Ok(number * multiplier)
}

fn parse_time_constraints(changed_within: &Option<String>, changed_before: &Option<String>) -> Result<TimeFilter> {
    let mut filter = TimeFilter::new();

    if let Some(within) = changed_within {
        let duration = parse_duration(within)
            .with_context(|| format!("Invalid duration: {}", within))?;
        let cutoff = SystemTime::now()
            .checked_sub(duration)
            .context("Duration too large")?;
        filter = filter.newer_than(cutoff);
    }

    if let Some(before) = changed_before {
        let duration = parse_duration(before)
            .with_context(|| format!("Invalid duration: {}", before))?;
        let cutoff = SystemTime::now()
            .checked_sub(duration)
            .context("Duration too large")?;
        filter = filter.older_than(cutoff);
    }

    Ok(filter)
}

fn parse_duration(duration_str: &str) -> Result<Duration> {
    let duration_str = duration_str.trim();

    // Parse time unit
    let (number_part, multiplier) = if duration_str.ends_with("s") {
        (&duration_str[..duration_str.len()-1], 1)
    } else if duration_str.ends_with("m") {
        (&duration_str[..duration_str.len()-1], 60)
    } else if duration_str.ends_with("h") {
        (&duration_str[..duration_str.len()-1], 60 * 60)
    } else if duration_str.ends_with("d") {
        (&duration_str[..duration_str.len()-1], 60 * 60 * 24)
    } else if duration_str.ends_with("w") {
        (&duration_str[..duration_str.len()-1], 60 * 60 * 24 * 7)
    } else {
        // Default to days if no unit specified
        (duration_str, 60 * 60 * 24)
    };

    let number: u64 = number_part.parse()
        .with_context(|| format!("Invalid number: {}", number_part))?;

    Ok(Duration::from_secs(number * multiplier))
}

fn print_path(path: &PathBuf, null_terminated: bool) {
    if null_terminated {
        print!("{}\0", path.display());
    } else {
        println!("{}", path.display());
    }
}

fn print_detailed_result(result: &fd_wrapper::SearchResult, null_terminated: bool) -> Result<()> {
    let type_char = match result.entry_type {
        EntryType::File => "f",
        EntryType::Directory => "d",
        EntryType::Symlink => "l",
        EntryType::Other => "?",
    };

    let size_str = result.size
        .map(|s| format!("{:>10}", s))
        .unwrap_or_else(|| "         -".to_string());

    let modified_str = result.modified
        .and_then(|m| {
            use std::time::UNIX_EPOCH;
            m.duration_since(UNIX_EPOCH).ok()
        })
        .map(|d| {
            let secs = d.as_secs();
            let dt = time::OffsetDateTime::from_unix_timestamp(secs as i64).ok()?;
            Some(dt.format(&time::format_description::well_known::Rfc3339).ok()?)
        })
        .flatten()
        .unwrap_or_else(|| "                   -".to_string());

    let output = format!("{} {} {} {}", type_char, size_str, modified_str, result.path.display());

    if null_terminated {
        print!("{}\0", output);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn print_tsv_result(result: &fd_wrapper::SearchResult, null_terminated: bool) -> Result<()> {
    let type_str = format!("{:?}", result.entry_type);
    let size_str = result.size.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
    let path_str = result.path.display().to_string();

    let output = format!("{}\t{}\t{}", type_str, size_str, path_str);

    if null_terminated {
        print!("{}\0", output);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn execute_command(cmd_args: &[String], path: &PathBuf) -> Result<()> {
    if cmd_args.is_empty() {
        return Ok(());
    }

    let mut command = std::process::Command::new(&cmd_args[0]);

    // Add arguments, replacing {} with the path
    for arg in &cmd_args[1..] {
        if arg == "{}" {
            command.arg(path.as_os_str());
        } else {
            command.arg(arg);
        }
    }

    let status = command.status()
        .with_context(|| format!("Failed to execute command: {:?}", cmd_args))?;

    if !status.success() {
        debug!("Command failed with status: {}", status);
    }

    Ok(())
}

// Add time dependency
use time;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("1k").unwrap(), 1024);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("1m").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1g").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("60s").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
        assert_eq!(parse_duration("1").unwrap(), Duration::from_secs(86400)); // defaults to days
    }

    #[test]
    fn test_size_filter() {
        let filter = parse_size_constraints(&["+1k".to_string(), "-1m".to_string()]).unwrap();
        assert!(filter.matches(2048)); // 2k, between 1k and 1m
        assert!(!filter.matches(512)); // 512b, less than 1k
        assert!(!filter.matches(2 * 1024 * 1024)); // 2m, more than 1m
    }
}
