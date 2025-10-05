//! Basic usage examples for the winpath library.

use winpath::{normalize_path, normalize_path_cow, PathNormalizer, detect_path_format};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== WinPath Library Examples ===\n");

    // Example 1: Basic path normalization
    println!("1. Basic Path Normalization:");
    demonstrate_basic_normalization()?;

    // Example 2: Path format detection
    println!("\n2. Path Format Detection:");
    demonstrate_format_detection();

    // Example 3: Zero-copy optimization
    println!("\n3. Zero-Copy Optimization:");
    demonstrate_zero_copy()?;

    // Example 4: Using the PathNormalizer
    println!("\n4. PathNormalizer Usage:");
    demonstrate_path_normalizer()?;

    // Example 5: Batch processing
    println!("\n5. Batch Processing:");
    demonstrate_batch_processing()?;

    // Example 6: Error handling
    println!("\n6. Error Handling:");
    demonstrate_error_handling();

    // Example 7: Long path handling
    println!("\n7. Long Path Handling:");
    demonstrate_long_paths()?;

    Ok(())
}

fn demonstrate_basic_normalization() -> Result<(), Box<dyn std::error::Error>> {
    let test_paths = vec![
        r"C:\Users\David",              // Already normalized DOS path
        "C:/Users/David/Documents",     // DOS with forward slashes
        "/mnt/c/users/david",          // WSL mount path
        "/cygdrive/c/users/david",     // Cygwin path
        r"\\?\C:\Users\David",         // UNC long path
        "//c/users/david",             // Unix-like path
        r"C:\Users/David\Documents",   // Mixed separators
    ];

    for path in test_paths {
        let normalized = normalize_path(path)?;
        println!("  {} -> {}", path, normalized);
    }

    Ok(())
}

fn demonstrate_format_detection() {
    let test_paths = vec![
        (r"C:\Users\David", "DOS path"),
        ("C:/Users/David", "DOS with forward slashes"),
        ("/mnt/c/users/david", "WSL mount path"),
        ("/cygdrive/c/users/david", "Cygwin path"),
        (r"\\?\C:\Users\David", "UNC long path"),
        ("//c/users/david", "Unix-like path"),
        (r"C:\Users/David\Documents", "Mixed separators"),
        ("Documents\\file.txt", "Relative path"),
    ];

    for (path, description) in test_paths {
        let format = detect_path_format(path);
        println!("  {:<30} -> {:?} ({})", path, format, description);
    }
}

fn demonstrate_zero_copy() -> Result<(), Box<dyn std::error::Error>> {
    let already_normalized = r"C:\Users\David\Documents";
    let needs_normalization = "/mnt/c/users/david/documents";

    // This won't allocate - returns borrowed string
    let result1 = normalize_path_cow(already_normalized)?;
    println!("  Already normalized (no allocation): {}", result1.path());
    println!("    Was modified: {}", result1.was_modified());

    // This will allocate - returns owned string
    let result2 = normalize_path_cow(needs_normalization)?;
    println!("  Needs normalization (allocation): {}", result2.path());
    println!("    Was modified: {}", result2.was_modified());
    println!("    Original format: {:?}", result2.original_format());

    Ok(())
}

fn demonstrate_path_normalizer() -> Result<(), Box<dyn std::error::Error>> {
    // Create a normalizer with caching enabled
    let normalizer = PathNormalizer::new();

    let test_paths = vec![
        "/mnt/c/users/david",
        "/mnt/d/projects/rust",
        "C:/Windows/System32",
    ];

    // First pass - cache misses
    println!("  First normalization (cache misses):");
    for path in &test_paths {
        let result = normalizer.normalize(path)?;
        println!("    {} -> {}", path, result.path());
    }

    // Second pass - cache hits (if caching is enabled)
    println!("  Second normalization (potential cache hits):");
    for path in &test_paths {
        let result = normalizer.normalize(path)?;
        println!("    {} -> {}", path, result.path());
    }

    // Show cache statistics if available
    #[cfg(feature = "cache")]
    if let Some(stats) = normalizer.cache_stats() {
        println!("  Cache statistics:");
        println!("    Size: {}/{}", stats.size, stats.capacity);
        println!("    Hits: {}, Misses: {}", stats.hits, stats.misses);
        println!("    Hit rate: {:.1}%", stats.hit_rate * 100.0);
    }

    Ok(())
}

fn demonstrate_batch_processing() -> Result<(), Box<dyn std::error::Error>> {
    let normalizer = PathNormalizer::new();

    let paths_to_process = vec![
        "/mnt/c/users/alice/documents",
        "/mnt/c/users/bob/pictures",
        "/cygdrive/d/projects/myapp",
        "C:/Windows/Temp",
        "/mnt/e/backup/files",
    ];

    // Process individually
    println!("  Individual processing:");
    for path in &paths_to_process {
        let result = normalizer.normalize_to_string(path)?;
        println!("    {} -> {}", path, result);
    }

    // Process as batch
    println!("  Batch processing:");
    let results = normalizer.normalize_batch(&paths_to_process)?;
    for (original, normalized) in paths_to_process.iter().zip(results.iter()) {
        println!("    {} -> {}", original, normalized);
    }

    Ok(())
}

fn demonstrate_error_handling() {
    let error_cases = vec![
        ("", "Empty path"),
        ("/mnt/", "Incomplete WSL path"),
        ("/mnt/invalid_drive/test", "Invalid drive letter"),
        ("/cygdrive/", "Incomplete Cygwin path"),
        (r"C:\con", "Reserved filename"),
        (r"C:\file<invalid>name", "Invalid character"),
    ];

    for (path, description) in error_cases {
        match normalize_path(path) {
            Ok(result) => println!("  {} -> {} (unexpected success)", path, result),
            Err(error) => println!("  {} -> Error: {} ({})", path, error, description),
        }
    }
}

fn demonstrate_long_paths() -> Result<(), Box<dyn std::error::Error>> {
    // Create a path that exceeds MAX_PATH (260 characters)
    let long_component = "very_long_directory_name_that_exceeds_normal_limits".repeat(3);
    let long_wsl_path = format!("/mnt/c/users/david/{}/file.txt", long_component);
    let long_dos_path = format!(r"C:\Users\David\{}\file.txt", long_component);

    println!("  Long WSL path ({} chars):", long_wsl_path.len());
    let normalized = normalize_path_cow(&long_wsl_path)?;
    println!("    Normalized: {}", normalized.path());
    println!("    Has long prefix: {}", normalized.has_long_path_prefix());
    println!("    Length: {}", normalized.len());

    println!("  Long DOS path ({} chars):", long_dos_path.len());
    let normalized = normalize_path_cow(&long_dos_path)?;
    println!("    Normalized: {}", normalized.path());
    println!("    Has long prefix: {}", normalized.has_long_path_prefix());
    println!("    Length: {}", normalized.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_run_without_panic() {
        // This test ensures all examples can run without panicking
        assert!(main().is_ok());
    }
}
