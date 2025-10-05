# Winutils Integration Guide

## Overview

The `mistralrs-agent-tools` crate now provides seamless integration with the **winutils** project, giving agents access to 90+ battle-tested, Windows-optimized coreutils executables through a clean Rust API.

## Architecture

### Integration Approach

Instead of reimplementing all 90+ utilities, we leverage the existing winutils executables through a wrapper system:

```
mistralrs-agent-tools
├── Native Rust Implementations (Phase 1-3)
│   ├── cat, ls, head, tail, wc
│   ├── grep, sort, uniq
│   └── Shell executor (PowerShell, Cmd, Bash)
└── Winutils Wrappers (New)
    ├── Text processing: cut, tr, expand, unexpand, fold, fmt, nl, tac
    ├── Encoding: base64, base32, basenc
    ├── File ops: cp, mv, rm, mkdir, rmdir, touch
    └── 80+ more utilities available via WinutilCommand
```

### Key Components

1. **WinutilCommand** - Builder pattern for constructing commands
1. **winutil_exec()** - Core executor that validates paths and runs executables
1. **Category Modules** - Organized wrappers (text, encoding, fileops)
1. **AgentToolkit Methods** - High-level API for common operations

## Installation

### Prerequisites

1. **Build winutils executables:**

   ```powershell
   cd T:\projects\coreutils\winutils
   cargo build --release
   ```

1. **Verify executables exist:**

   ```powershell
   ls T:\projects\coreutils\winutils\target\release\*.exe | measure
   ```

   You should see 90+ executables.

### Path Configuration

By default, winutils executables are expected at:

```
T:\projects\coreutils\winutils\target\release\
```

To use a different path:

```rust
use mistralrs_agent_tools::tools::winutils::WinutilCommand;

let cmd = WinutilCommand::new("base64")
    .arg("file.txt")
    .winutils_path(PathBuf::from("C:\\custom\\path\\to\\winutils"))
    .execute(&sandbox)?;
```

## Usage

### Method 1: AgentToolkit (Recommended)

The easiest way to use winutils is through the `AgentToolkit`:

```rust
use mistralrs_agent_tools::{AgentToolkit, SandboxConfig};
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();

// Text processing
let output = toolkit.cut(&[Path::new("data.csv")], "1,3,5", Some(','))?;
let reversed = toolkit.tac(&[Path::new("file.txt")])?;
let numbered = toolkit.nl(&[Path::new("file.txt")], Some(1))?;

// Encoding
let encoded = toolkit.base64_util(Path::new("file.txt"), false)?;
let decoded = toolkit.base64_util(Path::new("encoded.txt"), true)?;

// File operations
toolkit.cp_util(Path::new("source.txt"), Path::new("dest.txt"), false)?;
toolkit.mkdir_util(Path::new("newdir"), true)?;
toolkit.touch_util(Path::new("newfile.txt"))?;
```

### Method 2: Direct Function Calls

Use module functions directly for more control:

```rust
use mistralrs_agent_tools::tools::winutils;
use mistralrs_agent_tools::{Sandbox, SandboxConfig};
use std::path::Path;

let sandbox = Sandbox::new(SandboxConfig::default())?;

// Text processing
let output = winutils::text::cut(&sandbox, &[Path::new("file.csv")], "1,2", Some(','))?;
let translated = winutils::text::tr(&sandbox, &[Path::new("file.txt")], "a-z", Some("A-Z"))?;
let expanded = winutils::text::expand(&sandbox, &[Path::new("file.txt")], Some(4))?;

// Encoding
let encoded = winutils::encoding::base64(&sandbox, Path::new("file.txt"), false)?;

// File operations
winutils::fileops::cp(&sandbox, Path::new("src"), Path::new("dst"), true)?;
```

### Method 3: WinutilCommand Builder

For utilities not wrapped, use the command builder:

```rust
use mistralrs_agent_tools::tools::winutils::WinutilCommand;

// Run any winutils utility
let result = WinutilCommand::new("seq")
    .arg("1")
    .arg("10")
    .execute(&sandbox)?;

println!("{}", result.stdout);

// Run with custom working directory
let result = WinutilCommand::new("du")
    .arg("-sh")
    .arg(".")
    .working_dir(PathBuf::from("C:\\Projects"))
    .execute(&sandbox)?;
```

## Available Utilities

### Text Processing (`tools::winutils::text`)

| Utility  | Function     | Description                    |
| -------- | ------------ | ------------------------------ |
| cut      | `cut()`      | Extract fields from lines      |
| tr       | `tr()`       | Translate or delete characters |
| expand   | `expand()`   | Convert tabs to spaces         |
| unexpand | `unexpand()` | Convert spaces to tabs         |
| fold     | `fold()`     | Wrap lines to width            |
| fmt      | `fmt()`      | Simple text formatter          |
| nl       | `nl()`       | Number lines                   |
| tac      | `tac()`      | Reverse lines (reverse cat)    |

### Encoding (`tools::winutils::encoding`)

| Utility | Function   | Description               |
| ------- | ---------- | ------------------------- |
| base64  | `base64()` | Base64 encode/decode      |
| base32  | `base32()` | Base32 encode/decode      |
| basenc  | `basenc()` | Multiple encoding support |

### File Operations (`tools::winutils::fileops`)

| Utility | Function  | Description                    |
| ------- | --------- | ------------------------------ |
| cp      | `cp()`    | Copy files and directories     |
| mv      | `mv()`    | Move/rename files              |
| rm      | `rm()`    | Remove files and directories   |
| mkdir   | `mkdir()` | Create directories             |
| rmdir   | `rmdir()` | Remove empty directories       |
| touch   | `touch()` | Update timestamps/create files |

### 80+ More Utilities Available via WinutilCommand

All winutils coreutils can be executed using `WinutilCommand`:

- **Text**: comm, join, split, csplit, paste, pr, ptx, shuf
- **Math**: seq, factor, expr, numfmt
- **File info**: du, df, stat, sum, cksum, hashsum
- **Path**: basename, dirname, realpath, readlink
- **System**: whoami, hostname, arch, env, printenv, pwd
- **Misc**: yes, true, false, test, sleep, sync, date

## Examples

### CSV Processing Pipeline

```rust
let toolkit = AgentToolkit::with_defaults();

// Extract specific columns
let data = toolkit.cut(&[Path::new("data.csv")], "1,3,5", Some(','))?;

// Write to temp file
std::fs::write("temp.csv", data)?;

// Sort and remove duplicates (using native implementations)
let sorted = toolkit.sort(&[Path::new("temp.csv")], &SortOptions::default())?;
std::fs::write("sorted.csv", sorted)?;
let unique = toolkit.uniq(&[Path::new("sorted.csv")], &UniqOptions::default())?;

println!("{}", unique);
```

### File Operations

```rust
let toolkit = AgentToolkit::with_defaults();

// Create directory structure
toolkit.mkdir_util(Path::new("project/src"), true)?;
toolkit.mkdir_util(Path::new("project/tests"), true)?;

// Create files
toolkit.touch_util(Path::new("project/src/main.rs"))?;
toolkit.touch_util(Path::new("project/Cargo.toml"))?;

// Copy template
toolkit.cp_util(
    Path::new("templates/lib.rs"),
    Path::new("project/src/lib.rs"),
    false
)?;
```

### Text Transformation

```rust
// Convert file to uppercase
let uppercase = toolkit.tr(
    &[Path::new("file.txt")],
    "a-z",
    Some("A-Z")
)?;

// Number the lines
let numbered = toolkit.nl(&[Path::new("file.txt")], Some(1))?;

// Reverse line order
let reversed = toolkit.tac(&[Path::new("file.txt")])?;

// Expand tabs to 4 spaces
let expanded = toolkit.expand(&[Path::new("file.txt")], Some(4))?;
```

### Encoding/Decoding

```rust
// Encode file to base64
let encoded = toolkit.base64_util(Path::new("image.png"), false)?;
std::fs::write("image.b64", encoded)?;

// Decode back
let decoded = toolkit.base64_util(Path::new("image.b64"), true)?;
std::fs::write("image_decoded.png", decoded)?;
```

## Security

All winutils operations are subject to the same sandbox constraints:

- **Read validation**: All input files must be within sandbox
- **Write validation**: All output paths must be within sandbox
- **Timeout protection**: 60-second default timeout for all commands
- **Error handling**: Non-zero exit codes converted to AgentError

## Performance

### Native vs Winutils

| Operation               | Native Rust | Winutils | Notes                               |
| ----------------------- | ----------- | -------- | ----------------------------------- |
| cat, ls, head, tail, wc | ✅          | ⚠️       | Use native implementations (faster) |
| grep, sort, uniq        | ✅          | ⚠️       | Use native implementations (faster) |
| cut, tr, expand, tac    | ❌          | ✅       | Use winutils (only option)          |
| base64, base32          | ❌          | ✅       | Use winutils (only option)          |
| cp, mv, rm, mkdir       | ❌          | ✅       | Use winutils (only option)          |

### Optimization Tips

1. **Prefer native implementations** when available (grep, sort, wc, etc.)
1. **Batch operations** when possible to reduce process spawning overhead
1. **Use WinutilCommand** directly for one-off utilities to avoid function call overhead
1. **Consider async** for multiple independent winutils operations

## Error Handling

```rust
use mistralrs_agent_tools::types::AgentError;

match toolkit.cut(&[path], "1,2", Some(',')) {
    Ok(output) => println!("{}", output),
    Err(AgentError::NotFound(msg)) => {
        eprintln!("Winutils not found: {}", msg);
    }
    Err(AgentError::SandboxViolation(msg)) => {
        eprintln!("Security error: {}", msg);
    }
    Err(AgentError::IoError(msg)) => {
        eprintln!("Command failed: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Troubleshooting

### Executable Not Found

```
Error: Winutils executable not found: T:\projects\coreutils\winutils\target\release\cut.exe
```

**Solution:** Build winutils or specify correct path:

```rust
let cmd = WinutilCommand::new("cut")
    .winutils_path(PathBuf::from("your\\path\\here"))
    ...
```

### Sandbox Violations

```
Error: Sandbox violation: Path outside sandbox
```

**Solution:** Ensure files are within sandbox root or adjust sandbox config:

```rust
let config = SandboxConfig::new(PathBuf::from("C:\\Projects"))
    .allow_read_outside(true); // Allow reading outside sandbox
```

### Command Timeout

```
Error: Command timed out after 60 seconds
```

**Solution:** Use WinutilCommand with custom timeout (not currently exposed, file an issue).

## Future Enhancements

- [ ] Async winutils execution with tokio
- [ ] Streaming output for large files
- [ ] Configurable timeouts per utility
- [ ] Progress callbacks for long operations
- [ ] Batch execution API for multiple commands
- [ ] Winutils auto-discovery (check PATH)

## Comparison with Native Implementations

| Feature                  | Native (grep, sort, etc.) | Winutils                     |
| ------------------------ | ------------------------- | ---------------------------- |
| **Speed**                | Fastest (in-process)      | Fast (subprocess)            |
| **Memory**               | Efficient                 | Moderate                     |
| **Features**             | Core features             | Full coreutils compatibility |
| **Dependencies**         | Self-contained            | Requires winutils build      |
| **Windows optimization** | ✅                        | ✅✅ (optimized for Windows) |

## Conclusion

The winutils integration provides mistralrs agents with:

- **90+ utilities** covering text processing, file operations, encoding, and more
- **Clean Rust API** with type safety and error handling
- **Sandbox security** for all operations
- **Battle-tested** implementations from the winutils project
- **Windows-optimized** for best performance on Windows

Use native implementations (grep, sort, wc) when available for best performance, and winutils wrappers for everything else.
