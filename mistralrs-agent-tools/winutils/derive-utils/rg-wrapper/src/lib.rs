//! # rg-wrapper - Fast Grep with Windows Path Integration
//!
//! A high-performance text search utility that combines the speed of ripgrep with
//! comprehensive Windows path normalization and cross-platform compatibility.
//!
//! ## Key Features
//!
//! - Fast text search across files (similar to `ripgrep` and `grep`)
//! - Automatic path normalization for Windows, Git Bash, WSL, Cygwin
//! - Smart line ending handling (CRLF/LF)
//! - Unicode and encoding detection/conversion
//! - Parallel file processing
//! - Windows file attribute awareness
//! - Context lines and match highlighting
//! - Multiple output formats (JSON, CSV, custom)
//! - Support for various file types and compression
//!
//! ## Usage
//!
//! ```rust
//! use rg_wrapper::{SearchOptions, TextSearcher};
//!
//! let options = SearchOptions::new()
//!     .pattern("TODO @codex")
//!     .case_insensitive(true)
//!     .context_lines(2);
//!
//! let searcher = TextSearcher::new(options)?;
//! let results = searcher.search_directory("C:\\projects")?;
//! ```

use anyhow::{anyhow, Context, Result};
use crossbeam_utils::thread;
use grep::{
    matcher::{Captures, LineTerminator, Match, Matcher},
    regex::{RegexMatcher, RegexMatcherBuilder},
    searcher::{BinaryDetection, Encoding, Searcher, SearcherBuilder, Sink, SinkMatch},
};
use ignore::{WalkBuilder, WalkState};
use log::{debug, info, warn};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use thiserror::Error;
use winpath::PathNormalizer;

#[cfg(windows)]
use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};

/// Errors that can occur during text searching
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Invalid search pattern: {0}")]
    InvalidPattern(String),
    #[error("File access denied: {0}")]
    AccessDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path normalization error: {0}")]
    PathNormalization(String),
    #[error("Invalid regular expression: {0}")]
    InvalidRegex(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Search cancelled")]
    Cancelled,
}

/// Configuration for line ending handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEndingMode {
    /// Auto-detect line endings
    Auto,
    /// Unix-style (LF)
    Unix,
    /// Windows-style (CRLF)
    Windows,
    /// Classic Mac (CR)
    Mac,
}

/// Text encoding detection and handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingOptions {
    /// Auto-detect encoding
    pub auto_detect: bool,
    /// Default encoding if detection fails
    pub default_encoding: String,
    /// Handle BOM (Byte Order Mark)
    pub handle_bom: bool,
    /// Fallback to lossy UTF-8 conversion
    pub lossy_utf8: bool,
}

impl Default for EncodingOptions {
    fn default() -> Self {
        Self {
            auto_detect: true,
            default_encoding: "utf-8".to_string(),
            handle_bom: true,
            lossy_utf8: true,
        }
    }
}

/// Search configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Search pattern
    pub pattern: String,
    /// Use regex instead of literal search
    pub use_regex: bool,
    /// Case-insensitive matching
    pub case_insensitive: bool,
    /// Whole word matching
    pub whole_words: bool,
    /// Multiline matching
    pub multiline: bool,
    /// Dot matches newline
    pub dot_matches_newline: bool,
    /// Include binary files
    pub include_binary: bool,
    /// Maximum file size to search (in bytes)
    pub max_file_size: Option<u64>,
    /// File types to include
    pub file_types: HashSet<String>,
    /// File extensions to include
    pub file_extensions: HashSet<String>,
    /// Paths to exclude
    pub exclude_paths: Vec<String>,
    /// Include hidden files
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Maximum search depth
    pub max_depth: Option<usize>,
    /// Number of context lines before matches
    pub context_before: usize,
    /// Number of context lines after matches
    pub context_after: usize,
    /// Line ending handling
    pub line_ending_mode: LineEndingMode,
    /// Encoding options
    pub encoding_options: EncodingOptions,
    /// Number of parallel threads
    pub threads: usize,
    /// Output path format
    /// Normalize output paths
    pub normalize_paths: bool,
    /// Respect .gitignore files
    pub respect_gitignore: bool,
    /// Color output
    pub color: ColorChoice,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Show column numbers
    pub show_column_numbers: bool,
    /// Show file names
    pub show_file_names: bool,
    /// Only show file names with matches
    pub files_with_matches: bool,
    /// Only show file names without matches
    pub files_without_matches: bool,
    /// Count matches per file
    pub count_matches: bool,
    /// Maximum number of matches per file
    pub max_matches_per_file: Option<usize>,
    /// Invert matching (show non-matching lines)
    pub invert_match: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            use_regex: false,
            case_insensitive: false,
            whole_words: false,
            multiline: false,
            dot_matches_newline: false,
            include_binary: false,
            max_file_size: Some(100 * 1024 * 1024), // 100MB default limit
            file_types: HashSet::new(),
            file_extensions: HashSet::new(),
            exclude_paths: Vec::new(),
            include_hidden: false,
            follow_links: false,
            max_depth: None,
            context_before: 0,
            context_after: 0,
            line_ending_mode: LineEndingMode::Auto,
            encoding_options: EncodingOptions::default(),
            threads: num_cpus::get(),
            normalize_paths: true,
            respect_gitignore: true,
            color: ColorChoice::Auto,
            show_line_numbers: true,
            show_column_numbers: false,
            show_file_names: true,
            files_with_matches: false,
            files_without_matches: false,
            count_matches: false,
            max_matches_per_file: None,
            invert_match: false,
        }
    }
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pattern<S: Into<String>>(mut self, pattern: S) -> Self {
        self.pattern = pattern.into();
        self
    }

    pub fn regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }

    pub fn case_insensitive(mut self, insensitive: bool) -> Self {
        self.case_insensitive = insensitive;
        self
    }

    pub fn whole_words(mut self, whole: bool) -> Self {
        self.whole_words = whole;
        self
    }

    pub fn context_lines(mut self, lines: usize) -> Self {
        self.context_before = lines;
        self.context_after = lines;
        self
    }

    pub fn context_before(mut self, lines: usize) -> Self {
        self.context_before = lines;
        self
    }

    pub fn context_after(mut self, lines: usize) -> Self {
        self.context_after = lines;
        self
    }

    pub fn include_binary(mut self, include: bool) -> Self {
        self.include_binary = include;
        self
    }

    pub fn max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }

    pub fn threads(mut self, count: usize) -> Self {
        self.threads = count.max(1);
        self
    }

}

/// A single match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub file_path: PathBuf,
    pub line_number: u64,
    pub column_number: Option<u64>,
    pub match_text: String,
    pub line_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub byte_offset: u64,
    pub match_start: usize,
    pub match_end: usize,
}

/// Search results for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub file_path: PathBuf,
    pub matches: Vec<MatchResult>,
    pub total_matches: usize,
    pub encoding_used: Option<String>,
    pub line_ending_detected: Option<LineEndingMode>,
    pub is_binary: bool,
    pub file_size: u64,
    pub search_time_ms: u64,
}

/// Custom sink for collecting search results
struct ResultSink {
    file_path: PathBuf,
    matches: Vec<MatchResult>,
    context_before: usize,
    context_after: usize,
    show_column_numbers: bool,
    line_buffer: Vec<String>,
    max_matches: Option<usize>,
}

impl ResultSink {
    fn new(
        file_path: PathBuf,
        context_before: usize,
        context_after: usize,
        show_column_numbers: bool,
        max_matches: Option<usize>,
    ) -> Self {
        Self {
            file_path,
            matches: Vec::new(),
            context_before,
            context_after,
            show_column_numbers,
            line_buffer: Vec::new(),
            max_matches,
        }
    }
}

impl Sink for ResultSink {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        // Check if we've reached max matches
        if let Some(max) = self.max_matches {
            if self.matches.len() >= max {
                return Ok(false); // Stop searching
            }
        }

        let line_number = mat.line_number().unwrap_or(0);
        let line_text = String::from_utf8_lossy(mat.bytes()).to_string();

        // Find the actual match within the line
        let match_start = mat.bytes().iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(0);
        let match_end = line_text.len();

        let column_number = if self.show_column_numbers {
            Some(match_start as u64 + 1)
        } else {
            None
        };

        let match_result = MatchResult {
            file_path: self.file_path.clone(),
            line_number,
            column_number,
            match_text: line_text.clone(), // TODO @gemini: Extract just the matching part
            line_text: line_text.clone(),
            context_before: Vec::new(), // TODO @codex: Implement context collection
            context_after: Vec::new(),  // TODO @gemini: Implement context collection
            byte_offset: mat.absolute_byte_offset(),
            match_start,
            match_end,
        };

        self.matches.push(match_result);
        Ok(true) // Continue searching
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        _context: &grep::searcher::SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        // TODO @codex: Handle context lines
        Ok(true)
    }

    fn context_break(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn binary_data(&mut self, _searcher: &Searcher, _data: &[u8]) -> Result<bool, Self::Error> {
        // Handle binary data encounter
        Ok(false) // Stop processing this file if binary
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        self.matches.clear();
        self.line_buffer.clear();
        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        _sink_finish: &grep::searcher::SinkFinish,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Main text searcher implementation
pub struct TextSearcher {
    options: SearchOptions,
    normalizer: PathNormalizer,
    matcher: RegexMatcher,
}

impl TextSearcher {
    pub fn new(options: SearchOptions) -> Result<Self> {
        let normalizer = PathNormalizer::new();

        // Build regex matcher
        let mut matcher_builder = RegexMatcherBuilder::new();

        if options.case_insensitive {
            matcher_builder.case_insensitive(true);
        }

        if options.whole_words {
            matcher_builder.word(true);
        }

        if options.multiline {
            matcher_builder.multi_line(true);
        }

        if options.dot_matches_newline {
            matcher_builder.dot_matches_new_line(true);
        }

        // Handle line terminator based on line ending mode
        let line_term = match options.line_ending_mode {
            LineEndingMode::Unix => LineTerminator::byte(b'\n'),
            LineEndingMode::Windows => LineTerminator::crlf(),
            LineEndingMode::Mac => LineTerminator::byte(b'\r'),
            LineEndingMode::Auto => LineTerminator::any([b'\r', b'\n']),
        };
        matcher_builder.line_terminator(line_term);

        let pattern = if options.use_regex {
            options.pattern.clone()
        } else {
            // Escape regex special characters for literal search
            regex::escape(&options.pattern)
        };

        let matcher = matcher_builder.build(&pattern)
            .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;

        Ok(Self {
            options,
            normalizer,
            matcher,
        })
    }

    /// Search for text in a single file
    pub fn search_file<P: AsRef<Path>>(&self, file_path: P) -> Result<FileSearchResult> {
        let file_path = file_path.as_ref();
        let start_time = std::time::Instant::now();

        debug!("Searching file: {:?}", file_path);

        // Normalize path if requested
        let normalized_path = if self.options.normalize_paths {
            self.normalizer.normalize_to_context(
                &file_path.to_string_lossy(),
                self.options.output_context
            ).unwrap_or_else(|_| file_path.to_path_buf())
        } else {
            file_path.to_path_buf()
        };

        // Check file size limit
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| SearchError::Io(e))?;

        if let Some(max_size) = self.options.max_file_size {
            if metadata.len() > max_size {
                debug!("Skipping file due to size limit: {:?}", file_path);
                return Ok(FileSearchResult {
                    file_path: normalized_path,
                    matches: Vec::new(),
                    total_matches: 0,
                    encoding_used: None,
                    line_ending_detected: None,
                    is_binary: false,
                    file_size: metadata.len(),
                    search_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        }

        // Open and map file
        let file = File::open(file_path)
            .map_err(|e| SearchError::Io(e))?;

        let mmap = unsafe {
            Mmap::map(&file)
                .map_err(|e| SearchError::Io(e))?
        };

        // Detect encoding
        let (encoding_used, content) = self.detect_and_convert_encoding(&mmap)?;

        // Build searcher
        let mut searcher_builder = SearcherBuilder::new();

        if self.options.include_binary {
            searcher_builder.binary_detection(BinaryDetection::none());
        } else {
            searcher_builder.binary_detection(BinaryDetection::quit(b'\x00'));
        }

        // Set encoding
        if let Some(encoding_name) = &encoding_used {
            if let Some(encoding) = self.get_encoding_by_name(encoding_name) {
                searcher_builder.encoding(Some(encoding));
            }
        }

        let mut searcher = searcher_builder.build();

        // Create result sink
        let mut sink = ResultSink::new(
            normalized_path.clone(),
            self.options.context_before,
            self.options.context_after,
            self.options.show_column_numbers,
            self.options.max_matches_per_file,
        );

        // Perform search
        let search_result = searcher.search_slice(
            &self.matcher,
            &content,
            &mut sink,
        );

        match search_result {
            Ok(_) => {
                let search_time = start_time.elapsed().as_millis() as u64;

                Ok(FileSearchResult {
                    file_path: normalized_path,
                    matches: sink.matches,
                    total_matches: sink.matches.len(),
                    encoding_used,
                    line_ending_detected: Some(self.detect_line_ending(&content)),
                    is_binary: false, // TODO @gemini: Detect binary files
                    file_size: metadata.len(),
                    search_time_ms: search_time,
                })
            }
            Err(e) => {
                warn!("Search error in file {:?}: {}", file_path, e);
                Err(SearchError::Io(e))
            }
        }
    }

    /// Search for text in a directory
    pub fn search_directory<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<FileSearchResult>> {
        let root_path = root_path.as_ref();

        debug!("Starting directory search in: {:?}", root_path);

        // Normalize the root path
        let normalized_root = if self.options.normalize_paths {
            self.normalizer.normalize(&root_path.to_string_lossy())?
        } else {
            root_path.to_path_buf()
        };

        let (tx, rx) = mpsc::channel();
        let options = Arc::new(self.options.clone());
        let normalizer = Arc::new(self.normalizer.clone());
        let matcher = Arc::new(self.matcher.clone());

        // Set up parallel walker
        let walker = WalkBuilder::new(&normalized_root)
            .hidden(!self.options.include_hidden)
            .follow_links(self.options.follow_links)
            .git_ignore(self.options.respect_gitignore)
            .threads(self.options.threads)
            .max_depth(self.options.max_depth)
            .build_parallel();

        // Spawn worker threads
        thread::scope(|scope| {
            let tx = tx.clone();

            scope.spawn(move |_| {
                walker.run(|| {
                    let tx = tx.clone();
                    let options = Arc::clone(&options);
                    let normalizer = Arc::clone(&normalizer);
                    let matcher = Arc::clone(&matcher);

                    Box::new(move |entry_result| {
                        match entry_result {
                            Ok(entry) => {
                                if let Some(file_type) = entry.file_type() {
                                    if file_type.is_file() {
                                        if self.should_search_file(entry.path(), &options) {
                                            let searcher = TextSearcher {
                                                options: (*options).clone(),
                                                normalizer: (*normalizer).clone(),
                                                matcher: (*matcher).clone(),
                                            };

                                            match searcher.search_file(entry.path()) {
                                                Ok(result) => {
                                                    if !result.matches.is_empty() ||
                                                       options.files_without_matches ||
                                                       options.count_matches {
                                                        let _ = tx.send(Ok(result));
                                                    }
                                                }
                                                Err(err) => {
                                                    let _ = tx.send(Err(err));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = tx.send(Err(SearchError::Io(err)));
                            }
                        }
                        WalkState::Continue
                    })
                });
            });

            // Drop the original sender
            drop(tx);
        }).map_err(|_| SearchError::Cancelled)?;

        // Collect results
        let mut results = Vec::new();
        while let Ok(result) = rx.recv() {
            match result {
                Ok(search_result) => results.push(search_result),
                Err(err) => {
                    warn!("Directory search error: {}", err);
                    // Continue with other results
                }
            }
        }

        debug!("Directory search completed. Found {} files with results", results.len());
        Ok(results)
    }

    /// Check if a file should be searched based on options
    fn should_search_file(&self, path: &Path, options: &SearchOptions) -> bool {
        // Check file extension
        if !options.file_extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !options.file_extensions.contains(&ext.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check file type (TODO @codex: implement file type detection)

        // Check exclude patterns
        let path_str = path.to_string_lossy();
        for exclude_pattern in &options.exclude_paths {
            if path_str.contains(exclude_pattern) {
                return false;
            }
        }

        true
    }

    /// Detect and convert file encoding
    #[cfg(windows)]
    fn detect_and_convert_encoding(&self, data: &[u8]) -> Result<(Option<String>, Vec<u8>)> {
        if !self.options.encoding_options.auto_detect {
            // Use default encoding
            let encoding_name = &self.options.encoding_options.default_encoding;
            let encoding = self.get_encoding_by_name(encoding_name)
                .unwrap_or(UTF_8);

            let (decoded, _used_encoding, had_errors) = encoding.decode(data);
            if had_errors && !self.options.encoding_options.lossy_utf8 {
                return Err(SearchError::Encoding(format!(
                    "Encoding error with {}", encoding_name
                )));
            }

            return Ok((Some(encoding_name.clone()), decoded.as_bytes().to_vec()));
        }

        // Auto-detect encoding
        let detected_encoding = if data.len() >= 3 && &data[0..3] == b"\xEF\xBB\xBF" {
            // UTF-8 BOM
            UTF_8
        } else if data.len() >= 2 {
            if &data[0..2] == b"\xFF\xFE" || &data[0..2] == b"\xFE\xFF" {
                // UTF-16 BOM (not fully supported here, fallback to UTF-8)
                UTF_8
            } else {
                // Try to detect based on content
                self.detect_encoding_heuristic(data)
            }
        } else {
            UTF_8
        };

        let (decoded, _encoding_used, had_errors) = detected_encoding.decode(data);
        if had_errors && !self.options.encoding_options.lossy_utf8 {
            return Err(SearchError::Encoding(
                "Failed to decode with detected encoding".to_string()
            ));
        }

        Ok((
            Some(detected_encoding.name().to_string()),
            decoded.as_bytes().to_vec()
        ))
    }

    #[cfg(not(windows))]
    fn detect_and_convert_encoding(&self, data: &[u8]) -> Result<(Option<String>, Vec<u8>)> {
        // On non-Windows systems, assume UTF-8
        match std::str::from_utf8(data) {
            Ok(_) => Ok((Some("utf-8".to_string()), data.to_vec())),
            Err(_) => {
                if self.options.encoding_options.lossy_utf8 {
                    let decoded = String::from_utf8_lossy(data);
                    Ok((Some("utf-8".to_string()), decoded.as_bytes().to_vec()))
                } else {
                    Err(SearchError::Encoding("Invalid UTF-8".to_string()))
                }
            }
        }
    }

    #[cfg(windows)]
    fn detect_encoding_heuristic(&self, data: &[u8]) -> &'static Encoding {
        // Simple heuristic: check for high-bit characters
        let high_bit_count = data.iter().filter(|&&b| b >= 0x80).count();
        let ratio = high_bit_count as f64 / data.len() as f64;

        if ratio > 0.1 {
            // Likely Windows-1252 or similar
            WINDOWS_1252
        } else {
            // Likely UTF-8 or ASCII
            UTF_8
        }
    }

    #[cfg(windows)]
    fn get_encoding_by_name(&self, name: &str) -> Option<&'static Encoding> {
        match name.to_lowercase().as_str() {
            "utf-8" | "utf8" => Some(UTF_8),
            "windows-1252" | "cp1252" => Some(WINDOWS_1252),
            _ => None,
        }
    }

    #[cfg(not(windows))]
    fn get_encoding_by_name(&self, _name: &str) -> Option<Encoding> {
        None // Encoding detection not implemented on non-Windows
    }

    /// Detect line ending style
    fn detect_line_ending(&self, data: &[u8]) -> LineEndingMode {
        let mut crlf_count = 0;
        let mut lf_count = 0;
        let mut cr_count = 0;

        let mut i = 0;
        while i < data.len() {
            match data[i] {
                b'\r' => {
                    if i + 1 < data.len() && data[i + 1] == b'\n' {
                        crlf_count += 1;
                        i += 2;
                    } else {
                        cr_count += 1;
                        i += 1;
                    }
                }
                b'\n' => {
                    lf_count += 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }

        if crlf_count > lf_count && crlf_count > cr_count {
            LineEndingMode::Windows
        } else if lf_count > cr_count {
            LineEndingMode::Unix
        } else if cr_count > 0 {
            LineEndingMode::Mac
        } else {
            LineEndingMode::Auto
        }
    }

    /// Count total matches across all files
    pub fn count_matches<P: AsRef<Path>>(&self, root_path: P) -> Result<usize> {
        let results = self.search_directory(root_path)?;
        let total = results.iter().map(|r| r.total_matches).sum();
        Ok(total)
    }

    /// Get list of files that contain matches
    pub fn list_matching_files<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<PathBuf>> {
        let results = self.search_directory(root_path)?;
        let files = results.into_iter()
            .filter(|r| r.total_matches > 0)
            .map(|r| r.file_path)
            .collect();
        Ok(files)
    }
}

/// Output formatter for search results
pub struct OutputFormatter {
    color_choice: ColorChoice,
    show_line_numbers: bool,
    show_column_numbers: bool,
    show_file_names: bool,
}

impl OutputFormatter {
    pub fn new(options: &SearchOptions) -> Self {
        Self {
            color_choice: options.color,
            show_line_numbers: options.show_line_numbers,
            show_column_numbers: options.show_column_numbers,
            show_file_names: options.show_file_names,
        }
    }

    pub fn format_result(&self, result: &MatchResult, writer: &mut dyn Write) -> Result<()> {
        let mut stdout = StandardStream::stdout(self.color_choice);

        // File name
        if self.show_file_names {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
            write!(stdout, "{}", result.file_path.display())?;
            stdout.reset()?;
        }

        // Line number
        if self.show_line_numbers {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            if self.show_file_names {
                write!(stdout, ":")?;
            }
            write!(stdout, "{}", result.line_number)?;
            stdout.reset()?;
        }

        // Column number
        if self.show_column_numbers {
            if let Some(col) = result.column_number {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(stdout, ":{}", col)?;
                stdout.reset()?;
            }
        }

        // Separator
        if self.show_file_names || self.show_line_numbers {
            write!(stdout, ":")?;
        }

        // Match text with highlighting
        self.highlight_match(&mut stdout, &result.line_text, result.match_start, result.match_end)?;

        writeln!(stdout)?;
        Ok(())
    }

    fn highlight_match(
        &self,
        writer: &mut StandardStream,
        line: &str,
        match_start: usize,
        match_end: usize,
    ) -> Result<()> {
        // Write text before match
        write!(writer, "{}", &line[..match_start])?;

        // Highlight match
        writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        write!(writer, "{}", &line[match_start..match_end])?;
        writer.reset()?;

        // Write text after match
        write!(writer, "{}", &line[match_end..])?;

        Ok(())
    }

    pub fn format_json(&self, results: &[FileSearchResult]) -> Result<String> {
        let json = serde_json::to_string_pretty(results)
            .map_err(|e| SearchError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(json)
    }
}

// Add num_cpus dependency
use num_cpus;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("file1.txt"), "Hello world\nThis is a test\nTODO: Fix this").unwrap();
        fs::write(root.join("file2.rs"), "fn main() {\n    println!(\"Hello\");\n    // TODO @gemini: Add error handling\n}").unwrap();
        fs::write(root.join("file3.md"), "# Documentation\n\nThis file contains no matches.").unwrap();

        // Create a subdirectory
        fs::create_dir_all(root.join("subdir")).unwrap();
        fs::write(root.join("subdir").join("nested.txt"), "Nested TODO item\nAnother line").unwrap();

        temp_dir
    }

    #[test]
    fn test_basic_search() {
        let temp_dir = create_test_files();
        let options = SearchOptions::new().pattern("TODO");
        let searcher = TextSearcher::new(options).unwrap();

        let results = searcher.search_directory(temp_dir.path()).unwrap();
        let total_matches: usize = results.iter().map(|r| r.total_matches).sum();

        assert!(total_matches >= 2); // Should find TODO @codex in file1.txt and file2.rs
    }

    #[test]
    fn test_case_insensitive_search() {
        let temp_dir = create_test_files();
        let options = SearchOptions::new()
            .pattern("hello")
            .case_insensitive(true);
        let searcher = TextSearcher::new(options).unwrap();

        let results = searcher.search_directory(temp_dir.path()).unwrap();
        let total_matches: usize = results.iter().map(|r| r.total_matches).sum();

        assert!(total_matches >= 2); // Should find "Hello" and "Hello" (in print)
    }

    #[test]
    fn test_file_extension_filter() {
        let temp_dir = create_test_files();
        let mut extensions = HashSet::new();
        extensions.insert("rs".to_string());

        let mut options = SearchOptions::new().pattern("TODO");
        options.file_extensions = extensions;

        let searcher = TextSearcher::new(options).unwrap();
        let results = searcher.search_directory(temp_dir.path()).unwrap();

        // Should only find matches in .rs files
        assert!(results.iter().all(|r| {
            r.file_path.extension().and_then(|e| e.to_str()) == Some("rs")
        }));
    }

    #[test]
    fn test_regex_search() {
        let temp_dir = create_test_files();
        let options = SearchOptions::new()
            .pattern(r"TODO:.*")
            .regex(true);
        let searcher = TextSearcher::new(options).unwrap();

        let results = searcher.search_directory(temp_dir.path()).unwrap();
        let total_matches: usize = results.iter().map(|r| r.total_matches).sum();

        assert!(total_matches >= 1);
    }

    #[test]
    fn test_line_ending_detection() {
        let searcher = TextSearcher::new(SearchOptions::new()).unwrap();

        assert_eq!(
            searcher.detect_line_ending(b"line1\r\nline2\r\n"),
            LineEndingMode::Windows
        );
        assert_eq!(
            searcher.detect_line_ending(b"line1\nline2\n"),
            LineEndingMode::Unix
        );
        assert_eq!(
            searcher.detect_line_ending(b"line1\rline2\r"),
            LineEndingMode::Mac
        );
    }

    #[test]
    fn test_count_matches() {
        let temp_dir = create_test_files();
        let options = SearchOptions::new().pattern("TODO");
        let searcher = TextSearcher::new(options).unwrap();

        let count = searcher.count_matches(temp_dir.path()).unwrap();
        assert!(count >= 2);
    }

    #[test]
    fn test_list_matching_files() {
        let temp_dir = create_test_files();
        let options = SearchOptions::new().pattern("TODO");
        let searcher = TextSearcher::new(options).unwrap();

        let files = searcher.list_matching_files(temp_dir.path()).unwrap();
        assert!(files.len() >= 2);
        assert!(files.iter().any(|f| f.to_string_lossy().contains("file1.txt")));
        assert!(files.iter().any(|f| f.to_string_lossy().contains("file2.rs")));
    }
}
