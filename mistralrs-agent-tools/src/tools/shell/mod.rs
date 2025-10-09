//! Shell execution module.
//!
//! Provides secure shell command execution with multiple backends:
//! - executor: Core execution engine with timeout and sandbox
//! - pwsh: PowerShell executor
//! - cmd: Command Prompt executor
//! - bash: Bash executor (Git Bash/WSL/MSYS2)
//!
//! This is the most powerful module, enabling system automation,
//! build processes, DevOps tasks, and process management.

mod executor;

// TODO @gemini: Implement specialized shell executors (future enhancement)
// mod pwsh;
// mod cmd;
// mod bash;
// mod path_translation;

pub use executor::execute;
