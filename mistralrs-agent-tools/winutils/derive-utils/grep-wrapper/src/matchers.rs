//! Pattern matching implementations for grep wrapper

use anyhow::{anyhow, Result};
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;

/// Create a matcher based on configuration
pub fn create_matcher(pattern: &str, config: &crate::config::GrepConfig) -> Result<Box<dyn Matcher>> {
    let mut builder = grep_regex::RegexMatcherBuilder::new();

    // Configure case sensitivity
    if config.ignore_case {
        builder.case_insensitive(true);
    }

    // Configure pattern type
    if config.fixed_strings {
        builder.fixed_strings(true);
    }

    // Configure word/line boundaries
    if config.word_regexp {
        builder.word(true);
    }

    if config.line_regexp {
        builder.line_terminator(Some(b'\n'));
    }

    // Handle different regex flavors
    if config.basic_regexp {
        // Basic regular expressions (default)
        builder.syntax(grep_regex::RegexSyntax::default());
    } else if config.extended_regexp {
        // Extended regular expressions
        builder.syntax(
            grep_regex::RegexSyntax::default()
                .with_extended_regex(true)
        );
    } else if config.perl_regexp {
        // Perl-compatible regular expressions
        builder.syntax(
            grep_regex::RegexSyntax::default()
                .with_pcre2(true)
        );
    }

    // Configure multi-line handling
    if config.null_data {
        builder.line_terminator(Some(b'\0'));
    }

    // Build the pattern
    let effective_pattern = if config.word_regexp && !config.fixed_strings {
        format!(r"\b{}\b", pattern)
    } else if config.line_regexp && !config.fixed_strings {
        format!(r"^{}$", pattern)
    } else {
        pattern.to_string()
    };

    let matcher = builder.build(&effective_pattern)
        .map_err(|e| anyhow!("Failed to compile pattern '{}': {}", pattern, e))?;

    Ok(Box::new(matcher))
}

/// Multi-pattern matcher using Aho-Corasick for performance
pub struct MultiPatternMatcher {
    patterns: Vec<String>,
    aho_corasick: Option<aho_corasick::AhoCorasick>,
    ignore_case: bool,
}

impl MultiPatternMatcher {
    /// Create a new multi-pattern matcher
    pub fn new(patterns: Vec<String>, ignore_case: bool) -> Result<Self> {
        let aho_corasick = if patterns.len() > 1 {
            let mut builder = aho_corasick::AhoCorasickBuilder::new();
            builder.ascii_case_insensitive(ignore_case);

            Some(builder.build(&patterns)
                .map_err(|e| anyhow!("Failed to build Aho-Corasick automaton: {}", e))?)
        } else {
            None
        };

        Ok(Self {
            patterns,
            aho_corasick,
            ignore_case,
        })
    }

    /// Search for patterns in the given text
    pub fn find_matches(&self, text: &[u8]) -> Vec<aho_corasick::Match> {
        if let Some(ref ac) = self.aho_corasick {
            ac.find_iter(text).collect()
        } else if let Some(pattern) = self.patterns.first() {
            // Single pattern fallback
            self.find_single_pattern(text, pattern.as_bytes())
        } else {
            Vec::new()
        }
    }

    /// Find matches for a single pattern
    fn find_single_pattern(&self, text: &[u8], pattern: &[u8]) -> Vec<aho_corasick::Match> {
        let mut matches = Vec::new();

        if self.ignore_case {
            // Case-insensitive search
            let text_lower = text.to_ascii_lowercase();
            let pattern_lower = pattern.to_ascii_lowercase();

            let mut start = 0;
            while let Some(pos) = text_lower[start..].windows(pattern_lower.len())
                .position(|window| window == pattern_lower) {
                let absolute_pos = start + pos;
                matches.push(aho_corasick::Match::new(
                    0, // pattern_id
                    absolute_pos,
                    absolute_pos + pattern.len()
                ));
                start = absolute_pos + 1;
            }
        } else {
            // Case-sensitive search
            let mut start = 0;
            while let Some(pos) = text[start..].windows(pattern.len())
                .position(|window| window == pattern) {
                let absolute_pos = start + pos;
                matches.push(aho_corasick::Match::new(
                    0, // pattern_id
                    absolute_pos,
                    absolute_pos + pattern.len()
                ));
                start = absolute_pos + 1;
            }
        }

        matches
    }
}

/// Fixed string matcher optimized for literal patterns
pub struct FixedStringMatcher {
    pattern: Vec<u8>,
    ignore_case: bool,
}

impl FixedStringMatcher {
    /// Create a new fixed string matcher
    pub fn new(pattern: &str, ignore_case: bool) -> Self {
        let pattern = if ignore_case {
            pattern.to_lowercase().into_bytes()
        } else {
            pattern.as_bytes().to_vec()
        };

        Self {
            pattern,
            ignore_case,
        }
    }

    /// Find all matches in the given text
    pub fn find_all(&self, text: &[u8]) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();

        let search_text = if self.ignore_case {
            text.to_ascii_lowercase()
        } else {
            text.to_vec()
        };

        let mut start = 0;
        while let Some(pos) = search_text[start..].windows(self.pattern.len())
            .position(|window| window == self.pattern) {
            let absolute_pos = start + pos;
            matches.push((absolute_pos, absolute_pos + self.pattern.len()));
            start = absolute_pos + 1;
        }

        matches
    }

    /// Check if the pattern matches at a specific position
    pub fn matches_at(&self, text: &[u8], pos: usize) -> bool {
        if pos + self.pattern.len() > text.len() {
            return false;
        }

        let slice = &text[pos..pos + self.pattern.len()];

        if self.ignore_case {
            slice.to_ascii_lowercase() == self.pattern
        } else {
            slice == self.pattern
        }
    }
}

/// Word boundary detection utilities
pub struct WordBoundary;

impl WordBoundary {
    /// Check if a character is a word character
    pub fn is_word_char(ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || ch == b'_'
    }

    /// Check if position is at a word boundary
    pub fn is_word_boundary(text: &[u8], pos: usize) -> bool {
        let prev_is_word = pos > 0 && Self::is_word_char(text[pos - 1]);
        let curr_is_word = pos < text.len() && Self::is_word_char(text[pos]);

        prev_is_word != curr_is_word
    }

    /// Find word boundaries around a match
    pub fn find_word_boundaries(text: &[u8], start: usize, end: usize) -> (bool, bool) {
        let start_boundary = Self::is_word_boundary(text, start);
        let end_boundary = Self::is_word_boundary(text, end);

        (start_boundary, end_boundary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_string_matcher() {
        let matcher = FixedStringMatcher::new("test", false);
        let matches = matcher.find_all(b"this is a test string with test again");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], (10, 14)); // First "test"
        assert_eq!(matches[1], (32, 36)); // Second "test"
    }

    #[test]
    fn test_fixed_string_matcher_case_insensitive() {
        let matcher = FixedStringMatcher::new("TEST", true);
        let matches = matcher.find_all(b"this is a test string with Test again");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], (10, 14)); // "test"
        assert_eq!(matches[1], (32, 36)); // "Test"
    }

    #[test]
    fn test_multi_pattern_matcher() {
        let patterns = vec!["cat".to_string(), "dog".to_string()];
        let matcher = MultiPatternMatcher::new(patterns, false).unwrap();
        let matches = matcher.find_matches(b"the cat and dog are playing");

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_word_boundary() {
        assert!(WordBoundary::is_word_char(b'a'));
        assert!(WordBoundary::is_word_char(b'Z'));
        assert!(WordBoundary::is_word_char(b'5'));
        assert!(WordBoundary::is_word_char(b'_'));
        assert!(!WordBoundary::is_word_char(b' '));
        assert!(!WordBoundary::is_word_char(b'.'));
    }

    #[test]
    fn test_word_boundary_detection() {
        let text = b"hello world";

        // Start of "hello"
        assert!(WordBoundary::is_word_boundary(text, 0));

        // Between "hello" and " "
        assert!(WordBoundary::is_word_boundary(text, 5));

        // Between " " and "world"
        assert!(WordBoundary::is_word_boundary(text, 6));

        // Middle of "hello"
        assert!(!WordBoundary::is_word_boundary(text, 2));
    }
}
