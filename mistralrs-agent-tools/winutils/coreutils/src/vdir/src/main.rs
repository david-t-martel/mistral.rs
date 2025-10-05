//! Windows-optimized vdir utility
//!
//! List directory contents verbosely
//!
//! This is a Windows-optimized version of the vdir utility from uutils/coreutils.
//! It provides enhanced Windows path handling, performance optimizations,
//! and Windows-specific features while maintaining compatibility with the original.

use std::env;
use std::ffi::OsString;
use std::process;
use winpath::normalize_path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let processed_args = process_windows_args(args);

    // For now, delegate to the original uutils implementation with path normalization
    let exit_code = uu_vdir::uumain(processed_args.into_iter().map(OsString::from));
    process::exit(exit_code);
}

/// Windows-optimized argument processing
fn process_windows_args(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .map(|arg| {
            // Normalize Windows paths in arguments
            if arg.contains('\\') || arg.contains('/') {
                match normalize_path(&arg) {
                    Ok(normalized) => normalized,
                    Err(_) => arg, // Keep original if normalization fails
                }
            } else {
                arg
            }
        })
        .collect()
}
