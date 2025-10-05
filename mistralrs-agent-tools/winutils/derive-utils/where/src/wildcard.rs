// Copyright (c) 2024 uutils developers
// SPDX-License-Identifier: MIT OR Apache-2.0

//! High-performance wildcard pattern matching for Windows file names

use crate::error::{WhereError, WhereResult};
use glob::{Pattern, PatternError};
use regex::Regex;
use std::path::Path;

/// Wildcard pattern matcher optimized for Windows filename matching
#[derive(Debug, Clone)]
pub struct WildcardMatcher {
    /// The original pattern string
    pattern: String,
    /// Compiled glob pattern
    glob_pattern: Pattern,
    /// Compiled regex pattern for complex cases
    regex_pattern: Option<Regex>,
    /// Whether the pattern is case-sensitive
    case_sensitive: bool,
}

impl WildcardMatcher {
    /// Create a new wildcard matcher from a pattern
    pub fn new(pattern: &str) -> WhereResult<Self> {
        let case_sensitive = false; // Windows is case-insensitive

        // Normalize pattern for case-insensitive matching
        let normalized_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        // Create glob pattern
        let glob_pattern = Pattern::new(&normalized_pattern)
            .map_err(WhereError::Pattern)?;

        // Create regex pattern for complex patterns
        let regex_pattern = if pattern.contains('[') || pattern.contains('{') {
            Some(Self::create_regex_pattern(&normalized_pattern)?)
        } else {
            None
        };

        Ok(Self {
            pattern: normalized_pattern,
            glob_pattern,
            regex_pattern,
            case_sensitive,
        })
    }

    /// Check if a filename matches this pattern
    pub fn matches(&self, filename: &str) -> bool {
        let test_name = if self.case_sensitive {
            filename.to_string()
        } else {
            filename.to_lowercase()
        };

        // Try glob pattern first (faster for simple patterns)
        if self.glob_pattern.matches(&test_name) {
            return true;
        }

        // Try regex pattern for complex cases
        if let Some(ref regex) = self.regex_pattern {
            if regex.is_match(&test_name) {
                return true;
            }
        }

        false
    }

    /// Check if a full path matches this pattern (filename only)
    pub fn matches_path(&self, path: &Path) -> bool {
        if let Some(filename) = path.file_name() {
            if let Some(filename_str) = filename.to_str() {
                return self.matches(filename_str);
            }
        }
        false
    }

    /// Get the original pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Check if this is a simple pattern (no wildcards)
    pub fn is_literal(&self) -> bool {
        !self.pattern.contains('*') &&
        !self.pattern.contains('?') &&
        !self.pattern.contains('[') &&
        !self.pattern.contains('{')
    }

    /// Create a regex pattern from a glob-style pattern
    fn create_regex_pattern(pattern: &str) -> WhereResult<Regex> {
        let mut regex_pattern = String::new();
        regex_pattern.push('^');

        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '*' => regex_pattern.push_str(".*"),
                '?' => regex_pattern.push('.'),
                '[' => {
                    // Handle character classes
                    regex_pattern.push('[');
                    i += 1;
                    while i < chars.len() && chars[i] != ']' {
                        if chars[i] == '\\' {
                            regex_pattern.push('\\');
                            regex_pattern.push('\\');
                        } else {
                            regex_pattern.push(chars[i]);
                        }
                        i += 1;
                    }
                    if i < chars.len() {
                        regex_pattern.push(']');
                    }
                }
                '{' => {
                    // Handle brace expansion {a,b,c}
                    regex_pattern.push('(');
                    i += 1;
                    while i < chars.len() && chars[i] != '}' {
                        if chars[i] == ',' {
                            regex_pattern.push('|');
                        } else if chars[i] == '\\' {
                            regex_pattern.push('\\');
                            regex_pattern.push('\\');
                        } else {
                            regex_pattern.push(chars[i]);
                        }
                        i += 1;
                    }
                    if i < chars.len() {
                        regex_pattern.push(')');
                    }
                }
                '\\' => {
                    regex_pattern.push('\\');
                    if i + 1 < chars.len() {
                        i += 1;
                        regex_pattern.push(chars[i]);
                    }
                }
                '.' | '^' | '$' | '(' | ')' | '|' | '+' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(chars[i]);
                }
                c => regex_pattern.push(c),
            }
            i += 1;
        }

        regex_pattern.push('$');

        Regex::new(&regex_pattern).map_err(WhereError::Regex)
    }
}

/// Multiple pattern matcher for efficient batch matching
#[derive(Debug)]
pub struct MultiPatternMatcher {
    matchers: Vec<WildcardMatcher>,
}

impl MultiPatternMatcher {
    /// Create a new multi-pattern matcher
    pub fn new(patterns: &[String]) -> WhereResult<Self> {
        let mut matchers = Vec::new();

        for pattern in patterns {
            matchers.push(WildcardMatcher::new(pattern)?);
        }

        Ok(Self { matchers })
    }

    /// Check if any pattern matches the given filename
    pub fn matches_any(&self, filename: &str) -> bool {
        self.matchers.iter().any(|matcher| matcher.matches(filename))
    }

    /// Check if any pattern matches the given path
    pub fn matches_any_path(&self, path: &Path) -> bool {
        self.matchers.iter().any(|matcher| matcher.matches_path(path))
    }

    /// Get all patterns that match the given filename
    pub fn matching_patterns(&self, filename: &str) -> Vec<&str> {
        self.matchers
            .iter()
            .filter(|matcher| matcher.matches(filename))
            .map(|matcher| matcher.pattern())
            .collect()
    }

    /// Check if all matchers are literal (no wildcards)
    pub fn all_literal(&self) -> bool {
        self.matchers.iter().all(|matcher| matcher.is_literal())
    }

    /// Get the number of patterns
    pub fn len(&self) -> usize {
        self.matchers.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.matchers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_simple_wildcard() {
        let matcher = WildcardMatcher::new("*.exe").unwrap();

        assert!(matcher.matches("test.exe"));
        assert!(matcher.matches("program.EXE"));
        assert!(!matcher.matches("test.bat"));
        assert!(!matcher.matches("exe"));
    }

    #[test]
    fn test_question_mark_wildcard() {
        let matcher = WildcardMatcher::new("test?.exe").unwrap();

        assert!(matcher.matches("test1.exe"));
        assert!(matcher.matches("testa.exe"));
        assert!(!matcher.matches("test.exe"));
        assert!(!matcher.matches("test12.exe"));
    }

    #[test]
    fn test_literal_pattern() {
        let matcher = WildcardMatcher::new("python.exe").unwrap();

        assert!(matcher.matches("python.exe"));
        assert!(matcher.matches("PYTHON.EXE"));
        assert!(!matcher.matches("python3.exe"));
        assert!(matcher.is_literal());
    }

    #[test]
    fn test_path_matching() {
        let matcher = WildcardMatcher::new("*.exe").unwrap();
        let path = PathBuf::from("C:\\Program Files\\test.exe");

        assert!(matcher.matches_path(&path));
    }

    #[test]
    fn test_multi_pattern_matcher() {
        let patterns = vec![
            "*.exe".to_string(),
            "python*".to_string(),
            "test.bat".to_string(),
        ];

        let matcher = MultiPatternMatcher::new(&patterns).unwrap();

        assert!(matcher.matches_any("test.exe"));
        assert!(matcher.matches_any("python3"));
        assert!(matcher.matches_any("test.bat"));
        assert!(!matcher.matches_any("readme.txt"));

        assert_eq!(matcher.len(), 3);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let matcher = WildcardMatcher::new("Test*.EXE").unwrap();

        assert!(matcher.matches("test123.exe"));
        assert!(matcher.matches("TEST_FILE.EXE"));
        assert!(matcher.matches("tEsT.exe"));
    }

    #[test]
    fn test_complex_patterns() {
        let matcher = WildcardMatcher::new("test[0-9]*.exe").unwrap();

        assert!(matcher.matches("test1.exe"));
        assert!(matcher.matches("test9program.exe"));
        assert!(!matcher.matches("testa.exe"));
    }

    #[test]
    fn test_brace_expansion() {
        let matcher = WildcardMatcher::new("*.{exe,bat,cmd}").unwrap();

        assert!(matcher.matches("test.exe"));
        assert!(matcher.matches("script.bat"));
        assert!(matcher.matches("command.cmd"));
        assert!(!matcher.matches("file.txt"));
    }
}
