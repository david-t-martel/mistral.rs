# WinUtils Framework Architecture

## Overview

The WinUtils framework is a comprehensive collection of Windows-optimized Unix-like utilities implemented in Rust. It contains 95+ coreutils plus enhanced utilities with Windows-specific path handling and performance optimizations.

## Directory Structure

````
mistralrs-agent-tools/winutils/
├── Cargo.toml                 # Workspace root with 95+ member crates
├── shared/                    # Core shared libraries
│   ├── winpath/              # Path normalization and Git Bash/WSL conversion
│   └── winutils-core/        # Common utilities (help, version, diagnostics)
├── coreutils/src/            # 90+ GNU coreutils implementations
│   ├── cat/                  # Concatenate files (BOM, line endings, Windows FS)
│   ├── ls/                   # List directory contents
│   ├── cp/                   # Copy files (Windows attributes, junctions)
│   ├── mv/                   # Move files
│   ├── rm/                   # Remove files
│   ├── mkdir/                # Create directories
│   ├── head/                 # Output first part of files
│   ├── tail/                 # Output last part of files
│   ├── wc/                   # Word count
│   ├── grep/                 # Pattern search (via grep-wrapper)
│   ├── find/                 # File search (via find-wrapper)
│   ├── touch/                # Create/update file timestamps
│   ├── basename/             # Strip directory from filename
│   ├── dirname/              # Output directory name
│   ├── pwd/                  # Print working directory
│   ├── echo/                 # Display text
│   ├── env/                  # Display/set environment
│   ├── date/                 # Display/set date
│   ├── hostname/             # Show/set hostname
│   ├── whoami/               # Print username
│   └── ... (85+ more utilities)
├── derive-utils/             # Enhanced wrappers and custom tools
│   ├── where/                # Windows 'where' command (PATH search)
│   ├── which/                # Unix 'which' command
│   ├── tree/                 # Directory tree visualization
│   ├── find-wrapper/         # fd-powered find with winpath
│   ├── grep-wrapper/         # ripgrep-powered grep with winpath
│   └── shell wrappers/       # cmd, pwsh, bash wrappers
└── benchmarks/               # Performance testing framework

## Core Dependencies

### 1. winpath (Critical Path)

**Purpose**: Cross-platform path normalization and conversion

**Features**:
- Windows path normalization (backslash to forward slash)
- Git Bash path conversion (`/c/path` ↔ `C:\path`)
- WSL path conversion (`/mnt/c/path` ↔ `C:\path`)
- Path caching for performance
- Unicode path support

**API**:
```rust
pub fn normalize_path(path: &str) -> Result<String, PathError>
pub fn to_windows_path(path: &str) -> Result<String, PathError>
pub fn from_windows_path(path: &str) -> Result<String, PathError>
````

**Dependencies**: ALL utilities depend on this

### 2. winutils-core

**Purpose**: Shared functionality across utilities

**Features**:

- `help`: Standardized help text generation
- `version`: Version information with Git integration
- `testing`: Test framework helpers
- `windows-enhanced`: Windows-specific enhancements
- `diagnostics`: Performance and debug diagnostics

**API**:

```rust
pub fn print_help(app_name: &str, description: &str, options: &[Option])
pub fn print_version(app_name: &str, version: &str)
pub trait WindowsEnhanced { fn with_windows_attrs() -> Self; }
```

## Utility Categories

### File Operations (Priority for Agent Tools)

1. **cat** - Read and display files

   - BOM detection/stripping
   - Line ending conversion (CRLF ↔ LF)
   - Windows file attributes
   - Memory-mapped I/O for performance

1. **ls** - List directory contents

   - Windows attributes (hidden, system, readonly)
   - Color output support
   - Long format with permissions
   - Sorting options

1. **cp** - Copy files and directories

   - Windows file attributes preservation
   - Junction/symlink handling
   - Progress reporting
   - Sparse file support

1. **mv** - Move/rename files

   - Cross-volume moves
   - Windows attribute preservation
   - Atomic operations

1. **rm** - Remove files

   - Recursive deletion
   - Force deletion (readonly files)
   - Safety checks

1. **mkdir** - Create directories

   - Recursive creation (-p flag)
   - Permission setting

1. **touch** - Create/update timestamps

   - Access time, modification time
   - Create if not exists

### Text Processing (Priority for Agent Tools)

1. **head** - Display first N lines

   - Byte count mode
   - Multiple file support

1. **tail** - Display last N lines

   - Follow mode (-f)
   - Byte count mode

1. **wc** - Count lines, words, bytes

   - Character counting
   - Multiple file support

1. **cut** - Extract columns

   - Field/character selection
   - Delimiter support

1. **sort** - Sort lines

   - Numeric sort
   - Reverse sort
   - Unique lines

### Search Utilities (High Priority)

1. **find-wrapper** (via fd)

   - Fast file searching
   - Regex patterns
   - Type filtering
   - Gitignore support
   - Winpath integration

1. **grep-wrapper** (via ripgrep)

   - Fast content search
   - Regex patterns
   - Context lines
   - Winpath integration

1. **tree** - Directory tree

   - Recursive listing
   - Depth limits
   - Pattern filtering

1. **where** - Find executables in PATH

   - Windows PATHEXT support
   - Wildcard matching
   - Cache optimization

1. **which** - Unix-style command lookup

   - PATH searching
   - Alias resolution

### System Information

1. **pwd** - Print working directory
1. **hostname** - Show hostname
1. **whoami** - Show username
1. **env** - Show environment variables
1. **date** - Show/set date
1. **nproc** - Show CPU count

## Agent Tools Integration Strategy

### Phase 1: Core File Operations (Immediate)

Extract and wrap these utilities for LLM agent use:

1. **read_file** (cat) - Read file contents
1. **list_dir** (ls) - List directory
1. **search_files** (find-wrapper) - Find files
1. **search_content** (grep-wrapper) - Search in files
1. **file_info** (ls -l) - Get file metadata

### Phase 2: File Manipulation (Next)

6. **copy_file** (cp) - Copy files
1. **move_file** (mv) - Move/rename files
1. **delete_file** (rm) - Delete files
1. **create_dir** (mkdir) - Create directories
1. **touch_file** (touch) - Create/update timestamps

### Phase 3: Text Processing (Advanced)

11. **head_file** (head) - Get first N lines
01. **tail_file** (tail) - Get last N lines
01. **count_lines** (wc) - Count lines/words/chars
01. **sort_lines** (sort) - Sort content
01. **extract_columns** (cut) - Extract fields

### Phase 4: System Tools (Utility)

16. **get_pwd** (pwd) - Get current directory
01. **which_command** (which/where) - Find command path
01. **dir_tree** (tree) - Get directory tree
01. **get_env** (env) - Get environment variables
01. **sys_info** (hostname, whoami, nproc) - System info

## Simplification Strategy

### 1. Reduce Build Complexity

**Current**: 95+ member crates (slow builds, complex dependencies)

**Proposed**:

- Create a single `winutils-agent` crate
- Include only essential utilities (15-20 tools)
- Inline critical dependencies (winpath, winutils-core)
- Remove benchmarking framework
- Remove shell wrappers
- Remove GNU compatibility layers

### 2. Streamline Dependencies

**Remove**:

- `uucore` - Heavy GNU coreutils dependency
- `git2` - Version system integration (not needed)
- `criterion` - Benchmarking (not needed for agent)
- Shell wrappers - Agent doesn't need shell integration

**Keep**:

- `winpath` - Essential for path handling
- `clap` - CLI parsing (can be simplified to struct parsing)
- `walkdir` - Directory traversal
- `regex` - Pattern matching
- `serde`/`serde_json` - Serialization for tool schemas

### 3. API Simplification

**Current**: CLI-based utilities (spawn process, parse output)

**Proposed**: Direct Rust API

```rust
// Instead of:
let output = Command::new("cat").arg("file.txt").output()?;

// Use:
let content = agent_tools::read_file("file.txt")?;
```

### 4. Remove Windows-Specific Bloat

**Remove**:

- BOM handling (rarely needed)
- Line ending conversion (handle in agent layer)
- Windows attribute preservation (not critical for agent)
- Color output (not needed for programmatic use)
- Progress bars (not useful for LLM)

**Keep**:

- Path normalization (critical)
- Basic file operations (essential)
- Error handling (important)

## Refactoring Plan

### Step 1: Extract Core Path Library

Create `mistralrs-agent-tools/src/pathlib.rs`:

- Copy essential winpath functionality
- Remove caching (add later if needed)
- Remove Git Bash detection (simplify)
- Keep: normalize_path, is_absolute, join, parent

### Step 2: Create Utility Module Structure

```
mistralrs-agent-tools/src/
├── lib.rs              # Main API exports
├── pathlib.rs          # Path normalization
├── error.rs            # Unified error types
├── sandbox.rs          # Existing sandbox (keep as-is)
├── tools/              # New: utility implementations
│   ├── mod.rs          # Tool registry
│   ├── read.rs         # read_file (cat logic)
│   ├── list.rs         # list_dir (ls logic)
│   ├── find.rs         # search_files (find logic)
│   ├── grep.rs         # search_content (grep logic)
│   ├── copy.rs         # copy_file (cp logic)
│   ├── move.rs         # move_file (mv logic)
│   ├── delete.rs       # delete_file (rm logic)
│   ├── mkdir.rs        # create_dir (mkdir logic)
│   ├── touch.rs        # touch_file (touch logic)
│   ├── head.rs         # head_file (head logic)
│   ├── tail.rs         # tail_file (tail logic)
│   ├── wc.rs           # count_lines (wc logic)
│   ├── tree.rs         # dir_tree (tree logic)
│   ├── which.rs        # which_command (which/where logic)
│   └── sysinfo.rs      # System information
└── schemas/            # Tool schemas for LLM
    └── tools.json      # JSON schema definitions
```

### Step 3: Implement Clean API

```rust
// lib.rs
pub mod pathlib;
pub mod sandbox;  // Existing
pub mod tools;
pub mod error;
pub mod schemas;

pub use tools::*;  // Export all tool functions
pub use sandbox::{AgentTools, SandboxConfig, FsResult, FsError};

// Unified API combining sandbox + utilities
pub struct AgentToolkit {
    sandbox: AgentTools,
}

impl AgentToolkit {
    pub fn new(config: SandboxConfig) -> Self { ... }
    
    // File operations
    pub fn read_file(&self, path: &str) -> Result<String> { ... }
    pub fn list_dir(&self, path: &str) -> Result<Vec<FileInfo>> { ... }
    pub fn search_files(&self, pattern: &str) -> Result<Vec<String>> { ... }
    pub fn search_content(&self, pattern: &str) -> Result<Vec<Match>> { ... }
    
    // File manipulation
    pub fn copy_file(&self, src: &str, dst: &str) -> Result<()> { ... }
    pub fn move_file(&self, src: &str, dst: &str) -> Result<()> { ... }
    pub fn delete_file(&self, path: &str) -> Result<()> { ... }
    pub fn create_dir(&self, path: &str) -> Result<()> { ... }
    pub fn touch_file(&self, path: &str) -> Result<()> { ... }
    
    // Text processing
    pub fn head_file(&self, path: &str, n: usize) -> Result<String> { ... }
    pub fn tail_file(&self, path: &str, n: usize) -> Result<String> { ... }
    pub fn count_lines(&self, path: &str) -> Result<CountResult> { ... }
    
    // System utilities
    pub fn which_command(&self, name: &str) -> Result<PathBuf> { ... }
    pub fn dir_tree(&self, path: &str, depth: usize) -> Result<TreeNode> { ... }
}
```

## Build Optimization

### Current Issues

- 95+ member crates = slow build times
- Heavy dependencies (uucore, git2, criterion)
- Unnecessary features enabled

### Optimizations

1. **Single Crate**: Merge all utilities into `mistralrs-agent-tools`
1. **Minimal Dependencies**: Only essential crates
1. **Feature Flags**: Optional advanced features
1. **LTO**: Link-time optimization enabled
1. **Strip Symbols**: Reduce binary size

### Cargo.toml Updates

```toml
[package]
name = "mistralrs-agent-tools"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
thiserror = "1.0"
camino = "1.1"  # UTF-8 paths
walkdir = "2.4"
regex = "1.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"

[features]
default = ["search", "textproc"]
search = []          # find, grep functionality
textproc = []        # head, tail, wc, sort
advanced = []        # tree, which, sysinfo

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
strip = true
panic = "abort"
```

## Summary

### Current State

- 95+ utilities in winutils workspace
- Complex build system with heavy dependencies
- CLI-focused tools (not library-friendly)
- Windows-specific optimizations (some unnecessary)

### Target State

- 15-20 essential utilities in mistralrs-agent-tools
- Simplified dependencies (core functionality only)
- Library API (programmatic access)
- Sandbox-integrated (security built-in)
- Fast builds (single crate, minimal deps)
- LLM-friendly (JSON schemas, structured output)

### Next Steps

1. ✅ Update .gitignore
1. ⏳ Extract essential winpath functionality
1. ⏳ Implement core file operations (read, list, find)
1. ⏳ Add JSON schemas for LLM integration
1. ⏳ Test integration with agent_mode.rs
1. ⏳ Document API for developers
