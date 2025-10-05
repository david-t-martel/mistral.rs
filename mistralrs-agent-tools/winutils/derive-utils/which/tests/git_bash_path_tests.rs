// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Git Bash path normalization tests for the which utility
//!
//! These tests verify that the which utility properly handles various Git Bash
//! mangled path formats and returns normalized Windows paths when locating executables.

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test which with Git Bash PATH entries containing /c/ style paths
#[test]
fn test_which_with_git_bash_path_entries() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("testcmd.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("testcmd.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("testcmd.exe"))
        // Verify output uses Windows-style paths, not Git Bash paths
        .stdout(predicate::str::contains(":\\"));
}

/// Test which with WSL-style PATH entries containing /mnt/c/ paths
#[test]
fn test_which_with_wsl_path_entries() {
    let temp = TempDir::new().unwrap();

    // Create a test executable
    let exe_path = temp.path().join("wsltest.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create WSL style PATH entry
    let windows_path = temp.path().to_string_lossy();
    if let Some(drive) = windows_path.chars().next() {
        if windows_path.chars().nth(1) == Some(':') {
            let wsl_path = format!("/mnt/{}{}",
                drive.to_lowercase(),
                windows_path[2..].replace('\\', "/")
            );

            let new_path = format!("{};{}", original_path, wsl_path);

            let mut cmd = Command::cargo_bin("which").unwrap();
            cmd.env("PATH", &new_path)
                .arg("wsltest.exe")
                .assert()
                .success()
                .stdout(predicate::str::contains("wsltest.exe"))
                // Verify Windows-style path output
                .stdout(predicate::str::contains(":\\"));
        }
    }
}

/// Test which with mixed separator paths in PATH
#[test]
fn test_which_with_mixed_separators_in_path() {
    let temp = TempDir::new().unwrap();

    // Create subdirectory with test executable
    let sub_dir = temp.path().join("bin");
    fs::create_dir(&sub_dir).unwrap();
    let exe_path = sub_dir.join("mixedpath.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create path with mixed separators
    let mixed_path = sub_dir.to_string_lossy().replace('\\', "/");
    let mixed_path = format!("{}\\bin", temp.path().to_string_lossy()).replace("/bin", "\\bin");

    let new_path = format!("{};{}", original_path, mixed_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("mixedpath.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("mixedpath.exe"))
        // Verify consistent Windows path format in output
        .stdout(predicate::str::contains(":\\"));
}

/// Test which finds all instances with --all flag and Git Bash paths
#[test]
fn test_which_all_with_git_bash_paths() {
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();

    // Create identical executables in two directories
    let exe1 = temp1.path().join("common.exe");
    let exe2 = temp2.path().join("common.exe");
    fs::write(&exe1, b"test executable 1").unwrap();
    fs::write(&exe2, b"test executable 2").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entries
    let git_bash_path1 = format!("/c/{}",
        temp1.path().strip_prefix("C:\\")
            .unwrap_or(temp1.path())
            .to_string_lossy()
            .replace('\\', "/")
    );
    let git_bash_path2 = format!("/c/{}",
        temp2.path().strip_prefix("C:\\")
            .unwrap_or(temp2.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{};{}", original_path, git_bash_path1, git_bash_path2);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("--all")
        .arg("common.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("common.exe"))
        // Should find multiple instances
        .stdout(predicate::str::is_match(r"(?s)common\.exe.*common\.exe").unwrap())
        // All outputs should use Windows paths
        .stdout(predicate::str::contains(":\\"));
}

/// Test which handles current directory lookup with Git Bash paths
#[test]
fn test_which_current_directory_with_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create executable in temp directory
    let exe_path = temp.path().join("current.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Change to temp directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp).unwrap();

    // Test finding executable in current directory
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.arg("current.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("current.exe"))
        // Should return Windows-style path
        .stdout(predicate::str::contains(":\\"));

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}

/// Test which with PATHEXT and Git Bash paths
#[test]
fn test_which_pathext_with_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create executable without extension
    let exe_path = temp.path().join("noext.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH and PATHEXT
    let original_path = env::var("PATH").unwrap_or_default();
    let original_pathext = env::var("PATHEXT").unwrap_or_default();

    // Create Git Bash style PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .env("PATHEXT", ".EXE;.BAT;.CMD")
        .arg("noext")  // Search without extension
        .assert()
        .success()
        .stdout(predicate::str::contains("noext.exe"))
        // Verify Windows path output
        .stdout(predicate::str::contains(":\\"));
}

/// Test which handles spaces and special characters in Git Bash paths
#[test]
fn test_which_special_chars_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create directory with spaces and special characters
    let special_dir = temp.path().join("Program Files (x86)");
    fs::create_dir_all(&special_dir).unwrap();
    let exe_path = special_dir.join("special app.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry with spaces
    let git_bash_path = format!("/c/{}",
        special_dir.strip_prefix("C:\\")
            .unwrap_or(&special_dir)
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("special app.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("special app.exe"))
        .stdout(predicate::str::contains("Program Files (x86)"));
}

/// Test which silent mode with Git Bash paths
#[test]
fn test_which_silent_mode_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create test executable
    let exe_path = temp.path().join("silent.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    // Test successful silent mode
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("--silent")
        .arg("silent.exe")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    // Test failed silent mode
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("--silent")
        .arg("nonexistent.exe")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());
}

/// Test which performance with many Git Bash PATH entries
#[test]
fn test_which_performance_many_git_bash_paths() {
    let temp = TempDir::new().unwrap();

    // Create target executable
    let exe_path = temp.path().join("target.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create many Git Bash style PATH entries
    let mut new_path = original_path.clone();
    for i in 0..10 {
        let temp_dir = temp.path().join(format!("dir{}", i));
        fs::create_dir(&temp_dir).unwrap();

        let git_bash_path = format!("/c/{}",
            temp_dir.strip_prefix("C:\\")
                .unwrap_or(&temp_dir)
                .to_string_lossy()
                .replace('\\', "/")
        );
        new_path.push_str(&format!(";{}", git_bash_path));
    }

    // Add target directory at the end
    let target_git_bash = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );
    new_path.push_str(&format!(";{}", target_git_bash));

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("target.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("target.exe"));

    let duration = start.elapsed();

    // Should complete quickly even with many Git Bash path conversions
    assert!(duration.as_secs() < 3, "Search with many Git Bash paths took too long: {:?}", duration);
}

/// Test which case sensitivity with Git Bash paths
#[test]
fn test_which_case_sensitivity_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create test executable with mixed case
    let exe_path = temp.path().join("CaseTest.EXE");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    // Test with lowercase search (should work on Windows)
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("casetest.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("CaseTest.EXE"));
}

/// Test which with stdin input and Git Bash paths
#[test]
fn test_which_stdin_with_git_bash_paths() {
    let temp = TempDir::new().unwrap();

    // Create test executables
    let exe1 = temp.path().join("stdin1.exe");
    let exe2 = temp.path().join("stdin2.exe");
    fs::write(&exe1, b"test executable 1").unwrap();
    fs::write(&exe2, b"test executable 2").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("--read-alias")
        .write_stdin("stdin1.exe\nstdin2.exe\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin1.exe"))
        .stdout(predicate::str::contains("stdin2.exe"))
        // All outputs should use Windows paths
        .stdout(predicate::str::contains(":\\"));
}

/// Test which with non-existent command and Git Bash paths
#[test]
fn test_which_nonexistent_git_bash() {
    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create fake Git Bash style PATH entries
    let fake_paths = [
        "/c/fake/path1",
        "/c/fake/path2",
        "/mnt/c/fake/path3",
    ];

    let mut new_path = original_path;
    for path in &fake_paths {
        new_path.push_str(&format!(";{}", path));
    }

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("definitely_does_not_exist_12345")
        .assert()
        .failure();
}

/// Test which long path support with Git Bash
#[test]
fn test_which_long_paths_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create deeply nested directory
    let mut deep_path = temp.path().to_path_buf();
    for i in 0..8 {
        deep_path = deep_path.join(format!("very_long_directory_name_{}", i));
    }
    fs::create_dir_all(&deep_path).unwrap();

    let exe_path = deep_path.join("longpath.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Create Git Bash style PATH entry for deep path
    let git_bash_path = format!("/c/{}",
        deep_path.strip_prefix("C:\\")
            .unwrap_or(&deep_path)
            .to_string_lossy()
            .replace('\\', "/")
    );

    let new_path = format!("{};{}", original_path, git_bash_path);

    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &new_path)
        .arg("longpath.exe")
        .assert()
        .success()
        .stdout(predicate::str::contains("longpath.exe"));
}

/// Integration test: Search for real Git executables
#[test]
#[ignore] // Run manually as it depends on Git installation
fn test_real_git_executables() {
    // Common Git installation paths in Git Bash format
    let git_paths = [
        "/c/Program Files/Git/cmd",
        "/c/Program Files (x86)/Git/cmd",
        "/c/Program Files/Git/usr/bin",
        "/usr/bin",
    ];

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    for path in &git_paths {
        let new_path = format!("{};{}", original_path, path);

        let mut cmd = Command::cargo_bin("which").unwrap();
        let result = cmd.env("PATH", &new_path)
            .arg("git.exe")
            .assert();

        // If found, verify output is properly normalized
        if result.get_matches().is_ok() {
            result.stdout(predicate::str::contains(":\\"));
            break;
        }
    }
}

/// Benchmark test: Compare which performance with/without Git Bash paths
#[test]
#[ignore] // Run with --ignored for benchmarking
fn benchmark_git_bash_path_overhead() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("benchmark.exe");
    fs::write(&exe_path, b"test executable").unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Benchmark with normal Windows path
    let windows_path = format!("{};{}", original_path, temp.path().to_string_lossy());

    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &windows_path)
        .arg("benchmark.exe")
        .assert()
        .success();
    let windows_time = start.elapsed();

    // Benchmark with Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );
    let git_bash_full_path = format!("{};{}", original_path, git_bash_path);

    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("which").unwrap();
    cmd.env("PATH", &git_bash_full_path)
        .arg("benchmark.exe")
        .assert()
        .success();
    let git_bash_time = start.elapsed();

    println!("Windows path time: {:?}", windows_time);
    println!("Git Bash path time: {:?}", git_bash_time);

    // Git Bash path processing should not add significant overhead
    let overhead_ratio = git_bash_time.as_nanos() as f64 / windows_time.as_nanos() as f64;
    assert!(overhead_ratio < 3.0, "Git Bash path overhead too high: {}x", overhead_ratio);
}
