//! Test utilities and helpers for mistralrs-agent-tools tests.
//!
//! This module provides common testing utilities, fixtures, and helper functions
//! to make writing tests easier and more consistent across the crate.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Create a temporary directory for testing.
///
/// The directory will be automatically cleaned up when the `TempDir` is dropped.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::create_temp_dir;
///
/// let temp = create_temp_dir();
/// let file_path = temp.path().join("test.txt");
/// std::fs::write(&file_path, "test content").unwrap();
/// // temp dir is automatically cleaned up when it goes out of scope
/// ```
pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

/// Create a temporary file with specified content.
///
/// Returns a tuple of (`TempDir`, `PathBuf`) where the PathBuf points to the created file.
/// The TempDir must be kept alive for the file to exist.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::create_temp_file;
///
/// let (_temp, path) = create_temp_file("test.txt", "Hello, world!");
/// let content = std::fs::read_to_string(&path).unwrap();
/// assert_eq!(content, "Hello, world!");
/// ```
pub fn create_temp_file(filename: &str, content: &str) -> (TempDir, PathBuf) {
    let temp = create_temp_dir();
    let path = temp.path().join(filename);
    fs::write(&path, content).expect("Failed to write temporary file");
    (temp, path)
}

/// Create a temporary directory structure with multiple files.
///
/// # Arguments
///
/// * `files` - Slice of tuples containing (relative_path, content)
///
/// # Returns
///
/// A `TempDir` containing all the created files
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::create_temp_file_structure;
///
/// let temp = create_temp_file_structure(&[
///     ("file1.txt", "content 1"),
///     ("dir/file2.txt", "content 2"),
///     ("dir/subdir/file3.txt", "content 3"),
/// ]);
/// ```
pub fn create_temp_file_structure(files: &[(&str, &str)]) -> TempDir {
    let temp = create_temp_dir();

    for (relative_path, content) in files {
        let path = temp.path().join(relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        fs::write(&path, content).expect("Failed to write file");
    }

    temp
}

/// Assert that two floating-point values are approximately equal within epsilon.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::assert_approx_eq;
///
/// assert_approx_eq(1.0, 1.0001, 0.001);
/// ```
///
/// # Panics
///
/// Panics if the values are not within epsilon of each other.
pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
    assert!(
        (a - b).abs() <= epsilon,
        "Values not approximately equal: {} vs {} (epsilon: {})",
        a,
        b,
        epsilon
    );
}

/// Read a test fixture file from the fixtures directory.
///
/// # Arguments
///
/// * `name` - Name of the fixture file (without path)
///
/// # Returns
///
/// String content of the fixture file
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::load_fixture;
///
/// let data = load_fixture("sample_config.json");
/// ```
pub fn load_fixture(name: &str) -> String {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    fs::read_to_string(&fixture_path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", name))
}

/// Load a fixture as bytes.
///
/// Useful for binary fixtures like images or other binary data.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::load_fixture_bytes;
///
/// let data = load_fixture_bytes("sample_image.png");
/// ```
pub fn load_fixture_bytes(name: &str) -> Vec<u8> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    fs::read(&fixture_path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", name))
}

/// Create a sample configuration for testing.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::create_test_config;
///
/// let config = create_test_config();
/// ```
pub fn create_test_config() -> serde_json::Value {
    serde_json::json!({
        "model": "test-model",
        "temperature": 0.7,
        "max_tokens": 100
    })
}

/// Assert that a string contains a substring.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::assert_contains;
///
/// assert_contains("hello world", "world");
/// ```
///
/// # Panics
///
/// Panics if the haystack does not contain the needle.
pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "String '{}' does not contain '{}'",
        haystack,
        needle
    );
}

/// Assert that a string does not contain a substring.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::assert_not_contains;
///
/// assert_not_contains("hello world", "foo");
/// ```
///
/// # Panics
///
/// Panics if the haystack contains the needle.
pub fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "String '{}' contains '{}' (but shouldn't)",
        haystack,
        needle
    );
}

/// Assert that a path exists.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::assert_path_exists;
///
/// assert_path_exists("/path/to/file");
/// ```
///
/// # Panics
///
/// Panics if the path does not exist.
pub fn assert_path_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(path.exists(), "Path does not exist: {}", path.display());
}

/// Assert that a path does not exist.
///
/// # Examples
///
/// ```ignore
/// use mistralrs_agent_tools::test_utils::assert_path_not_exists;
///
/// assert_path_not_exists("/path/to/nonexistent");
/// ```
///
/// # Panics
///
/// Panics if the path exists.
pub fn assert_path_not_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(
        !path.exists(),
        "Path exists (but shouldn't): {}",
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_dir() {
        let temp = create_temp_dir();
        assert!(temp.path().exists());
        assert!(temp.path().is_dir());
    }

    #[test]
    fn test_create_temp_file() {
        let (_temp, path) = create_temp_file("test.txt", "test content");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_create_temp_file_structure() {
        let temp = create_temp_file_structure(&[
            ("file1.txt", "content 1"),
            ("dir/file2.txt", "content 2"),
            ("dir/subdir/file3.txt", "content 3"),
        ]);

        assert!(temp.path().join("file1.txt").exists());
        assert!(temp.path().join("dir/file2.txt").exists());
        assert!(temp.path().join("dir/subdir/file3.txt").exists());

        let content = fs::read_to_string(temp.path().join("dir/subdir/file3.txt")).unwrap();
        assert_eq!(content, "content 3");
    }

    #[test]
    fn test_assert_approx_eq() {
        assert_approx_eq(1.0, 1.0001, 0.001);
        assert_approx_eq(1.0, 1.0, 0.0);
    }

    #[test]
    #[should_panic(expected = "not approximately equal")]
    fn test_assert_approx_eq_fails() {
        assert_approx_eq(1.0, 2.0, 0.1);
    }

    #[test]
    fn test_assert_contains() {
        assert_contains("hello world", "world");
        assert_contains("hello world", "hello");
        assert_contains("hello world", "lo wo");
    }

    #[test]
    #[should_panic(expected = "does not contain")]
    fn test_assert_contains_fails() {
        assert_contains("hello world", "foo");
    }

    #[test]
    fn test_assert_not_contains() {
        assert_not_contains("hello world", "foo");
        assert_not_contains("hello world", "bar");
    }

    #[test]
    #[should_panic(expected = "contains")]
    fn test_assert_not_contains_fails() {
        assert_not_contains("hello world", "world");
    }

    #[test]
    fn test_assert_path_exists() {
        let (_temp, path) = create_temp_file("test.txt", "test");
        assert_path_exists(&path);
    }

    #[test]
    #[should_panic(expected = "does not exist")]
    fn test_assert_path_exists_fails() {
        assert_path_exists("/nonexistent/path/to/file");
    }

    #[test]
    fn test_assert_path_not_exists() {
        assert_path_not_exists("/nonexistent/path/to/file");
    }

    #[test]
    #[should_panic(expected = "exists")]
    fn test_assert_path_not_exists_fails() {
        let (_temp, path) = create_temp_file("test.txt", "test");
        assert_path_not_exists(&path);
    }

    #[test]
    fn test_create_test_config() {
        let config = create_test_config();
        assert_eq!(config["model"], "test-model");
        assert_eq!(config["temperature"], 0.7);
        assert_eq!(config["max_tokens"], 100);
    }
}
