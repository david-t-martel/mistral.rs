//! Tail utility - display last part of files
//!
//! Shows the last N lines (or bytes) of one or more files.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentResult, TailOptions};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Display the last part of files
pub fn tail(sandbox: &Sandbox, paths: &[&Path], options: &TailOptions) -> AgentResult<String> {
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
            output.push_str(&read_last_bytes(&validated_path, byte_count)?);
        } else {
            output.push_str(&read_last_lines(&validated_path, options.lines)?);
        }

        // Add newline between files
        if idx < paths.len() - 1 && show_headers {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Read last N lines from a file using a sliding window
fn read_last_lines(path: &Path, n: usize) -> AgentResult<String> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = VecDeque::with_capacity(n);

    for line in reader.lines() {
        let line = line?;
        if lines.len() >= n {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    let mut result = String::new();
    for line in lines {
        result.push_str(&line);
        result.push('\n');
    }

    Ok(result)
}

/// Read last N bytes from a file (efficiently seeks from end)
fn read_last_bytes(path: &Path, n: usize) -> AgentResult<String> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len() as usize;

    let bytes_to_read = n.min(file_size);
    let start_pos = file_size - bytes_to_read;

    // Seek to position
    file.seek(SeekFrom::Start(start_pos as u64))?;

    // Read bytes
    let mut buffer = vec![0u8; bytes_to_read];
    file.read_exact(&mut buffer)?;

    // Try to convert to string (may have incomplete UTF-8 at boundary)
    match String::from_utf8(buffer) {
        Ok(s) => Ok(s),
        Err(e) => {
            // Handle incomplete UTF-8 at boundary - find first valid character
            let bytes = e.into_bytes();
            // Find first valid UTF-8 sequence
            for i in 0..bytes.len().min(4) {
                if let Ok(s) = String::from_utf8(bytes[i..].to_vec()) {
                    return Ok(s);
                }
            }
            // Fallback to lossy conversion
            Ok(String::from_utf8_lossy(&bytes).to_string())
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
    fn test_tail_default() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write 20 lines
        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = TailOptions::default(); // 10 lines
        let result = tail(&sandbox, &[&file_path], &options).unwrap();

        // Should have 10 lines
        assert_eq!(result.lines().count(), 10);
        assert!(result.contains("Line 11"));
        assert!(result.contains("Line 20"));
        assert!(!result.contains("Line 10"));
    }

    #[test]
    fn test_tail_custom_lines() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = TailOptions {
            lines: 5,
            ..Default::default()
        };
        let result = tail(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result.lines().count(), 5);
        assert!(result.contains("Line 16"));
        assert!(result.contains("Line 20"));
        assert!(!result.contains("Line 15"));
    }

    #[test]
    fn test_tail_bytes() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "Hello, World! This is a test.").unwrap();

        let options = TailOptions {
            bytes: Some(5),
            ..Default::default()
        };
        let result = tail(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result, "test.");
    }

    #[test]
    fn test_tail_multiple_files() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        let mut f1 = File::create(&file1).unwrap();
        writeln!(f1, "File 1 Line 1").unwrap();
        writeln!(f1, "File 1 Line 2").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        writeln!(f2, "File 2 Line 1").unwrap();
        writeln!(f2, "File 2 Line 2").unwrap();

        let options = TailOptions::default();
        let result = tail(&sandbox, &[&file1, &file2], &options).unwrap();

        // Should have headers for both files
        assert!(result.contains("==> "));
        assert!(result.contains("file1.txt"));
        assert!(result.contains("file2.txt"));
        assert!(result.contains("File 1 Line 2"));
        assert!(result.contains("File 2 Line 2"));
    }

    #[test]
    fn test_tail_fewer_lines_than_requested() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write only 3 lines
        for i in 1..=3 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = TailOptions {
            lines: 10, // Request more than available
            ..Default::default()
        };
        let result = tail(&sandbox, &[&file_path], &options).unwrap();

        // Should have all 3 lines
        assert_eq!(result.lines().count(), 3);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 3"));
    }

    #[test]
    fn test_tail_empty_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let options = TailOptions::default();
        let result = tail(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result, "");
    }
}
