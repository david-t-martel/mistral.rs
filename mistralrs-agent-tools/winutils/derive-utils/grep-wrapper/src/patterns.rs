//! Pattern building and compilation for grep wrapper

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::config::GrepConfig;

/// Build the final pattern from configuration
pub fn build_pattern(config: &GrepConfig) -> Result<String> {
    let mut patterns = vec![config.pattern.clone()];

    // Add patterns from files
    for pattern_file in &config.pattern_files {
        let file_patterns = read_patterns_from_file(pattern_file)?;
        patterns.extend(file_patterns);
    }

    // Handle multiple patterns
    if patterns.len() == 1 {
        Ok(patterns.into_iter().next().unwrap())
    } else {
        // Combine multiple patterns with alternation
        let combined = patterns.join("|");
        Ok(format!("({})", combined))
    }
}

/// Read patterns from a file
fn read_patterns_from_file(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read pattern file '{}': {}", path.display(), e))?;

    let patterns: Vec<String> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect();

    if patterns.is_empty() {
        return Err(anyhow!("No patterns found in file '{}'", path.display()));
    }

    Ok(patterns)
}

/// Pattern validator for different regex flavors
pub struct PatternValidator;

impl PatternValidator {
    /// Validate a basic regular expression pattern
    pub fn validate_basic_regex(pattern: &str) -> Result<()> {
        // Basic regex validation - convert basic regex metacharacters
        let converted = Self::convert_basic_to_extended(pattern)?;
        Regex::new(&converted)
            .map_err(|e| anyhow!("Invalid basic regex pattern '{}': {}", pattern, e))?;
        Ok(())
    }

    /// Validate an extended regular expression pattern
    pub fn validate_extended_regex(pattern: &str) -> Result<()> {
        Regex::new(pattern)
            .map_err(|e| anyhow!("Invalid extended regex pattern '{}': {}", pattern, e))?;
        Ok(())
    }

    /// Validate a fixed string pattern
    pub fn validate_fixed_string(pattern: &str) -> Result<()> {
        // Fixed strings are always valid
        if pattern.is_empty() {
            return Err(anyhow!("Empty pattern not allowed"));
        }
        Ok(())
    }

    /// Convert basic regex to extended regex
    fn convert_basic_to_extended(pattern: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            // Basic regex escaped metacharacters
                            '+' | '?' | '|' | '(' | ')' | '{' | '}' => {
                                chars.next(); // consume the next character
                                result.push(next_ch); // add it unescaped
                            }
                            // Keep other escapes as-is
                            _ => {
                                result.push(ch);
                            }
                        }
                    } else {
                        result.push(ch);
                    }
                }
                // Basic regex unescaped metacharacters that need escaping in extended
                '+' | '?' | '|' | '(' | ')' | '{' | '}' => {
                    result.push('\\');
                    result.push(ch);
                }
                // Regular characters
                _ => {
                    result.push(ch);
                }
            }
        }

        Ok(result)
    }
}

/// Pattern optimization utilities
pub struct PatternOptimizer;

impl PatternOptimizer {
    /// Optimize a pattern for better performance
    pub fn optimize(pattern: &str, config: &GrepConfig) -> String {
        let mut optimized = pattern.to_string();

        // Add word boundaries if requested
        if config.word_regexp && !config.fixed_strings {
            optimized = format!(r"\b{}\b", optimized);
        }

        // Add line anchors if requested
        if config.line_regexp && !config.fixed_strings {
            optimized = format!(r"^{}$", optimized);
        }

        // Case insensitive optimization
        if config.ignore_case && !config.fixed_strings {
            optimized = format!("(?i){}", optimized);
        }

        optimized
    }

    /// Check if a pattern can benefit from fixed-string optimization
    pub fn can_use_fixed_string(pattern: &str) -> bool {
        // Check if pattern contains regex metacharacters
        let metacharacters = ['\\', '^', '$', '.', '|', '?', '*', '+', '(', ')', '[', ']', '{', '}'];
        !pattern.chars().any(|c| metacharacters.contains(&c))
    }

    /// Extract literal prefix from a regex pattern
    pub fn extract_literal_prefix(pattern: &str) -> Option<String> {
        let mut prefix = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                // Regex metacharacters - stop here
                '\\' | '^' | '$' | '.' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | '{' => {
                    break;
                }
                // Regular character
                _ => {
                    prefix.push(ch);
                }
            }
        }

        if prefix.is_empty() || prefix.len() < 3 {
            None // Too short to be useful
        } else {
            Some(prefix)
        }
    }
}

/// Pattern statistics for optimization decisions
pub struct PatternStats {
    pub length: usize,
    pub has_anchors: bool,
    pub has_quantifiers: bool,
    pub has_character_classes: bool,
    pub has_groups: bool,
    pub estimated_complexity: u32,
}

impl PatternStats {
    /// Analyze a pattern and compute statistics
    pub fn analyze(pattern: &str) -> Self {
        let mut stats = Self {
            length: pattern.len(),
            has_anchors: false,
            has_quantifiers: false,
            has_character_classes: false,
            has_groups: false,
            estimated_complexity: 0,
        };

        let mut chars = pattern.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '^' | '$' => {
                    stats.has_anchors = true;
                    stats.estimated_complexity += 1;
                }
                '*' | '+' | '?' => {
                    stats.has_quantifiers = true;
                    stats.estimated_complexity += 3;
                }
                '[' => {
                    stats.has_character_classes = true;
                    stats.estimated_complexity += 2;
                    // Skip to end of character class
                    while let Some(ch) = chars.next() {
                        if ch == ']' {
                            break;
                        }
                    }
                }
                '(' => {
                    stats.has_groups = true;
                    stats.estimated_complexity += 2;
                }
                '\\' => {
                    // Skip escaped character
                    chars.next();
                    stats.estimated_complexity += 1;
                }
                '.' | '|' => {
                    stats.estimated_complexity += 2;
                }
                '{' => {
                    // Quantifier range
                    stats.has_quantifiers = true;
                    stats.estimated_complexity += 4;
                    // Skip to end of quantifier
                    while let Some(ch) = chars.next() {
                        if ch == '}' {
                            break;
                        }
                    }
                }
                _ => {
                    stats.estimated_complexity += 1;
                }
            }
        }

        stats
    }

    /// Check if this is a simple pattern
    pub fn is_simple(&self) -> bool {
        self.estimated_complexity < 10 &&
        !self.has_quantifiers &&
        !self.has_character_classes &&
        !self.has_groups
    }

    /// Check if this pattern would benefit from compilation caching
    pub fn should_cache(&self) -> bool {
        self.estimated_complexity > 15 || (self.has_quantifiers && self.length > 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_pattern_validator() {
        // Valid patterns
        assert!(PatternValidator::validate_fixed_string("hello").is_ok());
        assert!(PatternValidator::validate_extended_regex("hello.*world").is_ok());
        assert!(PatternValidator::validate_basic_regex("hello\\+world").is_ok());

        // Invalid patterns
        assert!(PatternValidator::validate_fixed_string("").is_err());
        assert!(PatternValidator::validate_extended_regex("[").is_err());
    }

    #[test]
    fn test_basic_to_extended_conversion() {
        let converted = PatternValidator::convert_basic_to_extended("hello\\+world").unwrap();
        assert_eq!(converted, "hello+world");

        let converted = PatternValidator::convert_basic_to_extended("test\\(group\\)").unwrap();
        assert_eq!(converted, "test(group)");

        let converted = PatternValidator::convert_basic_to_extended("literal+text").unwrap();
        assert_eq!(converted, "literal\\+text");
    }

    #[test]
    fn test_pattern_optimization() {
        let config = GrepConfig {
            word_regexp: true,
            ignore_case: true,
            fixed_strings: false,
            ..Default::default()
        };

        let optimized = PatternOptimizer::optimize("test", &config);
        assert!(optimized.contains(r"\b"));
        assert!(optimized.contains("(?i)"));
    }

    #[test]
    fn test_literal_prefix_extraction() {
        assert_eq!(
            PatternOptimizer::extract_literal_prefix("hello.*world"),
            Some("hello".to_string())
        );

        assert_eq!(
            PatternOptimizer::extract_literal_prefix("test"),
            Some("test".to_string())
        );

        assert_eq!(
            PatternOptimizer::extract_literal_prefix(".*pattern"),
            None
        );

        assert_eq!(
            PatternOptimizer::extract_literal_prefix("ab"),
            None // Too short
        );
    }

    #[test]
    fn test_can_use_fixed_string() {
        assert!(PatternOptimizer::can_use_fixed_string("hello world"));
        assert!(PatternOptimizer::can_use_fixed_string("test123"));
        assert!(!PatternOptimizer::can_use_fixed_string("hello.*world"));
        assert!(!PatternOptimizer::can_use_fixed_string("test+"));
    }

    #[test]
    fn test_pattern_stats() {
        let stats = PatternStats::analyze("hello.*world");
        assert!(stats.has_quantifiers);
        assert!(!stats.has_anchors);
        assert!(!stats.has_character_classes);

        let stats = PatternStats::analyze("^test$");
        assert!(stats.has_anchors);
        assert!(!stats.has_quantifiers);

        let stats = PatternStats::analyze("[a-z]+");
        assert!(stats.has_character_classes);
        assert!(stats.has_quantifiers);

        let stats = PatternStats::analyze("simple");
        assert!(stats.is_simple());
    }

    #[test]
    fn test_read_patterns_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "pattern1").unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "pattern2").unwrap();
        writeln!(temp_file, "").unwrap(); // Empty line
        writeln!(temp_file, "pattern3").unwrap();

        let patterns = read_patterns_from_file(temp_file.path()).unwrap();
        assert_eq!(patterns, vec!["pattern1", "pattern2", "pattern3"]);
    }

    #[test]
    fn test_build_pattern() {
        let config = GrepConfig {
            pattern: "main_pattern".to_string(),
            pattern_files: vec![],
            ..Default::default()
        };

        let pattern = build_pattern(&config).unwrap();
        assert_eq!(pattern, "main_pattern");
    }
}
