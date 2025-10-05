//! Byte Order Mark (BOM) detection and handling
//!
//! This module provides utilities for detecting and handling different types of BOMs
//! commonly found in text files on Windows systems.

use std::fmt;

/// Different types of Byte Order Marks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BomType {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

impl fmt::Display for BomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BomType::None => write!(f, "None"),
            BomType::Utf8 => write!(f, "UTF-8"),
            BomType::Utf16Le => write!(f, "UTF-16 LE"),
            BomType::Utf16Be => write!(f, "UTF-16 BE"),
            BomType::Utf32Le => write!(f, "UTF-32 LE"),
            BomType::Utf32Be => write!(f, "UTF-32 BE"),
        }
    }
}

/// Information about a detected BOM
#[derive(Debug, Clone)]
pub struct BomInfo {
    pub bom_type: BomType,
    pub bom_length: usize,
}

impl BomInfo {
    pub fn new(bom_type: BomType, bom_length: usize) -> Self {
        Self { bom_type, bom_length }
    }

    /// Detect BOM from the beginning of file data
    pub fn detect(data: &[u8]) -> Self {
        // UTF-32 BOMs (4 bytes) - check first to avoid false positives
        if data.len() >= 4 {
            if data.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
                return BomInfo::new(BomType::Utf32Be, 4);
            }
            if data.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
                return BomInfo::new(BomType::Utf32Le, 4);
            }
        }

        // UTF-16 BOMs (2 bytes)
        if data.len() >= 2 {
            if data.starts_with(&[0xFF, 0xFE]) {
                return BomInfo::new(BomType::Utf16Le, 2);
            }
            if data.starts_with(&[0xFE, 0xFF]) {
                return BomInfo::new(BomType::Utf16Be, 2);
            }
        }

        // UTF-8 BOM (3 bytes)
        if data.len() >= 3 && data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return BomInfo::new(BomType::Utf8, 3);
        }

        BomInfo::new(BomType::None, 0)
    }

    /// Get the BOM bytes for a given type
    pub fn get_bom_bytes(bom_type: BomType) -> &'static [u8] {
        match bom_type {
            BomType::None => &[],
            BomType::Utf8 => &[0xEF, 0xBB, 0xBF],
            BomType::Utf16Le => &[0xFF, 0xFE],
            BomType::Utf16Be => &[0xFE, 0xFF],
            BomType::Utf32Le => &[0xFF, 0xFE, 0x00, 0x00],
            BomType::Utf32Be => &[0x00, 0x00, 0xFE, 0xFF],
        }
    }

    /// Check if this BOM indicates a Unicode encoding
    pub fn is_unicode(&self) -> bool {
        !matches!(self.bom_type, BomType::None)
    }

    /// Get the expected encoding for this BOM
    pub fn encoding_name(&self) -> &'static str {
        match self.bom_type {
            BomType::None => "Unknown",
            BomType::Utf8 => "UTF-8",
            BomType::Utf16Le => "UTF-16LE",
            BomType::Utf16Be => "UTF-16BE",
            BomType::Utf32Le => "UTF-32LE",
            BomType::Utf32Be => "UTF-32BE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_bom_detection() {
        let data = [0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf8);
        assert_eq!(bom_info.bom_length, 3);
    }

    #[test]
    fn test_utf16le_bom_detection() {
        let data = [0xFF, 0xFE, b'H', 0x00, b'e', 0x00];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf16Le);
        assert_eq!(bom_info.bom_length, 2);
    }

    #[test]
    fn test_utf16be_bom_detection() {
        let data = [0xFE, 0xFF, 0x00, b'H', 0x00, b'e'];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf16Be);
        assert_eq!(bom_info.bom_length, 2);
    }

    #[test]
    fn test_utf32le_bom_detection() {
        let data = [0xFF, 0xFE, 0x00, 0x00, b'H', 0x00, 0x00, 0x00];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf32Le);
        assert_eq!(bom_info.bom_length, 4);
    }

    #[test]
    fn test_utf32be_bom_detection() {
        let data = [0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, b'H'];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf32Be);
        assert_eq!(bom_info.bom_length, 4);
    }

    #[test]
    fn test_no_bom_detection() {
        let data = [b'H', b'e', b'l', b'l', b'o'];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::None);
        assert_eq!(bom_info.bom_length, 0);
    }

    #[test]
    fn test_partial_bom_data() {
        // Only one byte - should not detect UTF-8 BOM
        let data = [0xEF];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::None);
    }

    #[test]
    fn test_bom_priority() {
        // UTF-32 LE BOM should be detected, not UTF-16 LE
        let data = [0xFF, 0xFE, 0x00, 0x00, b'H', 0x00, 0x00, 0x00];
        let bom_info = BomInfo::detect(&data);
        assert_eq!(bom_info.bom_type, BomType::Utf32Le);
        assert_eq!(bom_info.bom_length, 4);
    }
}
