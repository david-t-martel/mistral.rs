//! Text processing module.
//!
//! Implements text manipulation utilities:
//! - base32, base64, basenc: Encoding/decoding
//! - comm, join: File comparison and joining
//! - csplit, split: File splitting
//! - cut, paste: Column extraction and merging
//! - expand, unexpand: Tab/space conversion
//! - fold, fmt: Text formatting
//! - head, tail: Display file beginning/end
//! - nl: Number lines
//! - od: Octal dump
//! - pr: Format for printing
//! - ptx: Permuted index
//! - shuf: Random permutation
//! - sort: Sort lines
//! - tac: Reverse concatenate
//! - tr: Character translation
//! - tsort: Topological sort
//! - uniq: Report/filter repeated lines

// Implemented utilities
mod grep;
mod head;
mod sort;
mod tail;
mod uniq;
mod wc;

// TODO @codex: Implement remaining text processing utilities
// mod cut;
// mod tr;
// mod base64, base32, basenc (encoding)
// mod comm, join (comparison)
// mod csplit, split (splitting)
// mod cut, paste (columns)
// mod expand, unexpand (tabs)
// mod fold, fmt (formatting)
// mod nl (numbering)
// mod od (octal)
// mod pr, ptx (printing)
// mod shuf, tac (ordering)
// mod tr, tsort (translation/topo)

pub use grep::grep;
pub use head::head;
pub use sort::sort;
pub use tail::tail;
pub use uniq::uniq;
pub use wc::{format_wc_output, wc};
