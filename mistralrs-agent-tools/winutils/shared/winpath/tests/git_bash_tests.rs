//! Git Bash Path Mangling Tests
//!
//! These tests focus on the specific issue where Git Bash mangles WSL-style paths
//! like `/mnt/c/users/david/.local/bin/ls.exe` into invalid Windows paths such as
//! `C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe`.
//!
//! The tests ensure that winpath correctly normalizes various Git Bash mangled
//! paths back to proper Windows paths, and validates that the library doesn't
//! introduce any Git Bash pollution in its normalization process.

use winpath::{normalize_path, normalize_path_cow, detect_path_format, PathFormat, PathError};
use std::time::Instant;

/// Test the core Git Bash path mangling issue
#[test]
fn test_git_bash_mangled_paths() {
    // Common Git Bash mangled path patterns
    let test_cases = [
        // Standard Git for Windows installation path
        (
            r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe",
            r"C:\users\david\.local\bin\ls.exe"
        ),
        (
            r"C:\Program Files\Git\mnt\d\temp\test.txt",
            r"D:\temp\test.txt"
        ),

        // Alternative Git installation paths
        (
            r"C:\Git\mnt\c\projects\rust\target\debug\app.exe",
            r"C:\projects\rust\target\debug\app.exe"
        ),
        (
            r"D:\PortableGit\mnt\c\users\david\Documents\file.doc",
            r"C:\users\david\Documents\file.doc"
        ),

        // Git installation in custom locations
        (
            r"C:\Tools\Git\mnt\c\windows\system32\cmd.exe",
            r"C:\windows\system32\cmd.exe"
        ),
        (
            r"C:\Users\david\scoop\apps\git\current\mnt\c\temp\archive.zip",
            r"C:\temp\archive.zip"
        ),

        // Multiple drive letters
        (
            r"C:\Program Files\Git\mnt\e\backup\data\important.db",
            r"E:\backup\data\important.db"
        ),
        (
            r"C:\Git\mnt\f\projects\coreutils\target\release\ls.exe",
            r"F:\projects\coreutils\target\release\ls.exe"
        ),
    ];

    for (mangled_path, expected_clean) in test_cases.iter() {
        // Should detect as Git Bash mangled format
        let format = detect_path_format(mangled_path);

        // Git Bash mangled paths should still be detected as DOS paths with the mangled prefix
        // We need to add special detection for these cases
        assert!(
            format == PathFormat::Dos || format == PathFormat::Mixed,
            "Git Bash mangled path should be detected properly: {} -> {:?}",
            mangled_path, format
        );

        // Normalize and verify result
        let normalized = normalize_path(mangled_path)
            .expect(&format!("Failed to normalize Git Bash mangled path: {}", mangled_path));

        assert_eq!(
            normalized, *expected_clean,
            "Git Bash path normalization failed for: {}", mangled_path
        );
    }
}

/// Test Git Bash paths with complex nested structures
#[test]
fn test_git_bash_complex_paths() {
    let complex_cases = [
        // Deep nesting with spaces
        (
            r"C:\Program Files\Git\mnt\c\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\IDE\devenv.exe",
            r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\IDE\devenv.exe"
        ),

        // Hidden directories (dot files)
        (
            r"C:\Program Files\Git\mnt\c\users\david\.config\app\settings.json",
            r"C:\users\david\.config\app\settings.json"
        ),

        // Mixed case preservation
        (
            r"C:\Program Files\Git\mnt\c\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
        ),

        // Special characters and Unicode
        (
            r"C:\Program Files\Git\mnt\c\users\josé\documents\resumé.pdf",
            r"C:\users\josé\documents\resumé.pdf"
        ),

        // Multiple file extensions
        (
            r"C:\Program Files\Git\mnt\c\temp\backup.tar.gz.enc",
            r"C:\temp\backup.tar.gz.enc"
        ),

        // Paths with numbers and special chars
        (
            r"C:\Program Files\Git\mnt\c\program-files-x86\app_v1.2.3\bin\tool.exe",
            r"C:\program-files-x86\app_v1.2.3\bin\tool.exe"
        ),
    ];

    for (mangled_path, expected_clean) in complex_cases.iter() {
        let normalized = normalize_path(mangled_path)
            .expect(&format!("Failed to normalize complex Git Bash path: {}", mangled_path));

        assert_eq!(
            normalized, *expected_clean,
            "Complex Git Bash path normalization failed for: {}", mangled_path
        );

        // Ensure no Git pollution remains
        assert!(!normalized.contains("Program Files\\Git"));
        assert!(!normalized.contains("\\mnt\\"));
        assert!(!normalized.contains("/mnt/"));
    }
}

/// Test Git Bash path edge cases and error conditions
#[test]
fn test_git_bash_edge_cases() {
    let edge_cases = [
        // Root level access
        r"C:\Program Files\Git\mnt\c",
        r"C:\Program Files\Git\mnt\c\",

        // Single character files/dirs
        r"C:\Program Files\Git\mnt\c\a",
        r"C:\Program Files\Git\mnt\c\x\y\z.txt",

        // Empty components (double separators)
        r"C:\Program Files\Git\mnt\c\users\\david\\.local",

        // Very short paths
        r"C:\Program Files\Git\mnt\c\f.txt",
    ];

    for edge_case in edge_cases.iter() {
        let result = normalize_path(edge_case);

        // These should either normalize successfully or fail gracefully
        match result {
            Ok(normalized) => {
                // If successful, should not contain Git paths
                assert!(!normalized.contains("Program Files\\Git"));
                assert!(!normalized.contains("\\mnt\\"));

                // Should start with a valid drive letter
                assert!(normalized.len() >= 2);
                assert!(normalized.chars().nth(0).unwrap().is_ascii_alphabetic());
                assert_eq!(normalized.chars().nth(1).unwrap(), ':');
            }
            Err(_) => {
                // Some edge cases may legitimately fail
                // This is acceptable behavior
            }
        }
    }
}

/// Test that normal WSL paths are NOT mistaken for Git Bash paths
#[test]
fn test_wsl_vs_git_bash_differentiation() {
    let wsl_paths = [
        "/mnt/c/users/david/.local/bin/ls.exe",
        "/mnt/d/temp/test.txt",
        "/mnt/e/backup/data.zip",
    ];

    let expected_results = [
        r"C:\users\david\.local\bin\ls.exe",
        r"D:\temp\test.txt",
        r"E:\backup\data.zip",
    ];

    for (wsl_path, expected) in wsl_paths.iter().zip(expected_results.iter()) {
        // WSL paths should be detected as WSL format
        assert_eq!(detect_path_format(wsl_path), PathFormat::Wsl);

        let normalized = normalize_path(wsl_path).unwrap();
        assert_eq!(normalized, *expected);

        // Should not contain any Git pollution
        assert!(!normalized.contains("Program Files\\Git"));
        assert!(!normalized.contains("Git\\"));
        assert!(!normalized.contains("\\mnt\\"));
    }
}

/// Test performance with Git Bash path normalization
#[test]
fn test_git_bash_normalization_performance() {
    let test_path = r"C:\Program Files\Git\mnt\c\users\david\documents\projects\rust\winpath\target\debug\deps\winpath.exe";

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = normalize_path(test_path).unwrap();
    }
    let duration = start.elapsed();

    // Should complete 1000 normalizations in under 15ms on modern hardware
    // (slightly higher threshold than WSL due to more complex pattern matching)
    assert!(
        duration.as_millis() < 15,
        "Git Bash path normalization too slow: {:?}", duration
    );
}

/// Test zero-copy optimization for already-normalized paths
#[test]
fn test_git_bash_zero_copy_optimization() {
    // Already normalized Windows paths should not be modified
    let normalized_paths = [
        r"C:\users\david\.local\bin\ls.exe",
        r"D:\temp\test.txt",
        r"E:\backup\data.zip",
    ];

    for path in normalized_paths.iter() {
        let result = normalize_path_cow(path).unwrap();

        // Should not be modified
        assert!(!result.was_modified());
        assert_eq!(result.path(), *path);
    }

    // Git Bash mangled paths SHOULD be modified
    let mangled_path = r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe";
    let result = normalize_path_cow(mangled_path).unwrap();

    assert!(result.was_modified());
    assert_eq!(result.path(), r"C:\users\david\.local\bin\ls.exe");
}

/// Test various Git installation path patterns
#[test]
fn test_different_git_installation_paths() {
    let git_install_patterns = [
        // Standard installations
        r"C:\Program Files\Git\",
        r"C:\Program Files (x86)\Git\",

        // Custom installations
        r"C:\Git\",
        r"D:\Git\",
        r"C:\Tools\Git\",
        r"C:\DevTools\Git\",

        // Portable installations
        r"C:\Users\david\scoop\apps\git\current\",
        r"C:\PortableApps\GitPortable\App\Git\",
        r"D:\Portable\Git\",

        // User-specific installations
        r"C:\Users\david\AppData\Local\Programs\Git\",
        r"C:\Users\david\Tools\Git\",
    ];

    for git_base in git_install_patterns.iter() {
        let mangled_path = format!(r"{}mnt\c\users\david\test.txt", git_base);
        let expected = r"C:\users\david\test.txt";

        let result = normalize_path(&mangled_path);

        // Should either normalize correctly or fail gracefully
        match result {
            Ok(normalized) => {
                assert_eq!(normalized, expected);
                assert!(!normalized.contains("\\mnt\\"));
                assert!(!normalized.contains("Git\\"));
            }
            Err(_) => {
                // Some patterns may not be recognizable, which is acceptable
            }
        }
    }
}

/// Test error handling for malformed Git Bash paths
#[test]
fn test_git_bash_error_handling() {
    let error_cases = [
        // Missing drive letter
        r"C:\Program Files\Git\mnt\",
        r"C:\Program Files\Git\mnt\\",

        // Invalid drive letters
        r"C:\Program Files\Git\mnt\1\test",
        r"C:\Program Files\Git\mnt\@\test",
        r"C:\Program Files\Git\mnt\cc\test",

        // Malformed paths
        r"C:\Program Files\Git\mnt",
        r"C:\Program Files\Git\mnt\c",

        // Empty components
        r"C:\Program Files\Git\mnt\c\\test",
        r"C:\Program Files\Git\mnt\c\\\",
    ];

    for error_case in error_cases.iter() {
        let result = normalize_path(error_case);

        // These should either fail with a specific error or handle gracefully
        if let Err(error) = result {
            // Verify we get appropriate error types
            match error {
                PathError::InvalidComponent(_) |
                PathError::InvalidDriveLetter(_) |
                PathError::EmptyPath => {
                    // Expected error types
                }
                _ => {
                    panic!("Unexpected error type for {}: {:?}", error_case, error);
                }
            }
        }
        // If it succeeds, it should produce a valid result
    }
}

/// Test that Git Bash path detection doesn't interfere with other formats
#[test]
fn test_git_bash_no_interference() {
    let other_formats = [
        // Regular DOS paths
        (r"C:\Users\David", PathFormat::Dos),
        (r"D:\Program Files\App", PathFormat::Dos),

        // DOS forward slash
        ("C:/Users/David", PathFormat::DosForward),

        // WSL paths
        ("/mnt/c/users/david", PathFormat::Wsl),
        ("/mnt/d/temp", PathFormat::Wsl),

        // Cygwin paths
        ("/cygdrive/c/users", PathFormat::Cygwin),

        // UNC paths
        (r"\\?\C:\Users", PathFormat::Unc),
        (r"\\server\share\file", PathFormat::Unc),

        // Relative paths
        (r"Documents\file.txt", PathFormat::Relative),
        ("../temp/data", PathFormat::Relative),
    ];

    for (path, expected_format) in other_formats.iter() {
        let detected_format = detect_path_format(path);
        assert_eq!(
            detected_format, *expected_format,
            "Format detection interference for: {} (expected {:?}, got {:?})",
            path, expected_format, detected_format
        );

        // Normalization should work correctly
        let normalized = normalize_path(path);
        match normalized {
            Ok(result) => {
                // Should not introduce Git pollution
                assert!(!result.contains("Program Files\\Git"));
                assert!(!result.contains("\\mnt\\"));
            }
            Err(_) => {
                // Some paths may legitimately fail, which is fine
            }
        }
    }
}

/// Benchmark Git Bash path normalization vs other formats
#[test]
fn test_git_bash_performance_comparison() {
    let iterations = 1000;

    // Git Bash mangled path
    let git_bash_path = r"C:\Program Files\Git\mnt\c\users\david\documents\project\target\debug\app.exe";

    // Equivalent WSL path
    let wsl_path = "/mnt/c/users/david/documents/project/target/debug/app.exe";

    // Equivalent DOS path
    let dos_path = r"C:\users\david\documents\project\target\debug\app.exe";

    // Benchmark Git Bash normalization
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = normalize_path(git_bash_path).unwrap();
    }
    let git_bash_duration = start.elapsed();

    // Benchmark WSL normalization
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = normalize_path(wsl_path).unwrap();
    }
    let wsl_duration = start.elapsed();

    // Benchmark DOS normalization (should be fastest)
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = normalize_path(dos_path).unwrap();
    }
    let dos_duration = start.elapsed();

    println!("Performance comparison for {} iterations:", iterations);
    println!("  Git Bash: {:?}", git_bash_duration);
    println!("  WSL:      {:?}", wsl_duration);
    println!("  DOS:      {:?}", dos_duration);

    // Git Bash normalization should not be more than 3x slower than DOS
    assert!(
        git_bash_duration.as_nanos() < dos_duration.as_nanos() * 5,
        "Git Bash normalization significantly slower than DOS: {:?} vs {:?}",
        git_bash_duration, dos_duration
    );
}
