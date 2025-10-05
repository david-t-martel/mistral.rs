//! # winpath - Windows Path Normalization CLI
//!
//! Command-line tool for normalizing paths across Windows, Git Bash, WSL, and Cygwin environments.

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use env_logger;
use log::{debug, info};
use std::io::{self, BufRead, BufReader, Write};
use winpath::{NormalizeOptions, PathContext, PathNormalizer};

#[derive(Parser)]
#[command(
    name = "winpath",
    version = env!("CARGO_PKG_VERSION"),
    about = "Windows path normalization tool for cross-platform compatibility",
    long_about = "Normalize paths between Windows, Git Bash, WSL, and Cygwin formats.
Supports batch processing, environment variable expansion, and symlink resolution.",
    after_help = "EXAMPLES:
    winpath /c/Users/test
    winpath --to windows /mnt/c/Users/test
    echo \"/c/Users/test\" | winpath --stdin
    winpath --batch paths.txt
    winpath --env \"$HOME/documents\" --to gitbash"
)]
struct Args {
    /// Path(s) to normalize
    #[arg(value_name = "PATH")]
    paths: Vec<String>,

    /// Target context for normalization
    #[arg(short = 't', long = "to", value_enum, default_value = "windows")]
    target: ContextArg,

    /// Source context (auto-detect if not specified)
    #[arg(short = 'f', long = "from", value_enum)]
    source: Option<ContextArg>,

    /// Read paths from stdin (one per line)
    #[arg(long = "stdin")]
    stdin: bool,

    /// Read paths from file (one per line)
    #[arg(short = 'b', long = "batch", value_name = "FILE")]
    batch_file: Option<String>,

    /// Resolve symbolic links
    #[arg(short = 'r', long = "resolve-symlinks")]
    resolve_symlinks: bool,

    /// Expand environment variables
    #[arg(short = 'e', long = "env")]
    expand_env: bool,

    /// Use long path format (\\?\ prefix on Windows)
    #[arg(short = 'l', long = "long-paths")]
    long_paths: bool,

    /// Don't normalize case
    #[arg(long = "no-case")]
    no_case: bool,

    /// Don't clean redundant path components
    #[arg(long = "no-clean")]
    no_clean: bool,

    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value = "path")]
    output: OutputFormat,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Quiet mode (only output results)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Null-terminated output (useful for xargs -0)
    #[arg(short = '0', long = "null")]
    null_terminated: bool,

    /// Show detected context information
    #[arg(long = "show-context")]
    show_context: bool,

    /// Test if paths exist before normalization
    #[arg(long = "check-exists")]
    check_exists: bool,

    /// Exit with error if any path doesn't exist
    #[arg(long = "require-exists")]
    require_exists: bool,
}

#[derive(Clone, ValueEnum)]
enum ContextArg {
    Windows,
    Gitbash,
    Wsl,
    Cygwin,
    Auto,
}

impl From<ContextArg> for PathContext {
    fn from(ctx: ContextArg) -> Self {
        match ctx {
            ContextArg::Windows => PathContext::Windows,
            ContextArg::Gitbash => PathContext::GitBash,
            ContextArg::Wsl => PathContext::WSL,
            ContextArg::Cygwin => PathContext::Cygwin,
            ContextArg::Auto => PathContext::Auto,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Just the normalized path
    Path,
    /// JSON format with metadata
    Json,
    /// Detailed format with source and target info
    Detailed,
    /// Tab-separated values
    Tsv,
}

fn main() -> Result<()> {
    let args = Args::parse();

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

    // Create normalizer with appropriate context
    let normalizer = if let Some(source) = args.source {
        PathNormalizer::with_context(source.into())
    } else {
        PathNormalizer::new()
    };

    // Show context information if requested
    if args.show_context {
        show_context_info(&normalizer, &args)?;
    }

    // Collect paths from various sources
    let paths = collect_paths(&args)?;

    if paths.is_empty() {
        if !args.quiet {
            eprintln!("No paths provided. Use --help for usage information.");
        }
        return Ok(());
    }

    // Process paths
    process_paths(&normalizer, &paths, &args)?;

    Ok(())
}

fn show_context_info(normalizer: &PathNormalizer, args: &Args) -> Result<()> {
    let current_context = normalizer.current_context();
    let detected_context = PathNormalizer::detect_context();

    if args.output == OutputFormat::Json {
        let info = serde_json::json!({
            "current_context": format!("{:?}", current_context),
            "detected_context": format!("{:?}", detected_context),
            "target_context": format!("{:?}", PathContext::from(args.target.clone())),
            "version": env!("CARGO_PKG_VERSION")
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Current context: {:?}", current_context);
        println!("Detected context: {:?}", detected_context);
        println!("Target context: {:?}", PathContext::from(args.target.clone()));
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
    }

    Ok(())
}

fn collect_paths(args: &Args) -> Result<Vec<String>> {
    let mut paths = Vec::new();

    // Add command line paths
    paths.extend(args.paths.clone());

    // Read from stdin if requested
    if args.stdin {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        for line in reader.lines() {
            let line = line.context("Failed to read line from stdin")?;
            if !line.trim().is_empty() {
                paths.push(line.trim().to_string());
            }
        }
    }

    // Read from batch file if specified
    if let Some(batch_file) = &args.batch_file {
        let file = std::fs::File::open(batch_file)
            .with_context(|| format!("Failed to open batch file: {}", batch_file))?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.context("Failed to read line from batch file")?;
            if !line.trim().is_empty() && !line.trim().starts_with('#') {
                paths.push(line.trim().to_string());
            }
        }
    }

    Ok(paths)
}

fn process_paths(normalizer: &PathNormalizer, paths: &[String], args: &Args) -> Result<()> {
    let options = NormalizeOptions {
        target_context: args.target.clone().into(),
        resolve_symlinks: args.resolve_symlinks,
        expand_env_vars: args.expand_env,
        use_long_paths: args.long_paths,
        normalize_case: !args.no_case,
        clean_path: !args.no_clean,
    };

    let mut stdout = io::stdout();
    let terminator = if args.null_terminated { "\0" } else { "\n" };

    for (index, path) in paths.iter().enumerate() {
        debug!("Processing path {}: '{}'", index + 1, path);

        // Check if path exists if requested
        if args.check_exists || args.require_exists {
            let path_for_check = if args.expand_env {
                normalizer.normalize_with_options(path, &options)
                    .unwrap_or_else(|_| std::path::PathBuf::from(path))
            } else {
                std::path::PathBuf::from(path)
            };

            if !normalizer.path_exists(&path_for_check) {
                if args.require_exists {
                    return Err(anyhow::anyhow!("Path does not exist: {}", path));
                }
                if !args.quiet {
                    eprintln!("Warning: Path does not exist: {}", path);
                }
            }
        }

        // Normalize the path
        match normalizer.normalize_with_options(path, &options) {
            Ok(normalized) => {
                match args.output {
                    OutputFormat::Path => {
                        print!("{}{}", normalized.display(), terminator);
                    }
                    OutputFormat::Json => {
                        let result = serde_json::json!({
                            "original": path,
                            "normalized": normalized.display().to_string(),
                            "exists": normalizer.path_exists(&normalized),
                            "is_absolute": normalizer.is_absolute(&normalized),
                            "extension": normalizer.get_extension(&normalized)
                        });
                        println!("{}", serde_json::to_string(&result)?);
                    }
                    OutputFormat::Detailed => {
                        println!("Original:   {}", path);
                        println!("Normalized: {}", normalized.display());
                        println!("Exists:     {}", normalizer.path_exists(&normalized));
                        println!("Absolute:   {}", normalizer.is_absolute(&normalized));
                        if let Some(ext) = normalizer.get_extension(&normalized) {
                            println!("Extension:  {}", ext);
                        }
                        if index < paths.len() - 1 {
                            println!();
                        }
                    }
                    OutputFormat::Tsv => {
                        println!(
                            "{}\t{}\t{}\t{}",
                            path,
                            normalized.display(),
                            normalizer.path_exists(&normalized),
                            normalizer.is_absolute(&normalized)
                        );
                    }
                }
            }
            Err(e) => {
                if !args.quiet {
                    eprintln!("Error normalizing '{}': {}", path, e);
                }
                if args.require_exists {
                    return Err(e);
                }
            }
        }
    }

    stdout.flush().context("Failed to flush stdout")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cli_basic_functionality() {
        let output = Command::new(env!("CARGO_BIN_EXE_winpath"))
            .args(&["/c/Users/test", "--to", "windows"])
            .output()
            .expect("Failed to execute winpath");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("C:\\Users\\test"));
    }

    #[test]
    fn test_cli_stdin_input() {
        let mut child = Command::new(env!("CARGO_BIN_EXE_winpath"))
            .args(&["--stdin", "--to", "gitbash"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn winpath");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(b"C:\\Users\\test\n").expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("/c/Users/test"));
    }

    #[test]
    fn test_cli_batch_file() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file.write_all(b"C:\\Users\\test1\n/c/Users/test2\n").expect("Failed to write to temp file");

        let output = Command::new(env!("CARGO_BIN_EXE_winpath"))
            .args(&["--batch", temp_file.path().to_str().unwrap(), "--to", "windows"])
            .output()
            .expect("Failed to execute winpath");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("C:\\Users\\test1"));
        assert!(stdout.contains("C:\\Users\\test2"));
    }

    #[test]
    fn test_cli_json_output() {
        let output = Command::new(env!("CARGO_BIN_EXE_winpath"))
            .args(&["/c/Users/test", "--output", "json"])
            .output()
            .expect("Failed to execute winpath");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();

        // Should be valid JSON
        let _: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");
    }

    #[test]
    fn test_cli_context_detection() {
        let output = Command::new(env!("CARGO_BIN_EXE_winpath"))
            .args(&["--show-context"])
            .output()
            .expect("Failed to execute winpath");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Current context:"));
        assert!(stdout.contains("Detected context:"));
    }
}
