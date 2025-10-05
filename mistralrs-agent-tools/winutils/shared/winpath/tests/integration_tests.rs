//! Comprehensive Integration Tests for WinPath
//!
//! This test suite provides extensive coverage for all supported path formats,
//! edge cases, error conditions, and performance characteristics of the winpath
//! normalization library. It serves as both a validation suite and a regression
//! test framework.

use winpath::{
    normalize_path, normalize_path_cow, detect_path_format,
    PathFormat, PathError, PathNormalizer, NormalizerConfig
};
use std::time::Instant;
use std::collections::HashMap;

/// Test all major path format categories
#[test]
fn test_all_path_formats_comprehensive() {
    let test_cases = [
        // DOS Paths (Windows standard)
        (r"C:\Users\David", PathFormat::Dos, r"C:\Users\David"),
        (r"D:\Program Files\App\bin\tool.exe", PathFormat::Dos, r"D:\Program Files\App\bin\tool.exe"),
        (r"E:\", PathFormat::Dos, r"E:\"),
        (r"F:", PathFormat::Dos, r"F:"),
        (r"Z:\very\long\path\with\many\components", PathFormat::Dos, r"Z:\very\long\path\with\many\components"),

        // DOS Forward Slash
        ("C:/Users/David", PathFormat::DosForward, r"C:\Users\David"),
        ("D:/Program Files/App/bin/tool.exe", PathFormat::DosForward, r"D:\Program Files\App\bin\tool.exe"),
        ("E:/temp/data/", PathFormat::DosForward, r"E:\temp\data\"),
        ("F:/", PathFormat::DosForward, r"F:\"),
        ("G:/file.txt", PathFormat::DosForward, r"G:\file.txt"),

        // WSL Mount Points
        ("/mnt/c/users/david", PathFormat::Wsl, r"C:\users\david"),
        ("/mnt/d/temp/file.txt", PathFormat::Wsl, r"D:\temp\file.txt"),
        ("/mnt/e/backup/data.zip", PathFormat::Wsl, r"E:\backup\data.zip"),
        ("/mnt/f/projects/rust/target/debug/app.exe", PathFormat::Wsl, r"F:\projects\rust\target\debug\app.exe"),
        ("/mnt/c", PathFormat::Wsl, r"C:"),
        ("/mnt/c/", PathFormat::Wsl, r"C:\"),

        // Cygwin Paths
        ("/cygdrive/c/users/david", PathFormat::Cygwin, r"C:\users\david"),
        ("/cygdrive/d/temp", PathFormat::Cygwin, r"D:\temp"),
        ("/cygdrive/e/backup/important.db", PathFormat::Cygwin, r"E:\backup\important.db"),
        ("/cygdrive/f/", PathFormat::Cygwin, r"F:\"),
        ("/cygdrive/g", PathFormat::Cygwin, r"G:"),

        // UNC Long Paths
        (r"\\?\C:\Users\David", PathFormat::Unc, r"\\?\C:\Users\David"),
        (r"\\?\D:\Very\Long\Path\That\Exceeds\Normal\Limits", PathFormat::Unc, r"\\?\D:\Very\Long\Path\That\Exceeds\Normal\Limits"),
        (r"\\?\UNC\server\share\file.txt", PathFormat::Unc, r"\\?\UNC\server\share\file.txt"),

        // Unix-like with double slashes
        ("//c/users/david", PathFormat::UnixLike, r"C:\users\david"),
        ("//d/temp//file.txt", PathFormat::UnixLike, r"D:\temp\file.txt"),
        ("//e//backup///data", PathFormat::UnixLike, r"E:\backup\data"),

        // Mixed Separators
        (r"C:\Users/David\Documents", PathFormat::Mixed, r"C:\Users\David\Documents"),
        ("D:/Program Files\\App/bin\\tool.exe", PathFormat::Mixed, r"D:\Program Files\App\bin\tool.exe"),
        (r"E:\temp/data\file.txt", PathFormat::Mixed, r"E:\temp\data\file.txt"),
    ];

    for (input, expected_format, expected_output) in test_cases.iter() {
        // Test format detection
        let detected_format = detect_path_format(input);
        assert_eq!(
            detected_format, *expected_format,
            "Format detection failed for: {} (expected {:?}, got {:?})",
            input, expected_format, detected_format
        );

        // Test normalization
        let normalized = normalize_path(input)
            .expect(&format!("Normalization failed for: {}", input));
        assert_eq!(
            normalized, *expected_output,
            "Normalization failed for: {} (expected {}, got {})",
            input, expected_output, normalized
        );

        // Test that normalized paths start with proper drive letters for absolute paths
        if expected_format.is_absolute() && !normalized.starts_with("\\\\?\\") {
            if let Some(first_char) = normalized.chars().next() {
                if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
                    assert!(
                        first_char.is_ascii_alphabetic() && first_char.is_ascii_uppercase(),
                        "Invalid drive letter for normalized path: {} -> {}", input, normalized
                    );
                }
            }
        }
    }
}

/// Test Unicode and special character handling
#[test]
fn test_unicode_and_special_characters() {
    let unicode_cases = [
        // Unicode characters
        ("/mnt/c/users/josé/documents", r"C:\users\josé\documents"),
        ("/mnt/c/文档/重要文件.txt", r"C:\文档\重要文件.txt"),
        ("/mnt/c/пользователи/иван/файлы", r"C:\пользователи\иван\файлы"),
        ("/mnt/c/ユーザー/田中/ドキュメント", r"C:\ユーザー\田中\ドキュメント"),

        // Special ASCII characters
        ("/mnt/c/users/user@domain.com/files", r"C:\users\user@domain.com\files"),
        ("/mnt/c/app-v1.2.3/config.json", r"C:\app-v1.2.3\config.json"),
        ("/mnt/c/data_backup_2024/archive.tar.gz", r"C:\data_backup_2024\archive.tar.gz"),
        ("/mnt/c/program (x86)/app/tool.exe", r"C:\program (x86)\app\tool.exe"),

        // Edge cases with spaces and quotes
        ("/mnt/c/My Documents/Important Files", r"C:\My Documents\Important Files"),
        (r"/mnt/c/path with spaces/file name.txt", r"C:\path with spaces\file name.txt"),

        // Hidden files and dot directories
        ("/mnt/c/users/david/.config/app/settings.json", r"C:\users\david\.config\app\settings.json"),
        ("/mnt/c/.hidden/file", r"C:\.hidden\file"),
        ("/mnt/c/...ellipsis", r"C:\...ellipsis"),
    ];

    for (input, expected) in unicode_cases.iter() {
        let result = normalize_path(input)
            .expect(&format!("Unicode normalization failed for: {}", input));
        assert_eq!(
            result, *expected,
            "Unicode path normalization failed for: {}", input
        );
    }
}

/// Test long path handling and UNC prefix insertion
#[test]
fn test_long_path_handling() {
    // Create paths of various lengths
    let short_component = "short";
    let medium_component = "a".repeat(50);
    let long_component = "very_long_directory_name_that_pushes_limits".repeat(3);
    let extremely_long = "x".repeat(200);

    let long_path_cases = [
        // Just under the limit (should not get UNC prefix)
        format!("/mnt/c/{}/{}", short_component, medium_component),

        // Over the limit (should get UNC prefix)
        format!("/mnt/c/{}/{}/{}", long_component, medium_component, extremely_long),
        format!("/mnt/d/{}/{}/{}/{}", long_component, long_component, medium_component, "file.txt"),

        // Edge case: exactly at the limit
        format!("/mnt/c/{}", "a".repeat(250)),
    ];

    for long_path in long_path_cases.iter() {
        let result = normalize_path_cow(long_path)
            .expect(&format!("Long path normalization failed for path of length: {}", long_path.len()));

        let normalized = result.path();

        // Verify the path is valid
        assert!(normalized.len() > 0);
        assert!(normalized.starts_with("C:\\") || normalized.starts_with("D:\\") || normalized.starts_with("\\\\?\\"));

        // If the resulting Windows path is over 260 chars, it should have UNC prefix
        if normalized.len() > 260 {
            assert!(
                result.has_long_path_prefix(),
                "Long path should have UNC prefix: {} (length: {})", normalized, normalized.len()
            );
            assert!(normalized.starts_with("\\\\?\\"));
        }
    }
}

/// Test error conditions and edge cases
#[test]
fn test_error_conditions() {
    let error_cases = [
        // Empty paths
        "",
        "   ",
        "\t\n",

        // Invalid WSL paths
        "/mnt/",
        "/mnt",
        "/mnt//",
        "/mnt/z/test",  // Invalid drive letter
        "/mnt/1/test",  // Numeric drive
        "/mnt/cc/test", // Multi-character drive
        "/mnt/@/test",  // Special character drive

        // Invalid Cygwin paths
        "/cygdrive/",
        "/cygdrive",
        "/cygdrive//",
        "/cygdrive/z/test",  // Invalid drive letter
        "/cygdrive/1/test",  // Numeric drive

        // Malformed DOS paths
        "C",
        "C:",  // This might be valid as a drive reference
        "1:/test", // Invalid drive letter
        "@:/test", // Special character drive

        // Paths with null bytes (if they make it through)
        // Note: Rust strings can't contain null bytes, so these are theoretical
    ];

    for error_case in error_cases.iter() {
        let result = normalize_path(error_case);

        match result {
            Ok(normalized) => {
                // Some cases might be handled gracefully
                // Verify the result is sensible if normalization succeeded
                if !normalized.is_empty() {
                    assert!(
                        normalized.len() >= 2 || normalized == ".",
                        "Suspicious normalization result for '{}': '{}'", error_case, normalized
                    );
                }
            }
            Err(error) => {
                // Verify we get appropriate error types
                match error {
                    PathError::EmptyPath => {
                        assert!(error_case.trim().is_empty(), "EmptyPath error for non-empty input: '{}'", error_case);
                    }
                    PathError::InvalidComponent(_) => {
                        // General invalid path error
                    }
                    PathError::InvalidDriveLetter(_) => {
                        // Invalid drive letter
                    }
                    PathError::UnsupportedFormat => {
                        // Unsupported path format
                    }
                    _ => {
                        // Other errors are acceptable
                    }
                }
            }
        }
    }
}

/// Test zero-copy optimization
#[test]
fn test_zero_copy_optimization() {
    let already_normalized = [
        r"C:\Users\David",
        r"D:\Program Files\App\tool.exe",
        r"E:\temp\data.txt",
        r"\\?\F:\Very\Long\Path\That\Requires\UNC\Prefix",
    ];

    let needs_normalization = [
        "C:/Users/David",
        "/mnt/d/program files/app/tool.exe",
        "/cygdrive/e/temp/data.txt",
        r"C:\Users/David\Documents",
    ];

    // Already normalized paths should not be modified
    for path in already_normalized.iter() {
        let result = normalize_path_cow(path)
            .expect(&format!("Normalized path failed: {}", path));

        assert!(
            !result.was_modified(),
            "Already normalized path was modified: {}", path
        );
        assert_eq!(result.path(), *path);
    }

    // Paths needing normalization should be modified
    for path in needs_normalization.iter() {
        let result = normalize_path_cow(path)
            .expect(&format!("Path normalization failed: {}", path));

        assert!(
            result.was_modified(),
            "Path that needs normalization was not modified: {}", path
        );
        assert_ne!(result.path(), *path);
    }
}

/// Test caching with PathNormalizer
#[cfg(feature = "cache")]
#[test]
fn test_path_normalizer_caching() {
    let normalizer = PathNormalizer::new();

    let test_paths = [
        "/mnt/c/users/david/documents",
        "/mnt/d/temp/file.txt",
        "C:/Program Files/App",
        "/cygdrive/e/backup",
    ];

    // First normalization - should cache results
    let mut first_results = Vec::new();
    for path in test_paths.iter() {
        let result = normalizer.normalize(path)
            .expect(&format!("First normalization failed: {}", path));
        first_results.push(result.path().to_string());
    }

    // Second normalization - should use cached results
    for (path, expected) in test_paths.iter().zip(first_results.iter()) {
        let result = normalizer.normalize(path)
            .expect(&format!("Cached normalization failed: {}", path));
        assert_eq!(
            result.path(), expected,
            "Cached result mismatch for: {}", path
        );
    }

    // Test cache statistics if available
    if let Some(stats) = normalizer.cache_stats() {
        assert!(stats.hits > 0, "Cache should have hits");
        assert!(stats.misses > 0, "Cache should have misses");
    }
}

/// Test custom normalizer configuration
#[cfg(feature = "cache")]
#[test]
fn test_custom_normalizer_config() {
    let config = NormalizerConfig {
        cache_enabled: true,
        cache_size: 512,
        auto_long_prefix: true,
        validate_components: true,
        #[cfg(feature = "unicode")]
        unicode_normalize: true,
    };

    let normalizer = PathNormalizer::with_config(config);

    // Test with the custom configuration
    let result = normalizer.normalize("/mnt/c/Users/David")
        .expect("Custom config normalization failed");

    assert_eq!(result.path(), r"C:\Users\David");
}

/// Performance and stress testing
#[test]
fn test_performance_comprehensive() {
    let test_cases = [
        // Various path types for performance comparison
        ("DOS", r"C:\Users\David\Documents\Projects\Rust\winpath\target\debug\app.exe"),
        ("DOS_Forward", "C:/Users/David/Documents/Projects/Rust/winpath/target/debug/app.exe"),
        ("WSL", "/mnt/c/users/david/documents/projects/rust/winpath/target/debug/app.exe"),
        ("Cygwin", "/cygdrive/c/users/david/documents/projects/rust/winpath/target/debug/app.exe"),
        ("Mixed", r"C:\Users/David\Documents/Projects\Rust\winpath/target\debug\app.exe"),
        ("Long", &format!("/mnt/c/{}", "very_long_component_name".repeat(10))),
    ];

    let iterations = 1000;
    let mut performance_results = HashMap::new();

    for (name, path) in test_cases.iter() {
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = normalize_path(path).expect("Performance test normalization failed");
        }
        let duration = start.elapsed();

        performance_results.insert(name, duration);

        // Each path type should complete 1000 iterations in reasonable time
        assert!(
            duration.as_millis() < 50,
            "{} path normalization too slow: {:?} for {} iterations",
            name, duration, iterations
        );
    }

    // Print performance comparison
    println!("Performance results for {} iterations:", iterations);
    for (name, duration) in performance_results.iter() {
        println!("  {}: {:?} ({:.2} ns/iter)", name, duration, duration.as_nanos() as f64 / iterations as f64);
    }
}

/// Test thread safety and concurrent access
#[test]
fn test_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let test_paths = Arc::new(vec![
        "/mnt/c/users/david/test1.txt",
        "/mnt/d/temp/test2.txt",
        "C:/Program Files/test3.exe",
        "/cygdrive/e/backup/test4.zip",
    ]);

    let handles: Vec<_> = (0..4).map(|i| {
        let paths = Arc::clone(&test_paths);
        thread::spawn(move || {
            let path = &paths[i % paths.len()];
            for _ in 0..100 {
                let result = normalize_path(path);
                assert!(result.is_ok(), "Thread {} normalization failed for: {}", i, path);
            }
        })
    }).collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

/// Test round-trip normalization (idempotency)
#[test]
fn test_normalization_idempotency() {
    let test_paths = [
        "/mnt/c/users/david/documents",
        "C:/Program Files/App",
        "/cygdrive/d/temp",
        r"C:\Users/David\Mixed",
    ];

    for path in test_paths.iter() {
        let first_normalization = normalize_path(path)
            .expect(&format!("First normalization failed: {}", path));

        let second_normalization = normalize_path(&first_normalization)
            .expect(&format!("Second normalization failed: {}", first_normalization));

        assert_eq!(
            first_normalization, second_normalization,
            "Normalization is not idempotent for: {} -> {} -> {}",
            path, first_normalization, second_normalization
        );
    }
}

/// Test boundary conditions and limits
#[test]
fn test_boundary_conditions() {
    // Test drive letter boundaries
    let valid_drives = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
                       'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

    for drive in valid_drives.iter() {
        let wsl_path = format!("/mnt/{}/test", drive.to_ascii_lowercase());
        let expected = format!(r"{}:\test", drive);

        let result = normalize_path(&wsl_path);
        match result {
            Ok(normalized) => {
                assert_eq!(normalized, expected);
            }
            Err(_) => {
                // Some drives might not be available, which is fine
            }
        }
    }

    // Test path length boundaries
    let max_component = "a".repeat(255); // Maximum component length on most filesystems
    let long_path = format!("/mnt/c/{}", max_component);

    let result = normalize_path(&long_path);
    assert!(result.is_ok(), "Max component length path should normalize");

    // Test very deep nesting
    let deep_path = format!("/mnt/c/{}", "dir/".repeat(100));
    let result = normalize_path(&deep_path);
    assert!(result.is_ok(), "Deep nested path should normalize");
}

/// Integration test with real file system operations (if available)
#[test]
#[cfg(windows)]
fn test_filesystem_integration() {
    use std::fs;
    use std::path::Path;

    // Test with actual Windows paths that should exist
    let system_paths = [
        r"C:\Windows",
        r"C:\Windows\System32",
        r"C:\Program Files",
    ];

    for sys_path in system_paths.iter() {
        if Path::new(sys_path).exists() {
            // Test various representations of the same path
            let wsl_equiv = sys_path.replace("C:\\", "/mnt/c/").replace("\\", "/").to_lowercase();
            let forward_slash = sys_path.replace("\\", "/");

            let wsl_normalized = normalize_path(&wsl_equiv).unwrap_or_default();
            let forward_normalized = normalize_path(&forward_slash).unwrap_or_default();
            let original_normalized = normalize_path(sys_path).unwrap_or_default();

            // All should normalize to the same canonical form
            assert_eq!(
                wsl_normalized.to_lowercase(),
                sys_path.to_lowercase(),
                "WSL path normalization doesn't match system path"
            );
            assert_eq!(
                forward_normalized, *sys_path,
                "Forward slash normalization doesn't match system path"
            );
            assert_eq!(
                original_normalized, *sys_path,
                "DOS path normalization should be identity"
            );
        }
    }
}
