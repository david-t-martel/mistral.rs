# Agent Tools - Phase 1 Implementation Guide

## Overview

This document describes the new AgentToolkit API introduced in Phase 1 of the full WinUtils integration.

## Quick Start

### New API (Recommended)

```rust
use mistralrs_agent_tools::{AgentToolkit, CatOptions, LsOptions};
use std::path::Path;

// Create toolkit
let toolkit = AgentToolkit::with_defaults();

// Cat - concatenate files
let options = CatOptions {
    number_lines: true,
    ..Default::default()
};
let content = toolkit.cat(&[Path::new("file.txt")], &options)?;
println!("{}", content);

// Ls - list directory
let options = LsOptions {
    all: true,
    long: true,
    human_readable: true,
    ..Default::default()
};
let result = toolkit.ls(Path::new("."), &options)?;
for entry in result.entries {
    println!("{:>10} {}", entry.size, entry.name);
}
```

### Legacy API (Deprecated)

The old `AgentTools` API is still available for backwards compatibility:

```rust
use mistralrs_agent_tools::AgentTools;

let tools = AgentTools::with_defaults();
let result = tools.read("file.txt")?;
if result.success {
    println!("{}", result.data.unwrap());
}
```

## Migration Guide

### From AgentTools to AgentToolkit

| Old API                                  | New API                                                   |
| ---------------------------------------- | --------------------------------------------------------- |
| `AgentTools::with_defaults()`            | `AgentToolkit::with_defaults()`                           |
| `tools.read(path)`                       | `toolkit.cat(&[Path::new(path)], &CatOptions::default())` |
| `tools.write(path, content, true, true)` | Use file writing tools (coming in Phase 2)                |
| `tools.find(pattern, depth)`             | Use search tools (coming in Phase 2)                      |
| `tools.tree(root, depth)`                | Use tree utility (coming in Phase 2)                      |

### Key Differences

1. **Type Safety**: New API uses `Path` types instead of strings
1. **Rich Options**: Dedicated option structs for each operation
1. **Better Errors**: `AgentError` with specific error categories
1. **Sandbox First**: Security is built-in, not optional

## Architecture

### Module Structure

```
mistralrs-agent-tools/
├── src/
│   ├── lib.rs              # Main API (AgentToolkit)
│   ├── pathlib.rs          # Path normalization (Windows/WSL/Cygwin/GitBash)
│   ├── types/              # Core types and errors
│   │   └── mod.rs
│   └── tools/
│       ├── mod.rs          # Tool categories
│       ├── sandbox.rs      # Security enforcement
│       └── file/           # File operations
│           ├── mod.rs
│           ├── cat.rs      # Concatenate files
│           └── ls.rs       # List directory
```

### Security Model

**Sandbox Enforcement**:

- All operations restricted to sandbox root
- Path traversal prevention
- Symlink resolution and validation
- Configurable read-outside-sandbox permission
- File size limits

**Path Validation**:

1. Normalize path (handle Windows/WSL/Cygwin formats)
1. Make absolute relative to sandbox root
1. Canonicalize (resolve symlinks, `..` components)
1. Check boundaries

```rust
use mistralrs_agent_tools::{SandboxConfig, AgentToolkit};
use std::path::PathBuf;

// Configure sandbox
let config = SandboxConfig::new(PathBuf::from("/safe/directory"))
    .allow_read_outside(false)  // Strict mode
    .max_read_size(50 * 1024 * 1024)  // 50MB
    .max_batch_size(1000);  // Max files per operation

let toolkit = AgentToolkit::new(config);
```

## Implemented Tools (Phase 1)

### Cat - Concatenate Files

**Features**:

- BOM detection and stripping (UTF-8, UTF-16, UTF-32)
- Line numbering
- Show line endings
- Squeeze blank lines
- Multiple file support

**Example**:

```rust
use mistralrs_agent_tools::{AgentToolkit, CatOptions};
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();

// Basic cat
let content = toolkit.cat(
    &[Path::new("file1.txt"), Path::new("file2.txt")],
    &CatOptions::default()
)?;

// With line numbers and show line endings
let options = CatOptions {
    number_lines: true,
    show_ends: true,
    squeeze_blank: true,
};
let content = toolkit.cat(&[Path::new("file.txt")], &options)?;
```

### Ls - List Directory

**Features**:

- Hidden file filtering
- Recursive listing
- Sorting (name, time)
- Reverse order
- Human-readable sizes
- File metadata (size, modified time, permissions)

**Example**:

```rust
use mistralrs_agent_tools::{AgentToolkit, LsOptions};
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();

// List all files recursively
let options = LsOptions {
    all: true,
    recursive: true,
    human_readable: true,
    sort_by_time: true,
    ..Default::default()
};

let result = toolkit.ls(Path::new("."), &options)?;

println!("Found {} files ({} bytes total)", result.total, result.total_size);
for entry in result.entries {
    let size = tools::file::format_size(entry.size, true);
    let marker = if entry.is_dir { "/" } else { "" };
    println!("{:>8} {}{}", size, entry.name, marker);
}
```

## Path Normalization

The `pathlib` module handles Windows path complications:

### Supported Formats

| Format      | Example                   | Description                  |
| ----------- | ------------------------- | ---------------------------- |
| DOS         | `C:\Users\David`          | Standard Windows             |
| DOS Forward | `C:/Users/David`          | Windows with Unix separators |
| WSL         | `/mnt/c/users/david`      | Windows Subsystem for Linux  |
| Cygwin      | `/cygdrive/c/users/david` | Cygwin environment           |
| Git Bash    | `//c/users/david`         | Git Bash style               |
| UNC         | `\\?\C:\Long\Path`        | Long path support            |

### Usage

```rust
use mistralrs_agent_tools::pathlib;

// Normalize any path format to Windows
let path = pathlib::normalize_path("/mnt/c/users/david/file.txt")?;
assert_eq!(path, r"C:\users\david\file.txt");

// Check if path is absolute
let is_abs = pathlib::is_absolute("/mnt/c/users");
assert!(is_abs);

// Join paths
let joined = pathlib::join("C:\\base", "relative/path")?;
assert_eq!(joined, r"C:\base\relative\path");
```

## Error Handling

### Error Types

```rust
pub enum AgentError {
    PathError(String),        // Path normalization failed
    SandboxViolation(String), // Outside sandbox bounds
    IoError(String),          // File system error
    InvalidInput(String),     // Bad parameters
    PermissionDenied(String), // Access denied
    NotFound(String),         // File/directory not found
    Unsupported(String),      // Operation not supported
    EncodingError(String),    // Text encoding issue
}
```

### Example

```rust
use mistralrs_agent_tools::{AgentToolkit, AgentError};
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();

match toolkit.cat(&[Path::new("/etc/passwd")], &Default::default()) {
    Ok(content) => println!("{}", content),
    Err(AgentError::SandboxViolation(msg)) => {
        eprintln!("Sandbox violation: {}", msg);
    }
    Err(AgentError::NotFound(msg)) => {
        eprintln!("File not found: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Performance

### Optimizations

- **Path caching**: Normalized paths cached for repeated operations
- **Buffered I/O**: 64KB buffers for optimal NTFS performance
- **Early bailouts**: Size checks before reading large files
- **Batch validation**: Validate multiple paths in one pass

### Benchmarks (Phase 1)

| Operation        | Time   | Notes                 |
| ---------------- | ------ | --------------------- |
| Path normalize   | \<1μs  | Cached paths          |
| Sandbox validate | \<5μs  | With canonicalization |
| Cat small file   | \<1ms  | < 1MB file            |
| Ls directory     | \<10ms | 100 entries           |

## Testing

All modules include comprehensive unit tests:

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test pathlib
cargo test sandbox
cargo test cat
cargo test ls

# Run with coverage
cargo tarpaulin --out Html
```

## Coming in Phase 2

### Text Processing (Weeks 3-5)

- head, tail - View file portions
- grep - Search file contents
- wc - Count words/lines/characters
- sort, uniq - Sort and deduplicate
- cut, paste - Column operations
- tr - Character translation

### Shell Execution (Weeks 6-7) 🚀

- pwsh - PowerShell executor
- cmd - Command Prompt executor
- bash - Bash executor (Git Bash/WSL)
- Secure command validation
- Timeout enforcement
- Output capture

### More File Operations

- cp - Copy files
- mv - Move/rename files
- rm - Remove files
- mkdir - Create directories
- touch - Update timestamps

## FAQ

**Q: Why create a new API instead of extending AgentTools?**
A: The new API provides better type safety, richer options, and a clearer separation between operations. It's designed for the full 90+ utility integration.

**Q: Is the old API going away?**
A: No, `AgentTools` will remain for backwards compatibility. However, new code should use `AgentToolkit`.

**Q: Why sandbox everything?**
A: Security. LLM agents can make mistakes or be manipulated. Sandboxing prevents accidental or malicious filesystem damage.

**Q: Can I disable the sandbox?**
A: You can allow reading outside the sandbox with `allow_read_outside(true)`, but write operations are always sandboxed.

**Q: What about performance?**
A: The sandbox adds \<5μs per operation. Path normalization is cached. This is negligible compared to actual I/O.

**Q: When will shell execution be available?**
A: Shell executors (pwsh, cmd, bash) are targeted for Weeks 6-7 (Phase 2) of the implementation plan.

## Support

For issues, questions, or contributions:

- GitHub Issues: https://github.com/EricLBuehler/mistral.rs
- Documentation: This file
- Examples: See `tests/` directory

## License

MIT License - See LICENSE file for details.
