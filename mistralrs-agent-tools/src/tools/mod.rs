//! Agent tools organized by category.
//!
//! This module contains all utility implementations organized into logical categories:
//! - file: File operations (cat, cp, mv, rm, ls, mkdir, etc.)
//! - text: Text processing (head, tail, sort, grep, cut, etc.)
//! - analysis: File analysis (wc, du, hashsum, etc.)
//! - path: Path operations (basename, dirname, realpath, etc.)
//! - system: System information (hostname, arch, env, etc.)
//! - output: Output utilities (echo, printf, yes, etc.)
//! - security: File security (shred, mktemp, etc.)
//! - numeric: Numeric operations (seq, factor, expr, etc.)
//! - testing: Testing utilities (test, sleep)
//! - search: Search tools (find, grep, tree, which, where)
//! - shell: Shell execution (pwsh, cmd, bash)

pub mod analysis;
pub mod file;
pub mod numeric;
pub mod output;
pub mod path;
pub mod search;
pub mod security;
pub mod shell;
pub mod system;
pub mod testing;
pub mod text;
pub mod winutils;

// Sandbox enforcement for all tools
pub mod sandbox;
