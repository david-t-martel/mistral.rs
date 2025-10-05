//! Comprehensive tests for WSL path normalization scenarios
//!
//! These tests validate the specific issue reported where WSL-style paths
//! like `/mnt/c/users/david/.local/bin/ls.exe` should normalize correctly
//! to `C:\users\david\.local\bin\ls.exe`.

use winpath::{normalize_path, detect_path_format, PathFormat};

#[test]
fn test_wsl_path_normalization_issue() {
    // The specific problematic case from the issue report
    let problematic_path = "/mnt/c/users/david/.local/bin/ls.exe";
    let expected = r"C:\users\david\.local\bin\ls.exe";

    assert_eq!(detect_path_format(problematic_path), PathFormat::Wsl);
    assert_eq!(normalize_path(problematic_path).unwrap(), expected);
}

#[test]
fn test_wsl_paths_various_drives() {
    let test_cases = [
        ("/mnt/c/users/david/documents", r"C:\users\david\documents"),
        ("/mnt/d/temp/file.txt", r"D:\temp\file.txt"),
        ("/mnt/e/backup/data", r"E:\backup\data"),
        ("/mnt/f/projects/rust", r"F:\projects\rust"),
    ];

    for (input, expected) in test_cases.iter() {
        assert_eq!(detect_path_format(input), PathFormat::Wsl);
        assert_eq!(normalize_path(input).unwrap(), *expected);
    }
}

#[test]
fn test_wsl_paths_complex_scenarios() {
    let test_cases = [
        // Deep nested paths
        ("/mnt/c/Program Files/Microsoft/Visual Studio/2022/Enterprise/Common7/IDE",
         r"C:\Program Files\Microsoft\Visual Studio\2022\Enterprise\Common7\IDE"),

        // Paths with spaces
        ("/mnt/c/users/david/My Documents/Important Files",
         r"C:\users\david\My Documents\Important Files"),

        // Mixed case preservation
        ("/mnt/c/Windows/System32/drivers",
         r"C:\Windows\System32\drivers"),

        // Hidden directories (starting with dot)
        ("/mnt/c/users/david/.config/app",
         r"C:\users\david\.config\app"),

        // Multiple extensions
        ("/mnt/c/temp/archive.tar.gz",
         r"C:\temp\archive.tar.gz"),
    ];

    for (input, expected) in test_cases.iter() {
        assert_eq!(detect_path_format(input), PathFormat::Wsl);
        assert_eq!(normalize_path(input).unwrap(), *expected);
    }
}

#[test]
fn test_wsl_path_edge_cases() {
    // Root paths
    assert_eq!(normalize_path("/mnt/c").unwrap(), r"C:");
    assert_eq!(normalize_path("/mnt/c/").unwrap(), r"C:\");

    // Single character files/directories
    assert_eq!(normalize_path("/mnt/c/a").unwrap(), r"C:\a");
    assert_eq!(normalize_path("/mnt/c/x/y/z").unwrap(), r"C:\x\y\z");
}

#[test]
fn test_wsl_path_error_cases() {
    let error_cases = [
        "/mnt/",           // Incomplete path
        "/mnt/z/test",     // Invalid drive letter
        "/mnt/1/test",     // Numeric drive
        "/mnt/cc/test",    // Multi-character drive
        "/mnt//test",      // Empty drive component
    ];

    for input in error_cases.iter() {
        assert_eq!(detect_path_format(input), PathFormat::Wsl);
        assert!(normalize_path(input).is_err(), "Expected error for: {}", input);
    }
}

#[test]
fn test_no_git_bash_pollution() {
    // Ensure that WSL paths don't get polluted with Git Bash paths
    // The original issue was paths becoming: C:\Program Files\Git\mnt\c\...

    let wsl_path = "/mnt/c/users/david/.local/bin/ls.exe";
    let normalized = normalize_path(wsl_path).unwrap();

    // Should NOT contain any Git Bash paths
    assert!(!normalized.contains("Program Files\\Git"));
    assert!(!normalized.contains("/usr/bin"));
    assert!(!normalized.contains("\\mnt\\"));

    // Should be a clean Windows path
    assert!(normalized.starts_with("C:\\"));
    assert_eq!(normalized, r"C:\users\david\.local\bin\ls.exe");
}

#[test]
fn test_wsl_path_performance() {
    // Test that normalization is reasonably fast
    let test_path = "/mnt/c/users/david/documents/projects/rust/target/debug/deps";

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = normalize_path(test_path).unwrap();
    }
    let duration = start.elapsed();

    // Should complete 1000 normalizations in under 10ms on modern hardware
    assert!(duration.as_millis() < 10, "Normalization too slow: {:?}", duration);
}

#[test]
fn test_wsl_vs_cygwin_differentiation() {
    // Ensure WSL and Cygwin paths are properly differentiated
    let wsl_path = "/mnt/c/users/david";
    let cygwin_path = "/cygdrive/c/users/david";
    let expected = r"C:\users\david";

    assert_eq!(detect_path_format(wsl_path), PathFormat::Wsl);
    assert_eq!(detect_path_format(cygwin_path), PathFormat::Cygwin);

    // Both should normalize to the same Windows path
    assert_eq!(normalize_path(wsl_path).unwrap(), expected);
    assert_eq!(normalize_path(cygwin_path).unwrap(), expected);
}

#[test]
fn test_wsl_path_with_special_characters() {
    // Test paths with characters that need special handling
    let test_cases = [
        // Unicode characters
        ("/mnt/c/users/josé/documents", r"C:\users\josé\documents"),

        // Numbers and underscores
        ("/mnt/c/program_files_x86/app_v1.2", r"C:\program_files_x86\app_v1.2"),

        // Hyphens and periods
        ("/mnt/c/my-app/config.json", r"C:\my-app\config.json"),
    ];

    for (input, expected) in test_cases.iter() {
        assert_eq!(normalize_path(input).unwrap(), *expected);
    }
}
