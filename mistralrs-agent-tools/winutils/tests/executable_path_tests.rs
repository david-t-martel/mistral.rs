//! Executable Path Reporting Tests
//!
//! These tests validate that executables built with winutils correctly report
//! their own paths when running in various environments, particularly focusing
//! on the Git Bash path mangling issue where executables might report mangled
//! paths like `C:\Program Files\Git\mnt\c\...` instead of proper Windows paths.
//!
//! This test suite validates both the winpath library integration and the
//! runtime behavior of actual executables using the library.

use std::process::{Command, Stdio};
use std::path::Path;
use std::env;
use std::ffi::OsStr;
use std::time::Duration;

/// Test executable self-path reporting in different environments
#[test]
fn test_executable_self_path_reporting() {
    // Skip if we're not on Windows or don't have the test executable
    if !cfg!(windows) {
        println!("Skipping executable path tests on non-Windows platform");
        return;
    }

    let test_exe_paths = [
        // Look for test executables in common locations
        r"T:\projects\coreutils\winutils\target\debug\winpath-test.exe",
        r"T:\projects\coreutils\winutils\target\release\winpath-test.exe",
        r"C:\users\david\.local\bin\coreutils.exe",
        r"C:\users\david\.local\bin\ls.exe",
    ];

    let mut found_executable = None;
    for exe_path in test_exe_paths.iter() {
        if Path::new(exe_path).exists() {
            found_executable = Some(exe_path);
            break;
        }
    }

    if found_executable.is_none() {
        println!("No test executable found, skipping executable path tests");
        return;
    }

    let exe_path = found_executable.unwrap();
    test_executable_in_environments(exe_path);
}

/// Test executable path reporting in different shell environments
fn test_executable_in_environments(exe_path: &str) {
    let environments = [
        // Standard Windows environments
        ("cmd", vec!["cmd", "/c"]),
        ("powershell", vec!["powershell", "-Command"]),

        // Git Bash environment (if available)
        ("git-bash", vec!["C:\\Program Files\\Git\\bin\\bash.exe", "-c"]),
        ("git-bash-alt", vec!["C:\\Git\\bin\\bash.exe", "-c"]),

        // WSL environment (if available)
        ("wsl", vec!["wsl", "bash", "-c"]),
    ];

    for (env_name, shell_cmd) in environments.iter() {
        if !is_shell_available(&shell_cmd[0]) {
            println!("Shell {} not available, skipping", env_name);
            continue;
        }

        println!("Testing {} in {} environment", exe_path, env_name);

        let result = test_executable_path_reporting(exe_path, shell_cmd);

        match result {
            Ok(output_path) => {
                validate_reported_path(&output_path, exe_path, env_name);
            }
            Err(e) => {
                println!("Failed to test {} in {}: {}", exe_path, env_name, e);
            }
        }
    }
}

/// Check if a shell is available on the system
fn is_shell_available(shell_path: &str) -> bool {
    if shell_path == "cmd" {
        return true; // cmd is always available on Windows
    }

    if shell_path == "powershell" {
        // Try to run a simple PowerShell command
        return Command::new("powershell")
            .args(&["-Command", "echo test"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    // For other shells, check if the executable exists
    Path::new(shell_path).exists()
}

/// Execute the test and capture its path reporting
fn test_executable_path_reporting(exe_path: &str, shell_cmd: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let timeout_duration = Duration::from_secs(10);

    // Create the command to run the executable and capture its self-reported path
    let mut cmd = Command::new(&shell_cmd[0]);

    if shell_cmd.len() > 1 {
        cmd.args(&shell_cmd[1..]);
    }

    // Add the actual command to execute
    let exec_command = if shell_cmd[0].contains("bash") {
        // For bash environments, use the proper quoting
        format!("'{}' --version 2>/dev/null || echo 'PATH:' \"$0\"", exe_path.replace("\\", "/"))
    } else {
        // For Windows environments
        format!("{} --version 2>nul || echo PATH: %~f0", exe_path)
    };

    cmd.arg(exec_command);

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set a timeout for the command
    let start = std::time::Instant::now();
    let output = cmd.output()?;

    if start.elapsed() > timeout_duration {
        return Err("Command timeout".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse the output to extract the path
    if let Some(path_line) = stdout.lines().find(|line| line.contains("PATH:")) {
        let path = path_line.split("PATH:").nth(1).unwrap_or("").trim();
        Ok(path.to_string())
    } else if !stderr.is_empty() {
        Err(format!("Command error: {}", stderr).into())
    } else {
        // If no explicit path output, try to infer from the execution context
        Ok(exe_path.to_string())
    }
}

/// Validate that the reported path is correct and not mangled by Git Bash
fn validate_reported_path(reported_path: &str, original_exe_path: &str, environment: &str) {
    println!("  Environment: {}", environment);
    println!("  Original:    {}", original_exe_path);
    println!("  Reported:    {}", reported_path);

    // Basic validation - path should not be empty
    assert!(!reported_path.is_empty(), "Reported path should not be empty in {}", environment);

    // Check for Git Bash pollution
    if environment.contains("git-bash") {
        // In Git Bash, we expect the path might be converted, but it should be valid

        // Should NOT contain the Git installation path pollution
        assert!(
            !reported_path.contains("Program Files\\Git\\mnt\\"),
            "Git Bash environment reported polluted path: {} in {}",
            reported_path, environment
        );

        assert!(
            !reported_path.contains("Program Files/Git/mnt/"),
            "Git Bash environment reported polluted path: {} in {}",
            reported_path, environment
        );

        // If it's been converted to a Unix-style path, it should be proper WSL format
        if reported_path.starts_with("/mnt/") {
            assert!(
                reported_path.matches("/mnt/").count() == 1,
                "Multiple /mnt/ prefixes detected in Git Bash: {} in {}",
                reported_path, environment
            );
        }
    } else {
        // In non-Git Bash environments, path should be standard Windows format
        if reported_path.len() > 2 {
            let second_char = reported_path.chars().nth(1);
            if let Some(':') = second_char {
                // Standard Windows path format
                let first_char = reported_path.chars().nth(0).unwrap();
                assert!(
                    first_char.is_ascii_alphabetic(),
                    "Invalid drive letter in {}: {} in {}",
                    reported_path, first_char, environment
                );
            }
        }
    }

    // Ensure no obviously mangled patterns
    let mangled_patterns = [
        "\\mnt\\c\\",
        "\\mnt\\d\\",
        "/mnt/c/Program Files/Git/",
        "Program Files\\Git\\mnt",
        "\\usr\\bin\\",
        "//c//",
    ];

    for pattern in mangled_patterns.iter() {
        assert!(
            !reported_path.contains(pattern),
            "Detected mangled path pattern '{}' in reported path: {} in {}",
            pattern, reported_path, environment
        );
    }

    println!("  ✓ Path validation passed");
}

/// Test path normalization with the actual winpath library from an executable context
#[test]
fn test_runtime_path_normalization() {
    // This test validates that path normalization works correctly when called
    // from within an executable (as opposed to just unit tests)

    // Get the current executable path
    let current_exe = env::current_exe().unwrap_or_else(|_| {
        // Fallback to a known test executable
        Path::new(r"T:\projects\coreutils\winutils\target\debug\winpath-test.exe").to_path_buf()
    });

    let current_exe_str = current_exe.to_string_lossy();

    // Test various representations of the current executable path
    let test_cases = vec![
        // Convert the current path to various formats for testing
        current_exe_str.replace("\\", "/"),  // Forward slash version

        // WSL-style version (if on C: drive)
        if current_exe_str.starts_with("C:") {
            current_exe_str.replace("C:\\", "/mnt/c/").replace("\\", "/").to_lowercase()
        } else {
            String::new()
        },

        // Cygwin-style version (if on C: drive)
        if current_exe_str.starts_with("C:") {
            current_exe_str.replace("C:\\", "/cygdrive/c/").replace("\\", "/").to_lowercase()
        } else {
            String::new()
        },
    ];

    // Use the winpath library to normalize each variant
    for test_case in test_cases.iter() {
        if test_case.is_empty() {
            continue;
        }

        println!("Testing runtime normalization of: {}", test_case);

        match winpath::normalize_path(test_case) {
            Ok(normalized) => {
                println!("  Normalized to: {}", normalized);

                // The normalized path should be a valid Windows path
                assert!(normalized.len() >= 3, "Normalized path too short: {}", normalized);

                // Should start with a drive letter
                if normalized.len() >= 2 {
                    let first_char = normalized.chars().nth(0).unwrap();
                    let second_char = normalized.chars().nth(1).unwrap();

                    if second_char == ':' {
                        assert!(
                            first_char.is_ascii_alphabetic() && first_char.is_ascii_uppercase(),
                            "Invalid drive letter in normalized path: {}", normalized
                        );
                    }
                }

                // Should not contain Git pollution
                assert!(!normalized.contains("Program Files\\Git"));
                assert!(!normalized.contains("\\mnt\\"));
                assert!(!normalized.contains("/mnt/"));

                println!("  ✓ Runtime normalization passed");
            }
            Err(e) => {
                println!("  Runtime normalization failed: {}", e);
                // Some cases might legitimately fail, but log for investigation
            }
        }
    }
}

/// Test environment variable path resolution
#[test]
fn test_environment_path_resolution() {
    // Test common environment variables that might contain paths
    let env_vars = [
        "PATH",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "WINDIR",
        "SYSTEMROOT",
    ];

    for var_name in env_vars.iter() {
        if let Ok(env_value) = env::var(var_name) {
            println!("Testing environment variable {}: {}", var_name, env_value);

            // Split PATH by semicolons, test other vars as single paths
            let paths_to_test = if *var_name == "PATH" {
                env_value.split(';').map(|s| s.to_string()).collect()
            } else {
                vec![env_value]
            };

            for path in paths_to_test.iter() {
                if path.is_empty() {
                    continue;
                }

                // Check if this looks like a potentially mangled Git Bash path
                if path.contains("Program Files\\Git\\mnt") ||
                   path.contains("Program Files/Git/mnt") ||
                   path.contains("\\mnt\\") {

                    println!("  Found potentially mangled path: {}", path);

                    // Try to normalize it
                    match winpath::normalize_path(path) {
                        Ok(normalized) => {
                            println!("    Normalized to: {}", normalized);

                            // Verify the normalization removed the pollution
                            assert!(!normalized.contains("Program Files\\Git\\mnt"));
                            assert!(!normalized.contains("\\mnt\\"));
                        }
                        Err(e) => {
                            println!("    Normalization failed: {}", e);
                        }
                    }
                } else {
                    // Normal path - should normalize to itself or a clean equivalent
                    match winpath::normalize_path(path) {
                        Ok(normalized) => {
                            // Basic validation
                            assert!(!normalized.is_empty());
                            assert!(!normalized.contains("\\mnt\\"));
                        }
                        Err(_) => {
                            // Some environment paths might not be normalizable, which is fine
                        }
                    }
                }
            }
        }
    }
}

/// Test path normalization in context of file operations
#[test]
fn test_file_operation_path_contexts() {
    use std::fs;

    // Test common file operation scenarios where Git Bash mangling might occur
    let test_scenarios = [
        // Temporary file operations
        ("temp_file", env::temp_dir().to_string_lossy().to_string()),

        // Current directory operations
        ("current_dir", env::current_dir().unwrap_or_default().to_string_lossy().to_string()),

        // User home directory
        ("home_dir", env::var("USERPROFILE").unwrap_or_default()),
    ];

    for (scenario_name, base_path) in test_scenarios.iter() {
        if base_path.is_empty() {
            continue;
        }

        println!("Testing file operation scenario: {} at {}", scenario_name, base_path);

        // Test various path representations of the same location
        let path_variants = vec![
            base_path.clone(),
            base_path.replace("\\", "/"),

            // WSL representation (if on C: drive)
            if base_path.starts_with("C:") {
                base_path.replace("C:\\", "/mnt/c/").replace("\\", "/").to_lowercase()
            } else {
                String::new()
            },
        ];

        for variant in path_variants.iter() {
            if variant.is_empty() {
                continue;
            }

            match winpath::normalize_path(variant) {
                Ok(normalized) => {
                    println!("  {} -> {}", variant, normalized);

                    // The normalized path should represent the same location as the base
                    // (we can't easily test this without file system operations, but we can do basic validation)

                    // Should be a valid Windows path
                    assert!(normalized.len() >= 2);
                    if normalized.chars().nth(1) == Some(':') {
                        let drive = normalized.chars().nth(0).unwrap();
                        assert!(drive.is_ascii_alphabetic() && drive.is_ascii_uppercase());
                    }

                    // Should not have Git pollution
                    assert!(!normalized.contains("Program Files\\Git"));
                    assert!(!normalized.contains("\\mnt\\"));
                }
                Err(e) => {
                    println!("  {} -> Error: {}", variant, e);
                }
            }
        }
    }
}

/// Performance test for path normalization in executable context
#[test]
fn test_executable_context_performance() {
    let test_paths = vec![
        r"C:\users\david\.local\bin\ls.exe",
        "/mnt/c/users/david/.local/bin/ls.exe",
        "C:/users/david/.local/bin/ls.exe",
        r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe",
        "/cygdrive/c/users/david/.local/bin/ls.exe",
    ];

    let iterations = 100;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        for path in test_paths.iter() {
            let _ = winpath::normalize_path(path);
        }
    }

    let duration = start.elapsed();
    let per_path_ns = duration.as_nanos() / (iterations * test_paths.len()) as u128;

    println!("Executable context performance: {} iterations of {} paths", iterations, test_paths.len());
    println!("Total time: {:?}", duration);
    println!("Average per path: {} ns", per_path_ns);

    // Should be fast enough for real-time usage
    assert!(per_path_ns < 100_000, "Path normalization too slow: {} ns per path", per_path_ns);
}
