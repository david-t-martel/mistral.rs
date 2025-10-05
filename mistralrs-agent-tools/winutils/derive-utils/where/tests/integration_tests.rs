// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the where utility

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test basic executable search functionality
#[test]
fn test_basic_search() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"));
}

/// Test wildcard pattern matching
#[test]
fn test_wildcard_patterns() {
    let temp = TempDir::new().unwrap();

    // Create test files
    let files = ["test1.exe", "test2.exe", "program.bat", "script.cmd"];
    for file in &files {
        let file_path = temp.path().join(file);
        fs::write(&file_path, b"").unwrap();
    }

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test*.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test1.exe"))
        .stdout(predicate::str::contains("test2.exe"))
        .stdout(predicate::str::contains("program.bat").not());
}

/// Test quiet mode
#[test]
fn test_quiet_mode() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .arg("-Q")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// Test full path output
#[test]
fn test_full_path_output() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .arg("-F")
        .assert()
        .success()
        .stdout(predicate::str::contains(temp.path().to_str().unwrap()));
}

/// Test time and size display
#[test]
fn test_time_display() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("test.exe");
    fs::write(&exe_path, b"test content").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .arg("-T")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.exe"))
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap());
}

/// Test recursive search
#[test]
fn test_recursive_search() {
    let temp = TempDir::new().unwrap();

    // Create nested directory structure
    let subdir1 = temp.path().join("subdir1");
    let subdir2 = subdir1.join("subdir2");
    fs::create_dir_all(&subdir2).unwrap();

    // Create executables in different levels
    let exe1 = temp.path().join("root.exe");
    let exe2 = subdir1.join("level1.exe");
    let exe3 = subdir2.join("level2.exe");

    fs::write(&exe1, b"").unwrap();
    fs::write(&exe2, b"").unwrap();
    fs::write(&exe3, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("*.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("root.exe"))
        .stdout(predicate::str::contains("level1.exe"))
        .stdout(predicate::str::contains("level2.exe"));
}

/// Test file not found
#[test]
fn test_file_not_found() {
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("nonexistent.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .failure()
        .code(1);
}

/// Test multiple patterns
#[test]
fn test_multiple_patterns() {
    let temp = TempDir::new().unwrap();

    let files = ["python.exe", "node.exe", "script.bat"];
    for file in &files {
        let file_path = temp.path().join(file);
        fs::write(&file_path, b"").unwrap();
    }

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("python.exe")
        .arg("node.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("python.exe"))
        .stdout(predicate::str::contains("node.exe"))
        .stdout(predicate::str::contains("script.bat").not());
}

/// Test case insensitive matching
#[test]
fn test_case_insensitive() {
    let temp = TempDir::new().unwrap();
    let exe_path = temp.path().join("Test.EXE");
    fs::write(&exe_path, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test.EXE"));
}

/// Test PATHEXT expansion
#[test]
fn test_pathext_expansion() {
    let temp = TempDir::new().unwrap();

    // Create executable without extension
    let exe_path = temp.path().join("python.exe");
    fs::write(&exe_path, b"").unwrap();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("python")  // Search without extension
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("python.exe"));
}

/// Test performance with large directory
#[test]
fn test_large_directory_performance() {
    let temp = TempDir::new().unwrap();

    // Create many files
    for i in 0..1000 {
        let file_path = temp.path().join(format!("file{}.txt", i));
        fs::write(&file_path, b"").unwrap();
    }

    // Create target file
    let target_path = temp.path().join("target.exe");
    fs::write(&target_path, b"").unwrap();

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("target.exe")
        .arg("-R")
        .arg(temp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("target.exe"));

    let duration = start.elapsed();

    // Should complete within a reasonable time (adjust as needed)
    assert!(duration.as_secs() < 5, "Search took too long: {:?}", duration);
}

/// Test invalid directory
#[test]
fn test_invalid_directory() {
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.arg("test.exe")
        .arg("-R")
        .arg("/nonexistent/directory")
        .assert()
        .failure();
}

/// Test empty pattern
#[test]
fn test_empty_pattern() {
    let mut cmd = Command::cargo_bin("where").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Benchmark test for comparison with native where.exe
#[test]
#[ignore] // Run with --ignored flag for benchmarking
fn benchmark_vs_native_where() {
    use std::process::Command as StdCommand;
    use std::time::Instant;

    // Test with a common executable
    let pattern = "cmd.exe";

    // Benchmark our implementation
    let start = Instant::now();
    let mut our_cmd = Command::cargo_bin("where").unwrap();
    our_cmd.arg(pattern).arg("-Q").assert().success();
    let our_time = start.elapsed();

    // Benchmark native where.exe
    let start = Instant::now();
    let native_result = StdCommand::new("where")
        .arg("/Q")
        .arg(pattern)
        .output();
    let native_time = start.elapsed();

    if let Ok(_) = native_result {
        println!("Our implementation: {:?}", our_time);
        println!("Native where.exe: {:?}", native_time);

        // Our implementation should be competitive
        // This is just informational, not a hard requirement
        if our_time < native_time {
            println!("✓ Our implementation is faster!");
        } else {
            println!("ℹ Native implementation is faster by {:?}", our_time - native_time);
        }
    }
}
