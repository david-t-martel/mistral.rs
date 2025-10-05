//! Integration tests for the Windows-optimized ls utility

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Test basic directory listing functionality
#[test]
fn test_basic_directory_listing() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create test files
    fs::write(temp_path.join("file1.txt"), "content1").unwrap();
    fs::write(temp_path.join("file2.log"), "content2").unwrap();
    fs::create_dir(temp_path.join("subdir")).unwrap();

    // Run ls command
    let output = Command::new("cargo")
        .args(&["run", "--", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that all files are listed
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.log"));
    assert!(stdout.contains("subdir"));
}

/// Test long format output
#[test]
fn test_long_format() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("test.txt"), "test content").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-l", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -l command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check long format components
    assert!(stdout.contains("test.txt"));
    // Should contain file size and date
    assert!(stdout.chars().filter(|&c| c.is_ascii_digit()).count() > 0);
}

/// Test JSON output format
#[test]
fn test_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("test.json"), "{}").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-j", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -j command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that output is valid JSON
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON: {}", stdout);

    let json = json_result.unwrap();
    assert!(json.get("directories").is_some());
}

/// Test hidden file handling
#[test]
fn test_hidden_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("visible.txt"), "visible").unwrap();
    fs::write(temp_path.join(".hidden.txt"), "hidden").unwrap();

    // Test without -a flag
    let output = Command::new("cargo")
        .args(&["run", "--", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("visible.txt"));
    assert!(!stdout.contains(".hidden.txt"));

    // Test with -a flag
    let output_all = Command::new("cargo")
        .args(&["run", "--", "-a", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -a command");

    let stdout_all = String::from_utf8_lossy(&output_all.stdout);
    assert!(stdout_all.contains("visible.txt"));
    assert!(stdout_all.contains(".hidden.txt"));
}

/// Test recursive listing
#[test]
fn test_recursive_listing() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create nested directory structure
    let subdir = temp_path.join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(temp_path.join("root.txt"), "root").unwrap();
    fs::write(subdir.join("nested.txt"), "nested").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-R", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -R command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root.txt"));
    assert!(stdout.contains("nested.txt"));
}

/// Test sorting options
#[test]
fn test_sorting() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create files with different names and times
    fs::write(temp_path.join("z_file.txt"), "z").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(temp_path.join("a_file.txt"), "a").unwrap();

    // Test alphabetical sorting (default)
    let output = Command::new("cargo")
        .args(&["run", "--", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let a_pos = stdout.find("a_file.txt").unwrap();
    let z_pos = stdout.find("z_file.txt").unwrap();
    assert!(a_pos < z_pos, "Files should be sorted alphabetically");

    // Test reverse sorting
    let output_reverse = Command::new("cargo")
        .args(&["run", "--", "-r", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -r command");

    let stdout_reverse = String::from_utf8_lossy(&output_reverse.stdout);
    let a_pos_reverse = stdout_reverse.find("a_file.txt").unwrap();
    let z_pos_reverse = stdout_reverse.find("z_file.txt").unwrap();
    assert!(z_pos_reverse < a_pos_reverse, "Files should be sorted in reverse");
}

/// Test Unix compatibility mode
#[test]
fn test_unix_compat_mode() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("test.txt"), "content").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-u", "-l", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -u -l command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain Unix-style permissions
    assert!(stdout.contains("-rw") || stdout.contains("drw"));
}

/// Test Windows attributes display
#[test]
fn test_windows_attributes() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("test.txt"), "content").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-w", "-l", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -w -l command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain Windows attribute flags
    assert!(stdout.contains("-") || stdout.contains("d"));
}

/// Test human-readable sizes
#[test]
fn test_human_readable_sizes() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a file with known size
    let content = "x".repeat(1024); // 1KB
    fs::write(temp_path.join("1kb.txt"), content).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "-h", "-l", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls -h -l command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain human-readable size format
    assert!(stdout.contains("KB") || stdout.contains("B") || stdout.contains("kB"));
}

/// Test multiple paths
#[test]
fn test_multiple_paths() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    fs::write(temp_dir1.path().join("file1.txt"), "content1").unwrap();
    fs::write(temp_dir2.path().join("file2.txt"), "content2").unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            temp_dir1.path().to_str().unwrap(),
            temp_dir2.path().to_str().unwrap()
        ])
        .current_dir(".")
        .output()
        .expect("Failed to run ls with multiple paths");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain files from both directories
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
}

/// Test performance with large directories
#[test]
fn test_large_directory_performance() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create many files to test performance
    for i in 0..100 {
        fs::write(temp_path.join(format!("file_{:03}.txt", i)), format!("content{}", i)).unwrap();
    }

    let start = std::time::Instant::now();

    let output = Command::new("cargo")
        .args(&["run", "--", "--stats", temp_path.to_str().unwrap()])
        .current_dir(".")
        .output()
        .expect("Failed to run ls with stats");

    let duration = start.elapsed();

    // Should complete within reasonable time (5 seconds for 100 files)
    assert!(duration.as_secs() < 5, "ls should complete quickly for 100 files");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should list all files
    assert!(stdout.matches("file_").count() >= 100);
}

/// Test error handling for non-existent paths
#[test]
fn test_error_handling() {
    let output = Command::new("cargo")
        .args(&["run", "--", "/this/path/does/not/exist"])
        .current_dir(".")
        .output()
        .expect("Failed to run ls command");

    // Should return error code for non-existent path
    assert!(!output.status.success());
}

/// Test path normalization
#[test]
fn test_path_normalization() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("test.txt"), "content").unwrap();

    // Test with different path separators
    let forward_slash_path = temp_path.to_str().unwrap().replace('\\', "/");

    let output = Command::new("cargo")
        .args(&["run", "--", &forward_slash_path])
        .current_dir(".")
        .output()
        .expect("Failed to run ls with forward slash path");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test.txt"));
}
