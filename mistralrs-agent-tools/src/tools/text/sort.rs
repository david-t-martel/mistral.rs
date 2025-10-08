//! sort - sort lines of text files
//!
//! Windows-optimized implementation with sandbox support.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult, SortOptions};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Sort lines from files
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Paths to read from (empty for stdin)
/// * `options` - Sort options
///
/// # Returns
/// * Sorted lines as a single string
pub fn sort(sandbox: &Sandbox, paths: &[&Path], options: &SortOptions) -> AgentResult<String> {
    let mut lines = Vec::new();

    if paths.is_empty() {
        return Err(AgentError::InvalidInput(
            "No paths provided (stdin not supported)".to_string(),
        ));
    }

    // Read all lines from all files
    for path in paths {
        sandbox.validate_read(path)?;

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for line_result in reader.lines() {
            let line = line_result?;
            lines.push(line);
        }
    }

    // Sort lines
    sort_lines(&mut lines, options);

    // Remove duplicates if unique flag is set
    if options.unique {
        lines.dedup();
    }

    Ok(lines.join("\n"))
}

/// Sort lines according to options
fn sort_lines(lines: &mut [String], options: &SortOptions) {
    if options.numeric {
        // Numeric sort
        lines.sort_by(|a, b| compare_numeric(a, b, options));
    } else if options.version_sort {
        // Version sort (natural sort)
        lines.sort_by(|a, b| compare_version(a, b, options));
    } else if options.month_sort {
        // Month sort
        lines.sort_by(|a, b| compare_month(a, b, options));
    } else if options.human_numeric {
        // Human numeric sort (1K, 1M, 1G)
        lines.sort_by(|a, b| compare_human_numeric(a, b, options));
    } else {
        // Lexical sort
        if options.ignore_case {
            lines.sort_by(|a, b| {
                let cmp = a.to_lowercase().cmp(&b.to_lowercase());
                if options.reverse {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        } else {
            lines.sort();
            if options.reverse {
                lines.reverse();
            }
        }
        return;
    }

    // Apply reverse if needed (for non-lexical sorts)
    if options.reverse && !options.numeric && !options.version_sort && !options.human_numeric {
        lines.reverse();
    }
}

/// Compare numerically
fn compare_numeric(a: &str, b: &str, options: &SortOptions) -> Ordering {
    let a_num = a.trim().parse::<f64>().ok();
    let b_num = b.trim().parse::<f64>().ok();

    let cmp = match (a_num, b_num) {
        (Some(an), Some(bn)) => an.partial_cmp(&bn).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    };

    if options.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

/// Compare version strings (natural sort)
fn compare_version(a: &str, b: &str, options: &SortOptions) -> Ordering {
    let a_parts = split_version(a);
    let b_parts = split_version(b);

    let cmp = compare_version_parts(&a_parts, &b_parts);

    if options.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

/// Split version string into comparable parts
fn split_version(s: &str) -> Vec<VersionPart> {
    let mut parts = Vec::new();
    let mut current_num = String::new();
    let mut current_text = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !current_text.is_empty() {
                parts.push(VersionPart::Text(current_text.clone()));
                current_text.clear();
            }
            current_num.push(ch);
        } else {
            if !current_num.is_empty() {
                if let Ok(num) = current_num.parse::<u64>() {
                    parts.push(VersionPart::Number(num));
                }
                current_num.clear();
            }
            current_text.push(ch);
        }
    }

    if !current_num.is_empty() {
        if let Ok(num) = current_num.parse::<u64>() {
            parts.push(VersionPart::Number(num));
        }
    }
    if !current_text.is_empty() {
        parts.push(VersionPart::Text(current_text));
    }

    parts
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionPart {
    Number(u64),
    Text(String),
}

fn compare_version_parts(a: &[VersionPart], b: &[VersionPart]) -> Ordering {
    for (ap, bp) in a.iter().zip(b.iter()) {
        match (ap, bp) {
            (VersionPart::Number(an), VersionPart::Number(bn)) => {
                let cmp = an.cmp(bn);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            (VersionPart::Text(at), VersionPart::Text(bt)) => {
                let cmp = at.cmp(bt);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            (VersionPart::Number(_), VersionPart::Text(_)) => return Ordering::Less,
            (VersionPart::Text(_), VersionPart::Number(_)) => return Ordering::Greater,
        }
    }

    a.len().cmp(&b.len())
}

/// Compare month names
fn compare_month(a: &str, b: &str, options: &SortOptions) -> Ordering {
    let a_month = parse_month(a.trim());
    let b_month = parse_month(b.trim());

    let cmp = match (a_month, b_month) {
        (Some(am), Some(bm)) => am.cmp(&bm),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    };

    if options.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

/// Parse month name to number (1-12)
fn parse_month(s: &str) -> Option<u32> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

/// Compare human-readable numbers (1K, 1M, 1G)
fn compare_human_numeric(a: &str, b: &str, options: &SortOptions) -> Ordering {
    let a_val = parse_human_numeric(a.trim());
    let b_val = parse_human_numeric(b.trim());

    let cmp = match (a_val, b_val) {
        (Some(av), Some(bv)) => av.partial_cmp(&bv).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    };

    if options.reverse {
        cmp.reverse()
    } else {
        cmp
    }
}

/// Parse human-readable number (1K, 1M, 1G)
fn parse_human_numeric(s: &str) -> Option<f64> {
    let s = s.trim().to_uppercase();
    if s.is_empty() {
        return None;
    }

    let last_char = s.chars().last()?;
    let multiplier = match last_char {
        'K' => 1024.0,
        'M' => 1024.0 * 1024.0,
        'G' => 1024.0 * 1024.0 * 1024.0,
        'T' => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    let num_str = if multiplier != 1.0 {
        &s[..s.len() - 1]
    } else {
        &s
    };

    num_str.parse::<f64>().ok().map(|n| n * multiplier)
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
    fn test_sort_lexical() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "zebra\napple\nbanana\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions::default();

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "apple\nbanana\nzebra");
    }

    #[test]
    fn test_sort_reverse() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "apple\nbanana\nzebra\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            reverse: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "zebra\nbanana\napple");
    }

    #[test]
    fn test_sort_numeric() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "100\n2\n30\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            numeric: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "2\n30\n100");
    }

    #[test]
    fn test_sort_unique() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "apple\napple\nbanana\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            unique: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "apple\nbanana");
    }

    #[test]
    fn test_sort_version() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "v1.10\nv1.2\nv1.9\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            version_sort: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "v1.2\nv1.9\nv1.10");
    }

    #[test]
    fn test_sort_human_numeric() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "1G\n500M\n2K\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            human_numeric: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "2K\n500M\n1G");
    }

    #[test]
    fn test_sort_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "Zebra\napple\nBanana\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);
        let options = SortOptions {
            ignore_case: true,
            ..Default::default()
        };

        let result = sort(&sandbox, &[&file], &options).unwrap();

        assert_eq!(result, "apple\nBanana\nZebra");
    }
}
