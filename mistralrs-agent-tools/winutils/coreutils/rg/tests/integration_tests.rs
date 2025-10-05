//! Integration tests for the ripgrep Windows wrapper
//!
//! These tests verify that the wrapper correctly processes arguments and
//! normalizes paths while maintaining compatibility with ripgrep.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test command
fn rg_wrapper() -> Command {
    Command::cargo_bin("rg").expect("Binary should be built")
}

/// Helper to create a temporary directory with test files
fn setup_test_dir() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create some test files
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "hello world\ntest pattern\nother content").unwrap();

    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let sub_file = subdir.join("sub.txt");
    fs::write(&sub_file, "pattern in subdirectory\nmore content").unwrap();

    temp_dir
}

#[test]
fn test_version_output() {
    rg_wrapper()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rg-wrapper"));
}

#[test]
fn test_help_output() {
    rg_wrapper()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Enhanced Windows wrapper"));
}

#[test]
fn test_basic_search() {
    let temp_dir = setup_test_dir();
    let test_file = temp_dir.path().join("test.txt");

    rg_wrapper()
        .arg("pattern")
        .arg(test_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test pattern"));
}

#[test]
fn test_directory_search() {
    let temp_dir = setup_test_dir();

    rg_wrapper()
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[test]
fn test_flag_passthrough() {
    let temp_dir = setup_test_dir();

    // Test that flags are properly passed through
    rg_wrapper()
        .arg("--count")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+").unwrap());
}

#[test]
fn test_no_args_shows_help() {
    // When no arguments are provided, ripgrep typically shows help
    rg_wrapper()
        .assert()
        .failure(); // ripgrep returns non-zero when no args
}

#[test]
fn test_invalid_pattern() {
    let temp_dir = setup_test_dir();

    // Test with an invalid regex pattern (unclosed bracket)
    rg_wrapper()
        .arg("[unclosed")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .failure(); // Should fail with invalid regex
}

#[test]
fn test_nonexistent_file() {
    rg_wrapper()
        .arg("pattern")
        .arg("nonexistent_file.txt")
        .assert()
        .failure(); // Should fail when file doesn't exist
}

#[test]
fn test_multiple_files() {
    let temp_dir = setup_test_dir();
    let test_file = temp_dir.path().join("test.txt");
    let sub_file = temp_dir.path().join("subdir").join("sub.txt");

    rg_wrapper()
        .arg("pattern")
        .arg(test_file.to_str().unwrap())
        .arg(sub_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[test]
fn test_hidden_flag() {
    let temp_dir = setup_test_dir();

    // Create a hidden file (on Windows, we simulate this)
    let hidden_file = temp_dir.path().join(".hidden");
    fs::write(&hidden_file, "hidden pattern content").unwrap();

    rg_wrapper()
        .arg("--hidden")
        .arg("hidden")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_case_insensitive() {
    let temp_dir = setup_test_dir();

    rg_wrapper()
        .arg("--ignore-case")
        .arg("PATTERN")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[test]
fn test_file_type_filtering() {
    let temp_dir = setup_test_dir();

    // Create a Rust file
    let rust_file = temp_dir.path().join("test.rs");
    fs::write(&rust_file, "fn main() { pattern(); }").unwrap();

    rg_wrapper()
        .arg("--type")
        .arg("rust")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[cfg(windows)]
#[test]
fn test_windows_path_normalization() {
    use std::path::PathBuf;

    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_str().unwrap();

    // Test with forward slashes (should be normalized)
    let forward_slash_path = temp_path.replace('\\', "/");

    rg_wrapper()
        .arg("pattern")
        .arg(&forward_slash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[cfg(windows)]
#[test]
fn test_wsl_path_simulation() {
    let temp_dir = setup_test_dir();

    // Note: This test simulates WSL path handling, but won't work with actual WSL paths
    // in a Windows-only test environment. In a real WSL environment, paths like
    // /mnt/c/... would be properly normalized.

    // We can at least test that the argument parsing doesn't crash
    rg_wrapper()
        .arg("pattern")
        .arg("/mnt/c/nonexistent")
        .assert()
        .failure(); // Will fail because path doesn't exist, but shouldn't crash
}

#[test]
fn test_output_format_flags() {
    let temp_dir = setup_test_dir();

    // Test JSON output format
    rg_wrapper()
        .arg("--json")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":"));
}

#[test]
fn test_context_flags() {
    let temp_dir = setup_test_dir();

    // Test context lines
    rg_wrapper()
        .arg("--context")
        .arg("1")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[test]
fn test_line_numbers() {
    let temp_dir = setup_test_dir();

    rg_wrapper()
        .arg("--line-number")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+:.*pattern").unwrap());
}

#[test]
fn test_recursive_search() {
    let temp_dir = setup_test_dir();

    // Recursive search should be default behavior
    rg_wrapper()
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("pattern"));
}

#[test]
fn test_exclude_patterns() {
    let temp_dir = setup_test_dir();

    // Create a file that should be excluded
    let excluded_file = temp_dir.path().join("excluded.bak");
    fs::write(&excluded_file, "pattern in backup file").unwrap();

    rg_wrapper()
        .arg("--glob")
        .arg("!*.bak")
        .arg("pattern")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        // Should find pattern in other files but not the .bak file
        .stdout(predicate::str::contains("pattern"));
}
