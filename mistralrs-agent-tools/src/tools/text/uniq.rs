//! uniq - filter adjacent duplicate lines
//!
//! Windows-optimized implementation with sandbox support.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult, UniqOptions};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Filter adjacent duplicate lines
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Paths to read from
/// * `options` - Uniq options
///
/// # Returns
/// * Filtered lines as a single string
pub fn uniq(sandbox: &Sandbox, paths: &[&Path], options: &UniqOptions) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput(
            "No paths provided (stdin not supported)".to_string(),
        ));
    }

    let mut result = Vec::new();

    for path in paths {
        sandbox.validate_read(path)?;

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let filtered = filter_lines(reader.lines(), options)?;
        result.extend(filtered);
    }

    Ok(result.join("\n"))
}

/// Filter lines according to options
fn filter_lines<I>(lines: I, options: &UniqOptions) -> AgentResult<Vec<String>>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    let mut result = Vec::new();
    let mut prev_line: Option<String> = None;
    let mut count = 0usize;

    for line_result in lines {
        let line = line_result?;

        // Prepare line for comparison
        let compare_line = prepare_line(&line, options);

        match prev_line {
            None => {
                // First line
                prev_line = Some(line.clone());
                count = 1;
            }
            Some(ref prev) => {
                let compare_prev = prepare_line(prev, options);

                if compare_line == compare_prev {
                    // Duplicate line
                    count += 1;
                } else {
                    // Different line - output previous
                    output_line(&mut result, prev, count, options);

                    prev_line = Some(line.clone());
                    count = 1;
                }
            }
        }
    }

    // Output last line
    if let Some(prev) = prev_line {
        output_line(&mut result, &prev, count, options);
    }

    Ok(result)
}

/// Prepare line for comparison according to options
fn prepare_line(line: &str, options: &UniqOptions) -> String {
    let mut result = line.to_string();

    // Skip fields
    if options.skip_fields > 0 {
        let fields: Vec<&str> = result.split_whitespace().collect();
        if options.skip_fields < fields.len() {
            result = fields[options.skip_fields..].join(" ");
        } else {
            result = String::new();
        }
    }

    // Skip characters
    if options.skip_chars > 0 && result.len() > options.skip_chars {
        result = result[options.skip_chars..].to_string();
    }

    // Case insensitive
    if options.ignore_case {
        result = result.to_lowercase();
    }

    result
}

/// Output line according to options
fn output_line(result: &mut Vec<String>, line: &str, count: usize, options: &UniqOptions) {
    // Filter by count
    if options.repeated && count <= 1 {
        return; // Skip unique lines
    }

    if options.unique && count > 1 {
        return; // Skip duplicate lines
    }

    // Format output
    if options.count {
        result.push(format!("{:>7} {}", count, line));
    } else {
        result.push(line.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SandboxConfig;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_uniq_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "apple\napple\nbanana\nbanana\nbanana\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions::default();

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "apple\nbanana");
    }

    #[test]
    fn test_uniq_count() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "apple\napple\nbanana\nbanana\nbanana\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            count: true,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "      2 apple\n      3 banana");
    }

    #[test]
    fn test_uniq_repeated() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "apple\napple\nbanana\ncherry\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            repeated: true,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // Only apple is repeated
        assert_eq!(result, "apple");
    }

    #[test]
    fn test_uniq_unique() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "apple\napple\nbanana\ncherry\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            unique: true,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // Only banana and cherry are unique
        assert_eq!(result, "banana\ncherry");
    }

    #[test]
    fn test_uniq_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "Apple\napple\nAPPLE\nbanana\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            ignore_case: true,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // All variations of "apple" should be treated as one
        assert_eq!(result, "Apple\nbanana");
    }

    #[test]
    fn test_uniq_skip_fields() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "1 apple\n2 apple\n3 banana\n4 banana\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            skip_fields: 1,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // Should ignore first field (numbers) and compare only fruit names
        assert_eq!(result, "1 apple\n3 banana");
    }

    #[test]
    fn test_uniq_skip_chars() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "XXapple\nYYapple\nZZbanana\nWWbanana\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions {
            skip_chars: 2,
            ..Default::default()
        };

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // Should skip first 2 chars and compare the rest
        assert_eq!(result, "XXapple\nZZbanana");
    }

    #[test]
    fn test_uniq_no_adjacent_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "apple\nbanana\napple\nbanana\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions::default();

        let result = uniq(&sandbox, &[&file], &options).unwrap();

        // uniq only removes *adjacent* duplicates
        assert_eq!(result, "apple\nbanana\napple\nbanana");
    }

    #[test]
    fn test_uniq_sandbox_violation() {
        let temp_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();
        let file = create_test_file(outside_dir.path(), "test.txt", "content");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = UniqOptions::default();

        let result = uniq(&sandbox, &[&file], &options);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::SandboxViolation(_)
        ));
    }
}
