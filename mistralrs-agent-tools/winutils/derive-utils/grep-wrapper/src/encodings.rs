//! Text encoding detection and handling for Windows

use anyhow::{anyhow, Result};
use encoding_rs::{Encoding, Decoder, UTF_8, UTF_16LE, UTF_16BE, WINDOWS_1252};
use std::io::Read;

/// Text encoding types supported on Windows
#[derive(Debug, Clone, PartialEq)]
pub enum TextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    Ascii,
    Auto,
}

impl TextEncoding {
    /// Parse encoding from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "utf8" | "utf-8" => Ok(Self::Utf8),
            "utf16le" | "utf-16le" => Ok(Self::Utf16Le),
            "utf16be" | "utf-16be" => Ok(Self::Utf16Be),
            "windows1252" | "windows-1252" | "cp1252" => Ok(Self::Windows1252),
            "ascii" => Ok(Self::Ascii),
            "auto" => Ok(Self::Auto),
            _ => Err(anyhow!("Unsupported encoding: {}", s)),
        }
    }

    /// Get the corresponding encoding_rs Encoding
    pub fn to_encoding_rs(&self) -> Option<&'static Encoding> {
        match self {
            Self::Utf8 => Some(UTF_8),
            Self::Utf16Le => Some(UTF_16LE),
            Self::Utf16Be => Some(UTF_16BE),
            Self::Windows1252 => Some(WINDOWS_1252),
            Self::Ascii => Some(UTF_8), // ASCII is a subset of UTF-8
            Self::Auto => None, // Will be detected
        }
    }
}

/// Detect encoding from configuration string
pub fn detect_encoding(encoding_str: &str) -> Result<Option<&'static Encoding>> {
    let text_encoding = TextEncoding::from_str(encoding_str)?;
    Ok(text_encoding.to_encoding_rs())
}

/// Encoding detector that uses byte order marks and heuristics
pub struct EncodingDetector;

impl EncodingDetector {
    /// Detect encoding from byte content
    pub fn detect(content: &[u8]) -> TextEncoding {
        // Check for BOM (Byte Order Mark)
        if let Some(encoding) = Self::detect_bom(content) {
            return encoding;
        }

        // Use heuristics to detect encoding
        Self::detect_heuristic(content)
    }

    /// Detect encoding from BOM
    fn detect_bom(content: &[u8]) -> Option<TextEncoding> {
        if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            // UTF-8 BOM
            Some(TextEncoding::Utf8)
        } else if content.starts_with(&[0xFF, 0xFE]) {
            // UTF-16LE BOM
            Some(TextEncoding::Utf16Le)
        } else if content.starts_with(&[0xFE, 0xFF]) {
            // UTF-16BE BOM
            Some(TextEncoding::Utf16Be)
        } else {
            None
        }
    }

    /// Detect encoding using heuristics
    fn detect_heuristic(content: &[u8]) -> TextEncoding {
        if content.is_empty() {
            return TextEncoding::Utf8;
        }

        // Check if it's valid UTF-8
        if std::str::from_utf8(content).is_ok() {
            return TextEncoding::Utf8;
        }

        // Check for UTF-16 patterns
        if Self::looks_like_utf16le(content) {
            return TextEncoding::Utf16Le;
        }

        if Self::looks_like_utf16be(content) {
            return TextEncoding::Utf16Be;
        }

        // Check if it's ASCII
        if content.iter().all(|&b| b < 128) {
            return TextEncoding::Ascii;
        }

        // Default to Windows-1252 for Windows systems
        TextEncoding::Windows1252
    }

    /// Check if content looks like UTF-16LE
    fn looks_like_utf16le(content: &[u8]) -> bool {
        if content.len() < 2 || content.len() % 2 != 0 {
            return false;
        }

        // Look for patterns typical of UTF-16LE text
        let mut ascii_like_chars = 0;
        let mut total_chars = 0;

        for chunk in content.chunks_exact(2) {
            let code_unit = u16::from_le_bytes([chunk[0], chunk[1]]);
            total_chars += 1;

            // Check for ASCII-like characters (0x0020-0x007E)
            if (0x0020..=0x007E).contains(&code_unit) {
                ascii_like_chars += 1;
            }
        }

        // If more than 70% of characters are ASCII-like, it's probably UTF-16LE
        total_chars > 0 && (ascii_like_chars * 100 / total_chars) > 70
    }

    /// Check if content looks like UTF-16BE
    fn looks_like_utf16be(content: &[u8]) -> bool {
        if content.len() < 2 || content.len() % 2 != 0 {
            return false;
        }

        // Look for patterns typical of UTF-16BE text
        let mut ascii_like_chars = 0;
        let mut total_chars = 0;

        for chunk in content.chunks_exact(2) {
            let code_unit = u16::from_be_bytes([chunk[0], chunk[1]]);
            total_chars += 1;

            // Check for ASCII-like characters (0x0020-0x007E)
            if (0x0020..=0x007E).contains(&code_unit) {
                ascii_like_chars += 1;
            }
        }

        // If more than 70% of characters are ASCII-like, it's probably UTF-16BE
        total_chars > 0 && (ascii_like_chars * 100 / total_chars) > 70
    }
}

/// Text decoder that handles various Windows encodings
pub struct TextDecoder {
    encoding: &'static Encoding,
    decoder: Decoder,
}

impl TextDecoder {
    /// Create a new text decoder
    pub fn new(encoding: &'static Encoding) -> Self {
        Self {
            encoding,
            decoder: encoding.new_decoder(),
        }
    }

    /// Decode bytes to string
    pub fn decode(&mut self, input: &[u8]) -> Result<String> {
        let max_output_len = self.decoder.max_utf8_buffer_length(input.len())
            .ok_or_else(|| anyhow!("Input too large"))?;

        let mut output = vec![0u8; max_output_len];
        let (result, _read, written, _had_errors) = self.decoder.decode_to_utf8(
            input,
            &mut output,
            false, // last
        );

        match result {
            encoding_rs::CoderResult::InputEmpty => {
                output.truncate(written);
                String::from_utf8(output)
                    .map_err(|e| anyhow!("Failed to convert to UTF-8: {}", e))
            }
            encoding_rs::CoderResult::OutputFull => {
                Err(anyhow!("Output buffer too small"))
            }
        }
    }

    /// Decode bytes to string with error handling
    pub fn decode_lossy(&mut self, input: &[u8]) -> String {
        let max_output_len = self.decoder.max_utf8_buffer_length(input.len())
            .unwrap_or(input.len() * 3);

        let mut output = vec![0u8; max_output_len];
        let (_result, _read, written, _had_errors) = self.decoder.decode_to_utf8(
            input,
            &mut output,
            true, // last - replace errors with replacement character
        );

        output.truncate(written);
        String::from_utf8_lossy(&output).into_owned()
    }
}

/// Line ending detection and normalization
pub struct LineEndingHandler;

impl LineEndingHandler {
    /// Detect line ending style
    pub fn detect(content: &[u8]) -> LineEndingStyle {
        let mut crlf_count = 0;
        let mut lf_count = 0;
        let mut cr_count = 0;

        let mut i = 0;
        while i < content.len() {
            match content[i] {
                b'\r' => {
                    if i + 1 < content.len() && content[i + 1] == b'\n' {
                        crlf_count += 1;
                        i += 2; // Skip the LF
                    } else {
                        cr_count += 1;
                        i += 1;
                    }
                }
                b'\n' => {
                    lf_count += 1;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        // Determine predominant style
        if crlf_count > lf_count && crlf_count > cr_count {
            LineEndingStyle::Crlf
        } else if lf_count > cr_count {
            LineEndingStyle::Lf
        } else if cr_count > 0 {
            LineEndingStyle::Cr
        } else {
            LineEndingStyle::Lf // Default
        }
    }

    /// Normalize line endings to LF
    pub fn normalize_to_lf(content: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(content.len());
        let mut i = 0;

        while i < content.len() {
            match content[i] {
                b'\r' => {
                    if i + 1 < content.len() && content[i + 1] == b'\n' {
                        // CRLF -> LF
                        result.push(b'\n');
                        i += 2;
                    } else {
                        // CR -> LF
                        result.push(b'\n');
                        i += 1;
                    }
                }
                other => {
                    result.push(other);
                    i += 1;
                }
            }
        }

        result
    }

    /// Convert content to lines, handling different line endings
    pub fn to_lines(content: &[u8]) -> Vec<&[u8]> {
        let mut lines = Vec::new();
        let mut start = 0;

        for (i, &byte) in content.iter().enumerate() {
            match byte {
                b'\n' => {
                    // Handle both LF and CRLF
                    let end = if i > 0 && content[i - 1] == b'\r' {
                        i - 1 // Exclude CR from CRLF
                    } else {
                        i // Just LF
                    };
                    lines.push(&content[start..end]);
                    start = i + 1;
                }
                b'\r' => {
                    // Handle standalone CR (old Mac style)
                    if i + 1 >= content.len() || content[i + 1] != b'\n' {
                        lines.push(&content[start..i]);
                        start = i + 1;
                    }
                    // If followed by LF, it will be handled by the LF case
                }
                _ => {}
            }
        }

        // Add the last line if it doesn't end with a newline
        if start < content.len() {
            lines.push(&content[start..]);
        }

        lines
    }
}

/// Line ending styles
#[derive(Debug, Clone, PartialEq)]
pub enum LineEndingStyle {
    Lf,   // Unix/Linux: \n
    Crlf, // Windows: \r\n
    Cr,   // Old Mac: \r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_detection() {
        // UTF-8 BOM
        let utf8_bom = &[0xEF, 0xBB, 0xBF, b'h', b'e', b'l', b'l', b'o'];
        assert_eq!(EncodingDetector::detect(utf8_bom), TextEncoding::Utf8);

        // UTF-16LE BOM
        let utf16le_bom = &[0xFF, 0xFE, b'h', 0, b'i', 0];
        assert_eq!(EncodingDetector::detect(utf16le_bom), TextEncoding::Utf16Le);

        // UTF-16BE BOM
        let utf16be_bom = &[0xFE, 0xFF, 0, b'h', 0, b'i'];
        assert_eq!(EncodingDetector::detect(utf16be_bom), TextEncoding::Utf16Be);

        // Plain ASCII
        let ascii = b"hello world";
        assert_eq!(EncodingDetector::detect(ascii), TextEncoding::Utf8);
    }

    #[test]
    fn test_line_ending_detection() {
        // CRLF (Windows)
        let crlf_content = b"line1\r\nline2\r\nline3\r\n";
        assert_eq!(LineEndingHandler::detect(crlf_content), LineEndingStyle::Crlf);

        // LF (Unix)
        let lf_content = b"line1\nline2\nline3\n";
        assert_eq!(LineEndingHandler::detect(lf_content), LineEndingStyle::Lf);

        // CR (Old Mac)
        let cr_content = b"line1\rline2\rline3\r";
        assert_eq!(LineEndingHandler::detect(cr_content), LineEndingStyle::Cr);
    }

    #[test]
    fn test_line_ending_normalization() {
        let crlf_content = b"line1\r\nline2\r\nline3";
        let normalized = LineEndingHandler::normalize_to_lf(crlf_content);
        assert_eq!(normalized, b"line1\nline2\nline3");

        let cr_content = b"line1\rline2\rline3";
        let normalized = LineEndingHandler::normalize_to_lf(cr_content);
        assert_eq!(normalized, b"line1\nline2\nline3");
    }

    #[test]
    fn test_to_lines() {
        let crlf_content = b"line1\r\nline2\r\nline3";
        let lines = LineEndingHandler::to_lines(crlf_content);
        assert_eq!(lines, vec![b"line1", b"line2", b"line3"]);

        let lf_content = b"line1\nline2\nline3";
        let lines = LineEndingHandler::to_lines(lf_content);
        assert_eq!(lines, vec![b"line1", b"line2", b"line3"]);

        let mixed_content = b"line1\r\nline2\nline3\rline4";
        let lines = LineEndingHandler::to_lines(mixed_content);
        assert_eq!(lines, vec![b"line1", b"line2", b"line3", b"line4"]);
    }

    #[test]
    fn test_text_decoder() {
        let mut decoder = TextDecoder::new(UTF_8);
        let result = decoder.decode(b"hello world").unwrap();
        assert_eq!(result, "hello world");

        let mut decoder = TextDecoder::new(WINDOWS_1252);
        let windows_text = &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0xE9]; // "Hello é"
        let result = decoder.decode(windows_text).unwrap();
        assert_eq!(result, "Hello é");
    }
}
