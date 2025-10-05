//! Head utility - display first part of files
//!
//! Shows the first N lines (or bytes) of one or more files.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentResult, HeadOptions};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Display the first part of files
pub fn head(sandbox: &Sandbox, paths: &[&Path], options: &HeadOptions) -> AgentResult<String> {
    let mut output = String::new();
    let show_headers = !options.quiet && (options.verbose || paths.len() > 1);

    for (idx, path) in paths.iter().enumerate() {
        // Validate path through sandbox
        let validated_path = sandbox.validate_read(path)?;

        // Check file size
        sandbox.validate_file_size(&validated_path)?;

        // Add header if needed
        if show_headers {
            if idx > 0 {
                output.push('\n');
            }
            output.push_str(&format!("==> {} <==\n", path.display()));
        }

        // Read and output
        if let Some(byte_count) = options.bytes {
            output.push_str(&read_bytes(&validated_path, byte_count)?);
        } else {
            output.push_str(&read_lines(&validated_path, options.lines)?);
        }

        // Add newline between files
        if idx < paths.len() - 1 && show_headers {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Read first N lines from a file
fn read_lines(path: &Path, n: usize) -> AgentResult<String> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = String::new();

    for (count, line) in reader.lines().enumerate() {
        if count >= n {
            break;
        }
        let line = line?;
        result.push_str(&line);
        result.push('\n');
    }

    Ok(result)
}

/// Read first N bytes from a file
fn read_bytes(path: &Path, n: usize) -> AgentResult<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; n];
    let bytes_read = reader.read(&mut buffer)?;

    buffer.truncate(bytes_read);

    // Try to convert to string (may have incomplete UTF-8 at boundary)
    match String::from_utf8(buffer) {
        Ok(s) => Ok(s),
        Err(e) => {
            // Handle incomplete UTF-8 at boundary - take valid portion
            let valid_len = e.utf8_error().valid_up_to();
            let valid_bytes = e.into_bytes();
            Ok(String::from_utf8_lossy(&valid_bytes[..valid_len]).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SandboxConfig;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_sandbox() -> (Sandbox, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));
        (sandbox, temp_dir)
    }

    #[test]
    fn test_head_default() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write 20 lines
        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = HeadOptions::default(); // 10 lines
        let result = head(&sandbox, &[&file_path], &options).unwrap();

        // Should have 10 lines
        assert_eq!(result.lines().count(), 10);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 10"));
        assert!(!result.contains("Line 11"));
    }

    #[test]
    fn test_head_custom_lines() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = HeadOptions {
            lines: 5,
            ..Default::default()
        };
        let result = head(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result.lines().count(), 5);
        assert!(result.contains("Line 5"));
        assert!(!result.contains("Line 6"));
    }

    #[test]
    fn test_head_bytes() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "Hello, World! This is a test.").unwrap();

        let options = HeadOptions {
            bytes: Some(5),
            ..Default::default()
        };
        let result = head(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_head_multiple_files() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        let mut f1 = File::create(&file1).unwrap();
        writeln!(f1, "File 1 Line 1").unwrap();
        writeln!(f1, "File 1 Line 2").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        writeln!(f2, "File 2 Line 1").unwrap();
        writeln!(f2, "File 2 Line 2").unwrap();

        let options = HeadOptions::default();
        let result = head(&sandbox, &[&file1, &file2], &options).unwrap();

        // Should have headers for both files
        assert!(result.contains("==> "));
        assert!(result.contains("file1.txt"));
        assert!(result.contains("file2.txt"));
        assert!(result.contains("File 1 Line 1"));
        assert!(result.contains("File 2 Line 1"));
    }

    #[test]
    fn test_head_quiet_mode() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        File::create(&file1)
            .unwrap()
            .write_all(b"Line 1\n")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"Line 2\n")
            .unwrap();

        let options = HeadOptions {
            quiet: true,
            ..Default::default()
        };
        let result = head(&sandbox, &[&file1, &file2], &options).unwrap();

        // Should not have headers
        assert!(!result.contains("==>"));
    }

    #[test]
    fn test_head_empty_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let options = HeadOptions::default();
        let result = head(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result, "");
    }
}
