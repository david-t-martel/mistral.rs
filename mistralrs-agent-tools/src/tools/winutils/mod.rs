//! Windows utilities integration module
//!
//! This module provides high-level wrappers around the winutils coreutils
//! executables, enabling agents to use battle-tested Windows-optimized
//! command-line utilities with a clean Rust API.
//!
//! The winutils project provides 90+ utilities optimized for Windows,
//! including: cut, tr, base64, comm, join, split, expand, unexpand,
//! fold, fmt, nl, od, pr, ptx, shuf, tac, seq, and many more.

mod wrapper;

pub use wrapper::{winutil_exec, WinutilCommand};

// Re-export common utilities
pub mod encoding;
pub mod fileops;
pub mod text;

pub use encoding::{base32, base64, basenc};
pub use fileops::{cp, mkdir, mv, rm, rmdir, touch};
pub use text::{cut, expand, fmt, fold, nl, tac, tr, unexpand};
