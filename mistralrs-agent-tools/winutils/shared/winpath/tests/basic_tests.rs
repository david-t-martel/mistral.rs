//! Basic integration tests for the winpath library.

use winpath::{normalize_path, normalize_path_cow, detect_path_format, PathFormat};

#[test]
fn test_dos_path_normalization() {
    // Already normalized DOS paths should remain unchanged
    let input = "C:\\Users\\David";
    let result = normalize_path(input).unwrap();
    assert_eq!(result, "C:\\Users\\David");
}

#[test]
fn test_dos_forward_slash_normalization() {
    let input = "C:/Users/David";
    let result = normalize_path(input).unwrap();
    assert_eq!(result, "C:\\Users\\David");
}

#[test]
fn test_wsl_path_normalization() {
    let input = "/mnt/c/users/david";
    let result = normalize_path(input).unwrap();
    assert_eq!(result, "C:\\users\\david");
}

#[test]
fn test_cygwin_path_normalization() {
    let input = "/cygdrive/c/users/david";
    let result = normalize_path(input).unwrap();
    assert_eq!(result, "C:\\users\\david");
}

#[test]
fn test_path_format_detection() {
    assert_eq!(detect_path_format("C:\\Users"), PathFormat::Dos);
    assert_eq!(detect_path_format("C:/Users"), PathFormat::DosForward);
    assert_eq!(detect_path_format("/mnt/c/users"), PathFormat::Wsl);
    assert_eq!(detect_path_format("/cygdrive/c/users"), PathFormat::Cygwin);
}

#[test]
fn test_zero_copy_optimization() {
    // Already normalized path should not be modified
    let input = "C:\\Users\\David";
    let result = normalize_path_cow(input).unwrap();
    assert!(!result.was_modified());
    assert_eq!(result.path(), input);

    // Path that needs normalization should be modified
    let input = "/mnt/c/users/david";
    let result = normalize_path_cow(input).unwrap();
    assert!(result.was_modified());
    assert_eq!(result.path(), "C:\\users\\david");
}

#[test]
fn test_error_handling() {
    // Empty path should fail
    assert!(normalize_path("").is_err());

    // Invalid WSL path should fail
    assert!(normalize_path("/mnt/").is_err());
}

#[test]
fn test_mixed_separators() {
    let input = "C:\\Users/David\\Documents";
    let result = normalize_path(input).unwrap();
    assert_eq!(result, "C:\\Users\\David\\Documents");
}

#[test]
fn test_long_path_handling() {
    // Create a path longer than 260 characters
    let long_component = "very_long_directory_name_that_exceeds_normal_limits".repeat(3);
    let long_path = format!("/mnt/c/{}", long_component);

    let result = normalize_path_cow(&long_path).unwrap();
    assert!(result.has_long_path_prefix());
    assert!(result.path().starts_with("\\\\?\\C:\\"));
}
