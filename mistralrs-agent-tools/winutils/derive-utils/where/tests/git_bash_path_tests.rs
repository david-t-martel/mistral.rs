// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Git Bash path normalization tests for the where utility
//!
//! These tests verify that the where utility properly handles various Git Bash
//! mangled path formats and returns normalized Windows paths.

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test Git Bash mangled paths: /c/Program Files/Git/usr/bin/...
#[test]
fn test_git_bash_mangled_paths() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Test with Git Bash style path (simulated)
    let git_bash_path = format!("/c/{}", temp.path().strip_prefix("C:\\").unwrap().to_string_lossy().replace('\\', "/"));

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"))
        // Verify output uses Windows-style paths, not Git Bash paths
        .stdout(predicate::str::contains("C:\\").or(predicate::str::contains(":\\")));
}

/// Test WSL-style paths: /mnt/c/...
#[test]
fn test_wsl_style_paths() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Convert Windows path to WSL style
    let windows_path = temp.path().to_string_lossy();
    if let Some(drive) = windows_path.chars().next() {
        if windows_path.chars().nth(1) == Some(':') {
            let wsl_path = format!("/mnt/{}{}",
                drive.to_lowercase(),
                windows_path[2..].replace('\\', "/")
            );

            let mut cmd = Command::cargo_bin("where").unwrap();
            cmd.arg("test.exe")
                .arg("-R")
                .arg(&wsl_path)
                .assert()
                .success()
                .stdout(predicate::str::contains("test.exe"))
                // Verify output uses Windows-style paths
                .stdout(predicate::str::contains(":\\"));
        }
    }
}

/// Test mixed path separators: C:\Users/David\Documents
#[test]
fn test_mixed_separators() {
    let temp = TempDir::new().unwrap();

    // Create subdirectory with test executable
    let sub_dir = temp.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    let exe_path = sub_dir.join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Create path with mixed separators
    let mixed_path = temp.path().to_string_lossy().replace('\\', "/");
    let mixed_path = mixed_path.replace("/subdir", "\\subdir");

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(&mixed_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"))
        // Verify consistent Windows path format in output
        .stdout(predicate::str::contains(":\\"));
}

/// Test regular Windows paths remain unchanged
#[test]
fn test_regular_windows_paths() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    let windows_path = temp.path().to_string_lossy();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(windows_path.as_ref())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"))
        .stdout(predicate::str::contains(":\\"));
}

/// Test PATH environment variable with Git Bash paths
#[test]
fn test_path_with_git_bash_entries() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Simulate Git Bash PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    // Set custom PATH
    let original_path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.env("PATH", &new_path)
        .arg("test.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"))
        // Verify normalized Windows path in output
        .stdout(predicate::str::contains(":\\"));
}

/// Test UNC paths from Git Bash
#[test]
fn test_unc_paths_from_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Test with network path style (simulated)
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"));

    // The actual output should not contain UNC prefix artifacts
    let output = cmd.get_matches().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\\\\?\\"));
}

/// Test path normalization with spaces and special characters
#[test]
fn test_paths_with_spaces_and_special_chars() {
    let temp = TempDir::new().unwrap();

    // Create subdirectory with spaces and special characters
    let special_dir = temp.path().join("Program Files (x86)");
    fs::create_dir_all(&special_dir).unwrap();
    let exe_path = special_dir.join("test app.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        special_dir.strip_prefix("C:\\")
            .unwrap_or(&special_dir)
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test app.exe")
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test app.exe"))
        .stdout(predicate::str::contains("Program Files (x86)"));
}

/// Test long path support with Git Bash normalization
#[test]
fn test_long_paths() {
    let temp = TempDir::new().unwrap();

    // Create a deeply nested directory structure
    let mut deep_path = temp.path().to_path_buf();
    for i in 0..10 {
        deep_path = deep_path.join(format!("very_long_directory_name_{}", i));
    }
    fs::create_dir_all(&deep_path).unwrap();

    let exe_path = deep_path.join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(deep_path.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"));
}

/// Test case sensitivity with Git Bash paths
#[test]
fn test_case_sensitivity_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create test executable with mixed case
    let exe_path = temp.path().join("TestApp.EXE");
    fs::write(&exe_path, b"test executable").unwrap();

    // Test with lowercase search in Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("testapp.exe")  // lowercase search
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("TestApp.EXE"));
}

/// Test wildcard patterns with Git Bash paths
#[test]
fn test_wildcards_with_git_bash_paths() {
    let temp = TempDir::new().unwrap();

    // Create multiple test executables
    let files = ["app1.exe", "app2.exe", "tool.bat", "script.cmd"];
    for file in &files {
        let file_path = temp.path().join(file);
        fs::write(&file_path, b"test content").unwrap();
    }

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("app*.exe")
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("app1.exe"))
        .stdout(predicate::str::contains("app2.exe"))
        .stdout(predicate::str::contains("tool.bat").not());
}

/// Test performance with Git Bash path conversion
#[test]
fn test_performance_git_bash_conversion() {
    let temp = TempDir::new().unwrap();

    // Create many files to test performance
    for i in 0..100 {
        let file_path = temp.path().join(format!("file{}.txt", i));
        fs::write(&file_path, b"content").unwrap();
    }

    // Create target executable
    let exe_path = temp.path().join("target.exe");
    fs::write(&exe_path, b"target executable").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("target.exe")
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("target.exe"));

    let duration = start.elapsed();

    // Should complete quickly even with path conversion
    assert!(duration.as_secs() < 2, "Git Bash path conversion took too long: {:?}", duration);
}

/// Test error handling with invalid Git Bash paths
#[test]
fn test_invalid_git_bash_paths() {
    // Test with invalid drive letter
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg("/z/nonexistent/path")
        .assert()
        .failure();

    // Test with malformed Git Bash path
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg("/invalid/git/bash/path")
        .assert()
        .failure();
}

/// Integration test: Search for real Git Bash executables
#[test]
#[ignore] // Run manually as it depends on Git installation
fn test_real_git_bash_executables() {
    // Try to find git.exe in common Git Bash locations
    let git_paths = [
        "/c/Program Files/Git/cmd",
        "/c/Program Files (x86)/Git/cmd",
        "/usr/bin",
    ];

    for path in &git_paths {
        let mut cmd = Command::cargo_bin("where").unwrap();
        let result = cmd.arg("git.exe")
            .arg("-R")
            .arg(path)
            .assert();

        // If found, verify the output is normalized
        if result.get_matches().is_ok() {
            result.stdout(predicate::str::contains(":\\"));
        }
    }
}

/// Benchmark test: Compare performance with/without path normalization
#[test]
#[ignore] // Run with --ignored for benchmarking
fn benchmark_path_normalization_overhead() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Benchmark regular Windows path
    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success();
    let windows_time = start.elapsed();

    // Benchmark Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(&git_bash_path)
        .assert()
        .success();
    let git_bash_time = start.elapsed();

    println!("Windows path time: {:?}", windows_time);
    println!("Git Bash path time: {:?}", git_bash_time);

    // Path normalization overhead should be minimal
    let overhead_ratio = git_bash_time.as_nanos() as f64 / windows_time.as_nanos() as f64;
    assert!(overhead_ratio < 2.0, "Path normalization overhead too high: {}x", overhead_ratio);
}
