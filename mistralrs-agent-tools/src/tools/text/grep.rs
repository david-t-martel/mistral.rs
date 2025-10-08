//! grep - search for patterns in files
//!
//! Windows-optimized implementation with sandbox support.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult, GrepMatch, GrepOptions};
use regex::Regex;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Search for pattern in files
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `pattern` - Pattern to search for (regex or fixed string)
/// * `paths` - Paths to search in
/// * `options` - Grep options
///
/// # Returns
/// * Vector of matches or formatted string if count/files-only mode
pub fn grep(
    sandbox: &Sandbox,
    pattern: &str,
    paths: &[&Path],
    options: &GrepOptions,
) -> AgentResult<Vec<GrepMatch>> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    if pattern.is_empty() {
        return Err(AgentError::InvalidInput("Empty pattern".to_string()));
    }

    // Build regex or fixed string matcher
    let matcher = if options.fixed_strings {
        // Escape regex special characters for literal matching
        let escaped = regex::escape(pattern);
        if options.ignore_case {
            Regex::new(&format!("(?i){}", escaped))
        } else {
            Regex::new(&escaped)
        }
    } else if options.ignore_case {
        Regex::new(&format!("(?i){}", pattern))
    } else {
        Regex::new(pattern)
    }
    .map_err(|e| AgentError::InvalidInput(format!("Invalid regex pattern: {}", e)))?;

    let mut all_matches = Vec::new();

    for path in paths {
        // Validate path with sandbox
        sandbox.validate_read(path)?;

        if path.is_dir() {
            if options.recursive {
                search_directory(sandbox, &matcher, path, options, &mut all_matches)?;
            } else {
                return Err(AgentError::InvalidInput(format!(
                    "{} is a directory (use recursive option)",
                    path.display()
                )));
            }
        } else {
            search_file(sandbox, &matcher, path, options, &mut all_matches)?;
        }
    }

    Ok(all_matches)
}

/// Search a single file
fn search_file(
    _sandbox: &Sandbox,
    matcher: &Regex,
    path: &Path,
    options: &GrepOptions,
    matches: &mut Vec<GrepMatch>,
) -> AgentResult<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let path_str = path.display().to_string();

    let mut before_buffer: VecDeque<String> = VecDeque::new();
    let mut after_counter = 0;
    let mut pending_match: Option<GrepMatch> = None;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        let line_num = line_num + 1; // Convert to 1-based

        let is_match = matcher.is_match(&line) != options.invert_match;

        if is_match {
            // If we have a pending match with after-context, finalize it
            if let Some(prev_match) = pending_match.take() {
                matches.push(prev_match);
            }

            // Create new match
            let grep_match = GrepMatch {
                path: path_str.clone(),
                line_number: line_num,
                line: line.clone(),
                before: before_buffer.iter().cloned().collect(),
                after: Vec::new(),
            };

            if options.after_context > 0 {
                // Store as pending to collect after-context
                pending_match = Some(grep_match);
                after_counter = options.after_context;
            } else {
                matches.push(grep_match);
            }

            // Reset before buffer
            before_buffer.clear();
        } else {
            // Not a match - handle context
            if after_counter > 0 {
                // Add to after-context of pending match
                if let Some(ref mut pm) = pending_match {
                    pm.after.push(line.clone());
                }
                after_counter -= 1;

                if after_counter == 0 {
                    // Finalize pending match
                    if let Some(pm) = pending_match.take() {
                        matches.push(pm);
                    }
                }
            }

            // Add to before-context buffer
            if options.before_context > 0 {
                before_buffer.push_back(line.clone());
                if before_buffer.len() > options.before_context {
                    before_buffer.pop_front();
                }
            }
        }
    }

    // Finalize any remaining pending match
    if let Some(pm) = pending_match {
        matches.push(pm);
    }

    Ok(())
}

/// Search directory recursively
fn search_directory(
    sandbox: &Sandbox,
    matcher: &Regex,
    path: &Path,
    options: &GrepOptions,
    matches: &mut Vec<GrepMatch>,
) -> AgentResult<()> {
    let entries = std::fs::read_dir(path)?;

    for entry_result in entries {
        let entry = entry_result?;
        let entry_path = entry.path();

        // Validate each entry
        if sandbox.validate_read(&entry_path).is_err() {
            continue; // Skip inaccessible paths
        }

        if entry_path.is_dir() {
            search_directory(sandbox, matcher, &entry_path, options, matches)?;
        } else if entry_path.is_file() {
            search_file(sandbox, matcher, &entry_path, options, matches)?;
        }
    }

    Ok(())
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
    fn test_grep_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "hello world\nfoo bar\nhello again\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions::default();

        let matches = grep(&sandbox, "hello", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[0].line, "hello world");
        assert_eq!(matches[1].line_number, 3);
    }

    #[test]
    fn test_grep_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "Hello world\nFOO bar\nhello again\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions {
            ignore_case: true,
            ..Default::default()
        };

        let matches = grep(&sandbox, "HELLO", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_grep_line_numbers() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "line1\nline2\nline3\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions {
            line_number: true,
            ..Default::default()
        };

        let matches = grep(&sandbox, "line", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 2);
        assert_eq!(matches[2].line_number, 3);
    }

    #[test]
    fn test_grep_context() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(
            temp_dir.path(),
            "test.txt",
            "before1\nbefore2\nmatch\nafter1\nafter2\n",
        );

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions {
            before_context: 2,
            after_context: 2,
            ..Default::default()
        };

        let matches = grep(&sandbox, "match", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].before.len(), 2);
        assert_eq!(matches[0].after.len(), 2);
        assert_eq!(matches[0].before[0], "before1");
        assert_eq!(matches[0].after[0], "after1");
    }

    #[test]
    fn test_grep_invert_match() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "keep\nremove\nkeep\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions {
            invert_match: true,
            ..Default::default()
        };

        let matches = grep(&sandbox, "remove", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, "keep");
    }

    #[test]
    fn test_grep_fixed_strings() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "test.*\ntest.foo\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions {
            fixed_strings: true,
            ..Default::default()
        };

        // Should match literal ".*" not regex
        let matches = grep(&sandbox, ".*", &[&file], &options).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, "test.*");
    }

    #[test]
    fn test_grep_sandbox_violation() {
        let temp_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();
        let file = create_test_file(outside_dir.path(), "test.txt", "content");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = GrepOptions::default();

        let result = grep(&sandbox, "test", &[&file], &options);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::SandboxViolation(_)
        ));
    }
}
