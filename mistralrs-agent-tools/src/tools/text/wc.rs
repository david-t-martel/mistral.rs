//! Wc utility - count words, lines, bytes, and characters
//!
//! Provides word, line, byte, and character counts for files.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentResult, WcOptions, WcResult};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Count words, lines, bytes, and/or characters in files
pub fn wc(
    sandbox: &Sandbox,
    paths: &[&Path],
    options: &WcOptions,
) -> AgentResult<Vec<(String, WcResult)>> {
    let mut results = Vec::new();
    let mut total = WcResult {
        lines: 0,
        words: 0,
        bytes: 0,
        chars: 0,
    };

    for path in paths {
        // Validate path through sandbox
        let validated_path = sandbox.validate_read(path)?;

        // Check file size
        sandbox.validate_file_size(&validated_path)?;

        // Count
        let result = count_file(&validated_path, options)?;

        // Add to total
        total.lines += result.lines;
        total.words += result.words;
        total.bytes += result.bytes;
        total.chars += result.chars;

        results.push((path.display().to_string(), result));
    }

    // Add total if multiple files
    if paths.len() > 1 {
        results.push(("total".to_string(), total));
    }

    Ok(results)
}

/// Count statistics for a single file
fn count_file(path: &Path, options: &WcOptions) -> AgentResult<WcResult> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut lines = 0;
    let mut words = 0;
    let mut bytes = 0;
    let mut chars = 0;

    // Determine what to count (if none specified, count all)
    let count_all = !options.lines && !options.words && !options.bytes && !options.chars;
    let need_lines = count_all || options.lines;
    let need_words = count_all || options.words;
    let need_bytes = count_all || options.bytes;
    let need_chars = count_all || options.chars;

    if need_bytes {
        // If we only need bytes, just get file size
        if !need_lines && !need_words && !need_chars {
            bytes = std::fs::metadata(path)?.len() as usize;
            return Ok(WcResult {
                lines,
                words,
                bytes,
                chars,
            });
        }
    }

    // Read file line by line
    let mut buffer = String::new();
    loop {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer)?;

        if bytes_read == 0 {
            break; // EOF
        }

        if need_lines {
            lines += 1;
        }

        if need_bytes {
            bytes += bytes_read;
        }

        if need_chars {
            chars += buffer.chars().count();
        }

        if need_words {
            // Count words (whitespace-separated)
            words += buffer.split_whitespace().count();
        }
    }

    Ok(WcResult {
        lines,
        words,
        bytes,
        chars,
    })
}

/// Format wc output
pub fn format_wc_output(results: &[(String, WcResult)], options: &WcOptions) -> String {
    let count_all = !options.lines && !options.words && !options.bytes && !options.chars;
    let mut output = String::new();

    for (path, result) in results {
        let mut parts = Vec::new();

        if count_all || options.lines {
            parts.push(format!("{:>8}", result.lines));
        }
        if count_all || options.words {
            parts.push(format!("{:>8}", result.words));
        }
        if count_all || options.chars {
            parts.push(format!("{:>8}", result.chars));
        }
        if options.bytes && !count_all {
            parts.push(format!("{:>8}", result.bytes));
        }
        if count_all {
            parts.push(format!("{:>8}", result.bytes));
        }

        output.push_str(&parts.join(" "));
        output.push(' ');
        output.push_str(path);
        output.push('\n');
    }

    output
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
    fn test_wc_basic() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello World").unwrap();
        writeln!(file, "Test line").unwrap();

        let options = WcOptions::default();
        let results = wc(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(results.len(), 1);
        let (_, result) = &results[0];
        assert_eq!(result.lines, 2);
        assert_eq!(result.words, 4); // "Hello", "World" | "Test", "line"
    }

    #[test]
    fn test_wc_lines_only() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let options = WcOptions {
            lines: true,
            ..Default::default()
        };
        let results = wc(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.lines, 10);
    }

    #[test]
    fn test_wc_words_only() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "one two three four five").unwrap();

        let options = WcOptions {
            words: true,
            ..Default::default()
        };
        let results = wc(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(results[0].1.words, 5);
    }

    #[test]
    fn test_wc_bytes() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "Hello").unwrap(); // 5 bytes

        let options = WcOptions {
            bytes: true,
            ..Default::default()
        };
        let results = wc(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(results[0].1.bytes, 5);
    }

    #[test]
    fn test_wc_multiple_files() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        File::create(&file1)
            .unwrap()
            .write_all(b"Line 1\nLine 2\n")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"Line A\n")
            .unwrap();

        let options = WcOptions::default();
        let results = wc(&sandbox, &[&file1, &file2], &options).unwrap();

        // Should have 3 results: file1, file2, and total
        assert_eq!(results.len(), 3);
        assert_eq!(results[2].0, "total");
        assert_eq!(results[2].1.lines, 3); // 2 + 1
    }

    #[test]
    fn test_wc_empty_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let options = WcOptions::default();
        let results = wc(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(results[0].1.lines, 0);
        assert_eq!(results[0].1.words, 0);
        assert_eq!(results[0].1.bytes, 0);
    }

    #[test]
    fn test_format_wc_output() {
        let results = vec![
            (
                "file1.txt".to_string(),
                WcResult {
                    lines: 10,
                    words: 50,
                    bytes: 200,
                    chars: 200,
                },
            ),
            (
                "file2.txt".to_string(),
                WcResult {
                    lines: 5,
                    words: 25,
                    bytes: 100,
                    chars: 100,
                },
            ),
        ];

        let options = WcOptions::default();
        let output = format_wc_output(&results, &options);

        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
        assert!(output.contains("10"));
        assert!(output.contains("50"));
    }
}
