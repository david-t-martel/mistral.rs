//! File operations module.
//!
//! Implements core file system operations:
//! - cat: Concatenate and display files
//! - cp: Copy files and directories
//! - dd: Convert and copy files with block-level control
//! - dir: List directory contents (DOS-style)
//! - ln: Create links
//! - ls: List directory contents (Unix-style)
//! - mkdir: Create directories
//! - mv: Move/rename files
//! - rm: Remove files
//! - rmdir: Remove directories
//! - touch: Update file timestamps
//! - vdir: Verbose directory listing

mod cat;
mod ls;
// TODO @gemini: Implement remaining file operations
// mod cp;
// mod mv;
// mod rm;
// mod mkdir;
// mod touch;
// mod dd;
// mod ln;
// mod dir;
// mod rmdir;
// mod vdir;

pub use cat::cat;
pub use ls::{format_size, ls};
