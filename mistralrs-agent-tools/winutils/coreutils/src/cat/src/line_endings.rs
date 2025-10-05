//! Line ending detection and conversion utilities
//!
//! This module provides functionality to detect and convert between different
//! line ending formats commonly used on different operating systems.

use crate::Result;
use memchr::{memchr, memchr2};

/// Line ending types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndingType {
    Lf,    // Unix/Linux: \n
    Crlf,  // Windows: \r\n
    Cr,    // Classic Mac: \r
    Mixed, // Mixed line endings detected
}

/// Statistics about line endings in a file
#[derive(Debug, Default)]
pub struct LineEndingStats {
    pub lf_count: usize,
    pub crlf_count: usize,
    pub cr_count: usize,
}

impl LineEndingStats {
    /// Analyze the predominant line ending type
    pub fn predominant_type(&self) -> LineEndingType {
        let total = self.lf_count + self.crlf_count + self.cr_count;
        if total == 0 {
            return LineEndingType::Lf; // Default to LF
        }

        let mut types = vec![
            (LineEndingType::Crlf, self.crlf_count),
            (LineEndingType::Lf, self.lf_count),
            (LineEndingType::Cr, self.cr_count),
        ];
        types.sort_by(|a, b| b.1.cmp(&a.1));

        let (most_common_type, most_common_count) = types[0];
        let (_second_most_type, second_most_count) = types[1];

        // If there's a clear majority (>50%), use that
        if most_common_count as f64 / total as f64 > 0.5 {
            most_common_type
        } else if second_most_count > 0 {
            // Mixed line endings
            LineEndingType::Mixed
        } else {
            most_common_type
        }
    }

    /// Check if the file has consistent line endings
    pub fn is_consistent(&self) -> bool {
        let non_zero_count = [self.lf_count > 0, self.crlf_count > 0, self.cr_count > 0]
            .iter()
            .filter(|&&x| x)
            .count();
        non_zero_count <= 1
    }
}

/// Line ending converter with buffering support
pub struct LineEndingConverter {
    convert_to_crlf: bool,
    convert_to_lf: bool,
    pending_cr: bool,
}

impl LineEndingConverter {
    /// Create a new line ending converter
    pub fn new(convert_to_crlf: bool, convert_to_lf: bool) -> Self {
        Self {
            convert_to_crlf,
            convert_to_lf,
            pending_cr: false,
        }
    }

    /// Convert line endings in the given data
    pub fn convert(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.convert_to_crlf && !self.convert_to_lf {
            return Ok(data.to_vec());
        }

        let mut result = Vec::with_capacity(data.len() + (data.len() / 10)); // 10% extra space
        let mut pos = 0;

        // Handle pending CR from previous buffer
        if self.pending_cr && !data.is_empty() {
            if data[0] != b'\n' {
                // Standalone CR
                if self.convert_to_crlf {
                    result.extend_from_slice(b"\r\n");
                } else if self.convert_to_lf {
                    result.push(b'\n');
                } else {
                    result.push(b'\r');
                }
            }
            // If data[0] is '\n', we'll handle the CRLF sequence below
            self.pending_cr = false;
        }

        while pos < data.len() {
            if let Some(cr_pos) = memchr(b'\r', &data[pos..]) {
                let actual_pos = pos + cr_pos;

                // Copy data before the CR
                result.extend_from_slice(&data[pos..actual_pos]);

                // Check if this is CRLF or standalone CR
                if actual_pos + 1 < data.len() && data[actual_pos + 1] == b'\n' {
                    // CRLF sequence
                    if self.convert_to_lf {
                        result.push(b'\n'); // Convert CRLF to LF
                    } else {
                        result.extend_from_slice(b"\r\n"); // Keep as CRLF
                    }
                    pos = actual_pos + 2;
                } else if actual_pos + 1 == data.len() {
                    // CR at end of buffer - might be part of CRLF
                    self.pending_cr = true;
                    pos = actual_pos + 1;
                } else {
                    // Standalone CR
                    if self.convert_to_crlf {
                        result.extend_from_slice(b"\r\n");
                    } else if self.convert_to_lf {
                        result.push(b'\n');
                    } else {
                        result.push(b'\r');
                    }
                    pos = actual_pos + 1;
                }
            } else if let Some(lf_pos) = memchr(b'\n', &data[pos..]) {
                let actual_pos = pos + lf_pos;

                // Copy data before the LF
                result.extend_from_slice(&data[pos..actual_pos]);

                // Standalone LF (not part of CRLF)
                if self.convert_to_crlf {
                    result.extend_from_slice(b"\r\n");
                } else {
                    result.push(b'\n');
                }
                pos = actual_pos + 1;
            } else {
                // No more line endings, copy rest of data
                result.extend_from_slice(&data[pos..]);
                break;
            }
        }

        Ok(result)
    }

    /// Finalize conversion (handle any pending state)
    pub fn finalize(&mut self) -> Vec<u8> {
        if self.pending_cr {
            self.pending_cr = false;
            if self.convert_to_crlf {
                return b"\r\n".to_vec();
            } else if self.convert_to_lf {
                return b"\n".to_vec();
            } else {
                return b"\r".to_vec();
            }
        }
        Vec::new()
    }
}

/// Analyze line endings in data
pub fn analyze_line_endings(data: &[u8]) -> LineEndingStats {
    let mut stats = LineEndingStats::default();
    let mut pos = 0;

    while pos < data.len() {
        if let Some(found_pos) = memchr2(b'\r', b'\n', &data[pos..]) {
            let actual_pos = pos + found_pos;

            match data[actual_pos] {
                b'\r' => {
                    if actual_pos + 1 < data.len() && data[actual_pos + 1] == b'\n' {
                        // CRLF
                        stats.crlf_count += 1;
                        pos = actual_pos + 2;
                    } else {
                        // Standalone CR
                        stats.cr_count += 1;
                        pos = actual_pos + 1;
                    }
                }
                b'\n' => {
                    // Standalone LF (not part of CRLF as we handle CRLF above)
                    stats.lf_count += 1;
                    pos = actual_pos + 1;
                }
                _ => unreachable!(),
            }
        } else {
            break;
        }
    }

    stats
}

/// Fast detection of line ending type by sampling
pub fn detect_line_ending_type(data: &[u8], sample_size: usize) -> LineEndingType {
    let sample_end = std::cmp::min(data.len(), sample_size);
    let sample = &data[..sample_end];

    let stats = analyze_line_endings(sample);
    stats.predominant_type()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crlf_to_lf_conversion() {
        let mut converter = LineEndingConverter::new(false, true);
        let input = b"Hello\r\nWorld\r\nTest\r\n";
        let result = converter.convert(input).unwrap();
        assert_eq!(result, b"Hello\nWorld\nTest\n");
    }

    #[test]
    fn test_lf_to_crlf_conversion() {
        let mut converter = LineEndingConverter::new(true, false);
        let input = b"Hello\nWorld\nTest\n";
        let result = converter.convert(input).unwrap();
        assert_eq!(result, b"Hello\r\nWorld\r\nTest\r\n");
    }

    #[test]
    fn test_mixed_line_endings() {
        let mut converter = LineEndingConverter::new(false, true);
        let input = b"Hello\r\nWorld\nTest\rEnd\r\n";
        let result = converter.convert(input).unwrap();
        assert_eq!(result, b"Hello\nWorld\nTest\nEnd\n");
    }

    #[test]
    fn test_pending_cr_at_buffer_boundary() {
        let mut converter = LineEndingConverter::new(false, true);

        // First buffer ends with CR
        let buffer1 = b"Hello\r";
        let result1 = converter.convert(buffer1).unwrap();
        assert_eq!(result1, b"Hello");

        // Second buffer starts with LF
        let buffer2 = b"\nWorld";
        let result2 = converter.convert(buffer2).unwrap();
        assert_eq!(result2, b"\nWorld");
    }

    #[test]
    fn test_standalone_cr_at_boundary() {
        let mut converter = LineEndingConverter::new(false, true);

        // First buffer ends with CR
        let buffer1 = b"Hello\r";
        let result1 = converter.convert(buffer1).unwrap();
        assert_eq!(result1, b"Hello");

        // Second buffer doesn't start with LF
        let buffer2 = b"World";
        let result2 = converter.convert(buffer2).unwrap();
        assert_eq!(result2, b"\nWorld");
    }

    #[test]
    fn test_line_ending_analysis() {
        let data = b"Line1\nLine2\r\nLine3\rLine4\r\n";
        let stats = analyze_line_endings(data);

        assert_eq!(stats.lf_count, 1);      // Line1\n
        assert_eq!(stats.crlf_count, 2);   // Line2\r\n, Line4\r\n
        assert_eq!(stats.cr_count, 1);     // Line3\r

        // With 1 LF, 2 CRLF, 1 CR - CRLF should be predominant (2/4 = 50%)
        let predominant = stats.predominant_type();
        assert!(predominant == LineEndingType::Crlf || predominant == LineEndingType::Mixed);
        assert!(!stats.is_consistent());
    }

    #[test]
    fn test_consistent_line_endings() {
        let data = b"Line1\nLine2\nLine3\n";
        let stats = analyze_line_endings(data);

        assert_eq!(stats.lf_count, 3);
        assert_eq!(stats.crlf_count, 0);
        assert_eq!(stats.cr_count, 0);

        assert_eq!(stats.predominant_type(), LineEndingType::Lf);
        assert!(stats.is_consistent());
    }

    #[test]
    fn test_no_line_endings() {
        let data = b"Single line with no ending";
        let stats = analyze_line_endings(data);

        assert_eq!(stats.lf_count, 0);
        assert_eq!(stats.crlf_count, 0);
        assert_eq!(stats.cr_count, 0);

        assert_eq!(stats.predominant_type(), LineEndingType::Lf); // Default
        assert!(stats.is_consistent());
    }
}
