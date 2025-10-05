use anyhow::{Context, Result};
use clap::Parser;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use winpath::normalize_path;

#[cfg(windows)]
use {
    dunce::canonicalize,
    path_slash::PathBufExt,
};

/// Cross-platform which command with Windows enhancements
#[derive(Parser)]
#[command(name = "which")]
#[command(about = "Find executables in PATH")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
struct Args {
    /// Show all matches in PATH (not just the first)
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Silent mode - exit code only (don't print anything)
    #[arg(short = 's', long = "silent")]
    silent: bool,

    /// Skip shell aliases (placeholder for future shell integration)
    #[arg(long = "skip-alias")]
    skip_alias: bool,

    /// Skip shell functions (placeholder for future shell integration)
    #[arg(long = "skip-functions")]
    skip_functions: bool,

    /// Read paths from stdin (one per line)
    #[arg(long = "read-alias")]
    read_alias: bool,

    /// Commands to find
    commands: Vec<String>,
}

fn main() {
    let args = Args::parse();

    if args.commands.is_empty() && !args.read_alias {
        eprintln!("Error: No commands specified");
        process::exit(1);
    }

    let mut exit_code = 0;
    let mut found_any = false;

    // Handle reading from stdin
    if args.read_alias {
        if let Err(e) = handle_stdin_input(&args) {
            if !args.silent {
                eprintln!("Error reading from stdin: {}", e);
            }
            process::exit(1);
        }
        return;
    }

    // Process each command
    for command in &args.commands {
        let results = find_command(command, args.all);

        match results {
            Ok(paths) => {
                if paths.is_empty() {
                    exit_code = 1;
                } else {
                    found_any = true;
                    if !args.silent {
                        for path in paths {
                            println!("{}", path.display());
                            if !args.all {
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if !args.silent {
                    eprintln!("Error finding '{}': {}", command, e);
                }
                exit_code = 1;
            }
        }
    }

    if !found_any && args.commands.len() == 1 {
        exit_code = 1;
    }

    process::exit(exit_code);
}

fn handle_stdin_input(args: &Args) -> Result<()> {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let mut found_any = false;

    for line in stdin.lock().lines() {
        let command = line.context("Failed to read line from stdin")?;
        let command = command.trim();

        if command.is_empty() {
            continue;
        }

        let results = find_command(command, args.all)?;

        if !results.is_empty() {
            found_any = true;
            if !args.silent {
                for path in results {
                    println!("{}", path.display());
                    if !args.all {
                        break;
                    }
                }
            }
        }
    }

    if !found_any {
        process::exit(1);
    }

    Ok(())
}

fn find_command(command: &str, find_all: bool) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    // On Windows, check current directory first (Windows convention)
    #[cfg(windows)]
    {
        if let Some(path) = check_current_directory(command)? {
            results.push(path);
            if !find_all {
                return Ok(results);
            }
        }
    }

    // Use the which crate for standard PATH searching
    if find_all {
        // Find all instances
        match which::which_all(command) {
            Ok(iter) => {
                let paths: Vec<PathBuf> = iter.collect();
                // Filter out duplicates that might come from current directory check
                #[cfg(windows)]
                {
                    for path in paths {
                        if !results.iter().any(|existing| paths_equal(existing, &path)) {
                            results.push(path);
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    results.extend(paths);
                }
            }
            Err(_) => {
                // No matches found in PATH
            }
        }
    } else {
        // Find first instance
        if results.is_empty() { // Only search PATH if not found in current directory
            match which::which(command) {
                Ok(path) => results.push(path),
                Err(_) => {
                    // No match found
                }
            }
        }
    }

    Ok(results)
}

#[cfg(windows)]
fn check_current_directory(command: &str) -> Result<Option<PathBuf>> {
    let current_dir_raw = env::current_dir().context("Failed to get current directory")?;

    // Normalize the current directory to handle Git Bash mangled paths
    let current_dir = match normalize_path(&current_dir_raw.to_string_lossy()) {
        Ok(normalized) => PathBuf::from(normalized),
        Err(_) => current_dir_raw, // Fallback to original on normalization failure
    };

    // Get PATHEXT or use default Windows executable extensions
    let pathext = env::var("PATHEXT").unwrap_or_else(|_| {
        ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.PS1".to_string()
    });

    let extensions: Vec<&str> = pathext.split(';').collect();

    // First try the command as-is
    let direct_path = current_dir.join(command);
    if direct_path.is_file() && is_executable(&direct_path) {
        return Ok(Some(normalize_windows_path(&direct_path)?));
    }

    // Try with each extension
    for ext in extensions {
        if !ext.is_empty() {
            let ext = if ext.starts_with('.') { ext } else { &format!(".{}", ext) };
            let path_with_ext = current_dir.join(format!("{}{}", command, ext));
            if path_with_ext.is_file() && is_executable(&path_with_ext) {
                return Ok(Some(normalize_windows_path(&path_with_ext)?));
            }
        }
    }

    Ok(None)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    // On Windows, check if file has executable extension or is explicitly executable
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_uppercase();
        matches!(ext.as_str(), "EXE" | "COM" | "BAT" | "CMD" | "PS1" | "VBS" | "VBE" | "JS" | "JSE" | "WSF" | "WSH")
    } else {
        false
    }
}

#[cfg(not(windows))]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = path.metadata() {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(windows)]
fn normalize_windows_path(path: &Path) -> Result<PathBuf> {
    // First try winpath normalization for Git Bash compatibility
    if let Ok(normalized) = normalize_path(&path.to_string_lossy()) {
        return Ok(PathBuf::from(normalized));
    }

    // Fallback to dunce for canonical path without UNC prefixes
    match canonicalize(path) {
        Ok(canonical) => {
            // Convert to forward slashes for consistent output
            Ok(canonical.to_slash_lossy().into_owned().into())
        }
        Err(_) => {
            // Fallback to original path if canonicalization fails
            Ok(path.to_path_buf())
        }
    }
}

#[cfg(windows)]
fn paths_equal(a: &Path, b: &Path) -> bool {
    // Case-insensitive comparison on Windows
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_find_command_nonexistent() {
        let results = find_command("definitely_does_not_exist_12345", false).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_command_existing() {
        // This test assumes 'cargo' is in PATH on most development systems
        let results = find_command("cargo", false).unwrap();
        if !results.is_empty() {
            assert!(results[0].is_file());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_extensions() {
        // Test that Windows executable extensions are recognized
        assert!(is_executable(Path::new("test.exe")));
        assert!(is_executable(Path::new("test.bat")));
        assert!(is_executable(Path::new("test.cmd")));
        assert!(is_executable(Path::new("test.ps1")));
        assert!(!is_executable(Path::new("test.txt")));
    }

    #[test]
    fn test_all_flag() {
        // Test finding all instances
        let results = find_command("cargo", true).unwrap();
        // Should find at least one instance of cargo
        if !results.is_empty() {
            assert!(results.iter().all(|p| p.is_file()));
        }
    }
}
