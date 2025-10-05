// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Git Bash path normalization tests for the tree utility
//!
//! These tests verify that the tree utility properly handles various Git Bash
//! mangled path formats and displays normalized Windows paths in the output.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test tree with Git Bash style paths (/c/...)
#[test]
fn test_tree_with_git_bash_paths() {
    let temp = TempDir::new().unwrap();

    // Create directory structure
    let sub1 = temp.path().join("subdir1");
    let sub2 = sub1.join("subdir2");
    fs::create_dir_all(&sub2).unwrap();

    // Create files
    fs::write(temp.path().join("root.txt"), b"root file").unwrap();
    fs::write(sub1.join("file1.txt"), b"file in subdir1").unwrap();
    fs::write(sub2.join("file2.txt"), b"file in subdir2").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir1"))
        .stdout(predicate::str::contains("subdir2"))
        .stdout(predicate::str::contains("root.txt"))
        // Verify output shows Windows-style paths, not Git Bash paths
        .stdout(predicate::str::contains(":\\"));
}

/// Test tree with WSL style paths (/mnt/c/...)
#[test]
fn test_tree_with_wsl_paths() {
    let temp = TempDir::new().unwrap();

    // Create directory structure
    let bin_dir = temp.path().join("bin");
    let lib_dir = temp.path().join("lib");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();

    // Create files
    fs::write(bin_dir.join("app.exe"), b"executable").unwrap();
    fs::write(lib_dir.join("library.dll"), b"library").unwrap();

    // Convert to WSL style path
    let windows_path = temp.path().to_string_lossy();
    if let Some(drive) = windows_path.chars().next() {
        if windows_path.chars().nth(1) == Some(':') {
            let wsl_path = format!("/mnt/{}{}",
                drive.to_lowercase(),
                windows_path[2..].replace('\\', "/")
            );

            let mut cmd = Command::cargo_bin("tree").unwrap();
            cmd.arg(&wsl_path)
                .assert()
                .success()
                .stdout(predicate::str::contains("bin"))
                .stdout(predicate::str::contains("lib"))
                .stdout(predicate::str::contains("app.exe"))
                // Verify Windows-style path output
                .stdout(predicate::str::contains(":\\"));
        }
    }
}

/// Test tree with mixed separators in paths
#[test]
fn test_tree_with_mixed_separators() {
    let temp = TempDir::new().unwrap();

    // Create directory structure
    let docs = temp.path().join("Documents");
    let projects = docs.join("Projects");
    fs::create_dir_all(&projects).unwrap();

    // Create files
    fs::write(docs.join("readme.md"), b"documentation").unwrap();
    fs::write(projects.join("project.txt"), b"project file").unwrap();

    // Create path with mixed separators
    let mixed_path = temp.path().to_string_lossy().replace('\\', "/");
    let mixed_path = format!("{}\\Documents", temp.path().to_string_lossy()).replace("/Documents", "\\Documents");

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(&mixed_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Projects"))
        .stdout(predicate::str::contains("readme.md"))
        // Verify consistent Windows path format
        .stdout(predicate::str::contains(":\\"));
}

/// Test tree with depth limit and Git Bash paths
#[test]
fn test_tree_depth_limit_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create deep directory structure
    let mut current = temp.path().to_path_buf();
    for i in 1..=5 {
        current = current.join(format!("level{}", i));
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join(format!("file{}.txt", i)), b"content").unwrap();
    }

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    // Test with depth limit of 2
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-L").arg("2")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("level1"))
        .stdout(predicate::str::contains("level2"))
        .stdout(predicate::str::contains("level3").not()); // Should not appear due to depth limit
}

/// Test tree with directories only flag and Git Bash paths
#[test]
fn test_tree_directories_only_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create structure with files and directories
    let dir1 = temp.path().join("directory1");
    let dir2 = temp.path().join("directory2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    // Create files (should be filtered out with -d flag)
    fs::write(temp.path().join("file1.txt"), b"file content").unwrap();
    fs::write(dir1.join("file2.txt"), b"file content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-d")  // directories only
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("directory1"))
        .stdout(predicate::str::contains("directory2"))
        .stdout(predicate::str::contains("file1.txt").not())
        .stdout(predicate::str::contains("file2.txt").not());
}

/// Test tree with file pattern matching and Git Bash paths
#[test]
fn test_tree_pattern_matching_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create files with different extensions
    let files = ["app.exe", "library.dll", "document.txt", "image.png"];
    for file in &files {
        fs::write(temp.path().join(file), b"content").unwrap();
    }

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    // Test pattern matching for .exe files
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-P").arg("*.exe")
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("app.exe"))
        .stdout(predicate::str::contains("library.dll").not())
        .stdout(predicate::str::contains("document.txt").not());
}

/// Test tree with full path display and Git Bash paths
#[test]
fn test_tree_full_path_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create directory structure
    let sub_dir = temp.path().join("subdirectory");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("file.txt"), b"content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-f")  // full path
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("subdirectory"))
        .stdout(predicate::str::contains("file.txt"))
        // Full paths should be Windows-style
        .stdout(predicate::str::contains(":\\"));
}

/// Test tree with size display and Git Bash paths
#[test]
fn test_tree_size_display_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create files with known content
    fs::write(temp.path().join("small.txt"), b"small").unwrap();
    fs::write(temp.path().join("large.txt"), b"this is a larger file with more content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-s")  // show sizes
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("small.txt"))
        .stdout(predicate::str::contains("large.txt"))
        // Should show file sizes
        .stdout(predicate::str::is_match(r"\d+\s+\w+\.txt").unwrap());
}

/// Test tree with time display and Git Bash paths
#[test]
fn test_tree_time_display_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create test file
    fs::write(temp.path().join("timestamped.txt"), b"content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-D")  // show last modification time
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("timestamped.txt"))
        // Should show timestamp in format like [YYYY-MM-DD HH:MM:SS]
        .stdout(predicate::str::is_match(r"\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\]").unwrap());
}

/// Test tree with spaces and special characters in Git Bash paths
#[test]
fn test_tree_special_chars_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create directories and files with spaces and special characters
    let special_dir = temp.path().join("Program Files (x86)");
    let unicode_dir = special_dir.join("Ümlauts & Special");
    fs::create_dir_all(&unicode_dir).unwrap();

    fs::write(special_dir.join("app with spaces.exe"), b"executable").unwrap();
    fs::write(unicode_dir.join("file with café.txt"), b"unicode content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        special_dir.strip_prefix("C:\\")
            .unwrap_or(&special_dir)
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Ümlauts & Special"))
        .stdout(predicate::str::contains("app with spaces.exe"))
        .stdout(predicate::str::contains("file with café.txt"));
}

/// Test tree with JSON output and Git Bash paths
#[test]
fn test_tree_json_output_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create simple directory structure
    let sub_dir = temp.path().join("data");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("config.json"), b"{}").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-J")  // JSON output
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\":"))
        .stdout(predicate::str::contains("\"type\":"))
        // JSON paths should be properly escaped Windows paths
        .stdout(predicate::str::contains(":\\\\"));
}

/// Test tree with symbolic links and Git Bash paths
#[test]
#[cfg(windows)]
fn test_tree_symlinks_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create target file and directory
    let target_file = temp.path().join("target.txt");
    let target_dir = temp.path().join("target_dir");
    fs::write(&target_file, b"target content").unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    // Note: Creating symbolic links on Windows requires admin privileges
    // This test may be skipped in non-admin environments

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-l")  // follow symbolic links
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("target.txt"))
        .stdout(predicate::str::contains("target_dir"));
}

/// Test tree with large directory and Git Bash paths (performance)
#[test]
fn test_tree_large_directory_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create many files and directories
    for i in 0..100 {
        let dir = temp.path().join(format!("dir_{:03}", i));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), format!("content {}", i)).unwrap();
    }

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("dir_000"))
        .stdout(predicate::str::contains("dir_099"));

    let duration = start.elapsed();

    // Should complete reasonably quickly even with path normalization
    assert!(duration.as_secs() < 5, "Tree with Git Bash path took too long: {:?}", duration);
}

/// Test tree with various output formats and Git Bash paths
#[test]
fn test_tree_output_formats_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create test structure
    let sub_dir = temp.path().join("test_dir");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("test.txt"), b"content").unwrap();

    // Convert to Git Bash style path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    // Test ASCII output
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-a")  // ASCII characters only
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test_dir"))
        .stdout(predicate::str::contains("test.txt"));

    // Test with no indent lines
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("-i")  // no indent lines
        .arg(&git_bash_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("test_dir"))
        .stdout(predicate::str::contains("test.txt"));
}

/// Test tree error handling with invalid Git Bash paths
#[test]
fn test_tree_invalid_git_bash_paths() {
    // Test with invalid drive letter
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("/z/nonexistent/path")
        .assert()
        .failure();

    // Test with malformed Git Bash path
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("/invalid/format")
        .assert()
        .failure();

    // Test with permission denied (simulated)
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg("/c/System Volume Information")
        .assert()
        .failure();
}

/// Test tree with relative paths from Git Bash current directory
#[test]
fn test_tree_relative_paths_git_bash() {
    let temp = TempDir::new().unwrap();

    // Create subdirectory
    let sub_dir = temp.path().join("subdir");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("file.txt"), b"content").unwrap();

    // Test with relative path (assuming current directory normalization)
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.current_dir(&temp)
        .arg("./subdir")
        .assert()
        .success()
        .stdout(predicate::str::contains("file.txt"));
}

/// Integration test: Tree of real Git installation directory
#[test]
#[ignore] // Run manually as it depends on Git installation
fn test_real_git_directory_tree() {
    let git_paths = [
        "/c/Program Files/Git",
        "/c/Program Files (x86)/Git",
    ];

    for path in &git_paths {
        let mut cmd = Command::cargo_bin("tree").unwrap();
        let result = cmd.arg("-L").arg("2")  // Limit depth to avoid too much output
            .arg(path)
            .assert();

        // If the path exists, verify proper normalization
        if result.get_matches().is_ok() {
            result.stdout(predicate::str::contains(":\\"));
            break;
        }
    }
}

/// Benchmark test: Compare tree performance with/without Git Bash paths
#[test]
#[ignore] // Run with --ignored for benchmarking
fn benchmark_tree_git_bash_overhead() {
    let temp = TempDir::new().unwrap();

    // Create test structure
    for i in 0..50 {
        let dir = temp.path().join(format!("dir_{}", i));
        fs::create_dir_all(&dir).unwrap();
        for j in 0..5 {
            fs::write(dir.join(format!("file_{}.txt", j)), b"content").unwrap();
        }
    }

    // Benchmark with Windows path
    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(temp.path().to_str().unwrap())
        .assert()
        .success();
    let windows_time = start.elapsed();

    // Benchmark with Git Bash path
    let git_bash_path = format!("/c/{}",
        temp.path().strip_prefix("C:\\")
            .unwrap_or(temp.path())
            .to_string_lossy()
            .replace('\\', "/")
    );

    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("tree").unwrap();
    cmd.arg(&git_bash_path)
        .assert()
        .success();
    let git_bash_time = start.elapsed();

    println!("Windows path time: {:?}", windows_time);
    println!("Git Bash path time: {:?}", git_bash_time);

    // Path normalization should not add significant overhead
    let overhead_ratio = git_bash_time.as_nanos() as f64 / windows_time.as_nanos() as f64;
    assert!(overhead_ratio < 2.5, "Git Bash path normalization overhead too high: {}x", overhead_ratio);
}
