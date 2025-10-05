# WinUtils Integration Plan for mistralrs-agent-tools

## Executive Summary

This document outlines the plan to incorporate the winutils framework (95+ coreutils) into the mistralrs-agent-tools crate, optimized for LLM agent use. The goal is to provide a clean, sandboxed, library-based API rather than CLI-based utilities.

## Current State Analysis

### mistralrs-agent-tools (Current)

- ✅ Pure Rust implementation
- ✅ Sandbox enforcement with configurable root
- ✅ Basic file operations: read, write, append, delete, exists, find, tree
- ✅ Path validation and traversal prevention
- ✅ Read-only path protection
- ✅ Size and result limits
- ❌ Limited utility set (7 operations)
- ❌ No text processing capabilities
- ❌ No advanced search features
- ❌ No file metadata operations

### winutils Framework

- ✅ 95+ GNU coreutils implementations
- ✅ Windows-optimized with path normalization
- ✅ Comprehensive test coverage
- ✅ Performance benchmarks
- ❌ CLI-focused (not library-friendly)
- ❌ Complex build system (95+ member crates)
- ❌ Heavy dependencies (uucore, git2, criterion)
- ❌ Slow build times
- ❌ No sandbox integration

## Integration Strategy

### Approach: Selective Extraction with Refactoring

**NOT**: Link entire winutils workspace
**NOT**: Subprocess to winutils binaries\
**YES**: Extract and refactor essential utilities
**YES**: Integrate into existing sandbox framework
**YES**: Provide unified library API

### Rationale

1. **Performance**: Direct Rust API faster than subprocess
1. **Security**: Sandbox validation at API level
1. **Maintainability**: Single codebase, simpler dependencies
1. **Build Time**: Avoid 95+ crate builds
1. **Type Safety**: Rust types instead of CLI string parsing

## Implementation Plan

### Phase 1: Foundation (Priority: Critical)

#### 1.1 Extract Path Library

**File**: `mistralrs-agent-tools/src/pathlib.rs`

**Source**: `winutils/shared/winpath/src/`

**Extract**:

- Path normalization (backslash conversion)
- Path validation (absolute/relative detection)
- Path joining and parent operations
- Windows path detection

**Exclude**:

- Git Bash path conversion (complex, rarely needed)
- WSL path conversion (can add later)
- Path caching (add if performance needed)

**API**:

```rust
pub fn normalize_path(path: &str) -> Result<String>
pub fn is_absolute(path: &str) -> bool
pub fn join_paths(base: &str, rel: &str) -> Result<String>
pub fn parent_path(path: &str) -> Option<String>
```

#### 1.2 Enhance Sandbox Integration

**File**: `mistralrs-agent-tools/src/lib.rs` (update existing)

**Changes**:

- Integrate pathlib for path operations
- Add utility methods to AgentTools
- Maintain existing sandbox enforcement

### Phase 2: Core File Operations (Priority: High)

#### 2.1 Enhanced File Reading

**Tool**: `list_contents` (from cat)

**Source**: `winutils/coreutils/src/cat/src/`

**Extract**:

- Buffered file reading
- Error handling for permissions
- Line-by-line iteration

**Exclude**:

- BOM handling (rarely needed)
- Line ending conversion (add if requested)
- Color output (not relevant for API)
- Number lines (can add as option)

**API**:

```rust
pub fn read_lines(&self, path: &str) -> Result<Vec<String>>
pub fn read_bytes(&self, path: &str, max_size: Option<usize>) -> Result<Vec<u8>>
```

#### 2.2 Directory Listing

**Tool**: `list_directory` (from ls)

**Source**: `winutils/coreutils/src/ls/src/`

**Extract**:

- Directory entry reading
- File metadata extraction
- Sorting options
- Pattern filtering

**Exclude**:

- Color output
- Long format display (format in API consumer)
- Windows attributes (can add as metadata)

**API**:

```rust
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub fn list_directory(
    &self,
    path: &str,
    recursive: bool,
    pattern: Option<&str>,
) -> Result<Vec<FileEntry>>
```

#### 2.3 File Search

**Tool**: `search_files` (from find-wrapper)

**Source**: `winutils/derive-utils/find-wrapper/src/`

**Extract**:

- Recursive directory walking
- Pattern matching (glob/regex)
- Type filtering (file/directory)
- Depth limits

**Exclude**:

- fd integration (use walkdir directly)
- Gitignore support (can add later)
- Parallel search (add if needed)

**API**:

```rust
pub struct SearchOptions {
    pub pattern: Option<String>,
    pub regex: bool,
    pub file_type: Option<FileType>,  // File, Dir, Any
    pub max_depth: Option<usize>,
    pub case_sensitive: bool,
}

pub fn search_files(
    &self,
    root: &str,
    options: SearchOptions,
) -> Result<Vec<String>>
```

#### 2.4 Content Search

**Tool**: `search_content` (from grep-wrapper)

**Source**: `winutils/derive-utils/grep-wrapper/src/`

**Extract**:

- File content searching
- Regex pattern matching
- Context lines (before/after)
- Line number tracking

**Exclude**:

- ripgrep integration (use regex directly)
- Color output
- Binary file detection (can add)

**API**:

```rust
pub struct Match {
    pub file: String,
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub fn search_content(
    &self,
    pattern: &str,
    paths: &[String],
    context_lines: usize,
) -> Result<Vec<Match>>
```

### Phase 3: File Manipulation (Priority: Medium)

#### 3.1 Copy Files

**Tool**: `copy_file` (from cp)

**Source**: `winutils/coreutils/src/cp/src/`

**Extract**:

- File copying with metadata
- Directory recursion
- Overwrite protection
- Progress tracking (optional)

**API**:

```rust
pub struct CopyOptions {
    pub recursive: bool,
    pub overwrite: bool,
    pub preserve_attrs: bool,
}

pub fn copy_file(
    &self,
    src: &str,
    dst: &str,
    options: CopyOptions,
) -> Result<()>
```

#### 3.2 Move/Rename Files

**Tool**: `move_file` (from mv)

**API**:

```rust
pub fn move_file(
    &self,
    src: &str,
    dst: &str,
    overwrite: bool,
) -> Result<()>
```

#### 3.3 Create Directories

**Tool**: `create_directory` (from mkdir)

**API**:

```rust
pub fn create_directory(
    &self,
    path: &str,
    recursive: bool,  // -p flag
) -> Result<()>
```

#### 3.4 Touch Files

**Tool**: `touch_file` (from touch)

**API**:

```rust
pub fn touch_file(&self, path: &str) -> Result<()>
```

### Phase 4: Text Processing (Priority: Medium)

#### 4.1 Head/Tail

**Tools**: `head_file`, `tail_file`

**Source**: `winutils/coreutils/src/{head,tail}/src/`

**API**:

```rust
pub fn head_file(&self, path: &str, n: usize) -> Result<Vec<String>>
pub fn tail_file(&self, path: &str, n: usize) -> Result<Vec<String>>
```

#### 4.2 Word Count

**Tool**: `count_lines` (from wc)

**Source**: `winutils/coreutils/src/wc/src/`

**API**:

```rust
pub struct CountResult {
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
    pub bytes: usize,
}

pub fn count_file(&self, path: &str) -> Result<CountResult>
```

#### 4.3 Sort

**Tool**: `sort_lines` (from sort)

**API**:

```rust
pub fn sort_lines(
    &self,
    lines: Vec<String>,
    numeric: bool,
    reverse: bool,
) -> Vec<String>
```

### Phase 5: System Utilities (Priority: Low)

#### 5.1 Which/Where

**Tool**: `which_command`

**Source**: `winutils/derive-utils/{which,where}/src/`

**API**:

```rust
pub fn which_command(&self, name: &str) -> Result<PathBuf>
pub fn find_in_path(&self, name: &str) -> Result<Vec<PathBuf>>  // All matches
```

#### 5.2 Directory Tree

**Tool**: `directory_tree`

**Source**: `winutils/derive-utils/tree/src/`

**API**:

```rust
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

pub fn directory_tree(
    &self,
    root: &str,
    max_depth: Option<usize>,
) -> Result<TreeNode>
```

## Module Structure

```
mistralrs-agent-tools/src/
├── lib.rs              # Main API, AgentToolkit
├── error.rs            # Unified error types (existing)
├── sandbox.rs          # AgentTools with sandbox (existing)
├── pathlib.rs          # Path utilities (new - extracted from winpath)
├── tools/              # Tool implementations (new)
│   ├── mod.rs          # Tool registry and common types
│   ├── read.rs         # Enhanced file reading
│   ├── list.rs         # Directory listing with metadata
│   ├── search.rs       # File and content search
│   ├── copy.rs         # File copying
│   ├── move.rs         # File moving/renaming
│   ├── mkdir.rs        # Directory creation
│   ├── touch.rs        # Timestamp updates
│   ├── textproc.rs     # head, tail, wc, sort
│   ├── tree.rs         # Directory tree
│   └── which.rs        # Command lookup
└── schemas.rs          # Tool schemas for LLM (new)
```

## Unified API Design

### Current AgentTools API

```rust
pub struct AgentTools {
    config: SandboxConfig,
}

impl AgentTools {
    pub fn read(&self, path: &str) -> Result<FsResult>
    pub fn write(&self, path: &str, content: &str, create: bool, overwrite: bool) -> Result<FsResult>
    pub fn append(&self, path: &str, content: &str) -> Result<FsResult>
    pub fn delete(&self, path: &str) -> Result<FsResult>
    pub fn exists(&self, path: &str) -> Result<bool>
    pub fn find(&self, pattern: &str, max_depth: Option<usize>) -> Result<Vec<String>>
    pub fn tree(&self, root: Option<String>, max_depth: Option<usize>) -> Result<Vec<String>>
}
```

### Enhanced AgentToolkit API (Proposed)

```rust
pub struct AgentToolkit {
    sandbox: AgentTools,
}

impl AgentToolkit {
    pub fn new(config: SandboxConfig) -> Self
    
    // Core operations (keep existing, enhance)
    pub fn read_file(&self, path: &str) -> Result<String>
    pub fn read_lines(&self, path: &str) -> Result<Vec<String>>
    pub fn read_bytes(&self, path: &str, max_size: Option<usize>) -> Result<Vec<u8>>
    pub fn write_file(&self, path: &str, content: &str) -> Result<()>
    pub fn append_file(&self, path: &str, content: &str) -> Result<()>
    pub fn delete_file(&self, path: &str) -> Result<()>
    pub fn file_exists(&self, path: &str) -> Result<bool>
    
    // Enhanced operations (new)
    pub fn list_directory(&self, path: &str, recursive: bool) -> Result<Vec<FileEntry>>
    pub fn search_files(&self, root: &str, options: SearchOptions) -> Result<Vec<String>>
    pub fn search_content(&self, pattern: &str, paths: &[String]) -> Result<Vec<Match>>
    pub fn copy_file(&self, src: &str, dst: &str, options: CopyOptions) -> Result<()>
    pub fn move_file(&self, src: &str, dst: &str) -> Result<()>
    pub fn create_directory(&self, path: &str, recursive: bool) -> Result<()>
    pub fn touch_file(&self, path: &str) -> Result<()>
    
    // Text processing (new)
    pub fn head_file(&self, path: &str, n: usize) -> Result<Vec<String>>
    pub fn tail_file(&self, path: &str, n: usize) -> Result<Vec<String>>
    pub fn count_file(&self, path: &str) -> Result<CountResult>
    pub fn sort_lines(&self, lines: Vec<String>, options: SortOptions) -> Vec<String>
    
    // System utilities (new)
    pub fn which_command(&self, name: &str) -> Result<PathBuf>
    pub fn directory_tree(&self, root: &str, max_depth: Option<usize>) -> Result<TreeNode>
}
```

## Tool Schemas for LLM Integration

### Schema Format

```json
{
  "tools": [
    {
      "name": "read_file",
      "description": "Read the contents of a file",
      "parameters": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Path to the file to read"
          },
          "max_size": {
            "type": "integer",
            "description": "Maximum bytes to read (optional)",
            "default": null
          }
        },
        "required": ["path"]
      },
      "returns": {
        "type": "string",
        "description": "File contents as a string"
      },
      "errors": [
        "OutsideSandbox",
        "ReadOnly",
        "FileTooLarge",
        "NotFound",
        "PermissionDenied"
      ]
    },
    {
      "name": "list_directory",
      "description": "List files and directories",
      "parameters": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Directory path to list"
          },
          "recursive": {
            "type": "boolean",
            "description": "Include subdirectories",
            "default": false
          },
          "pattern": {
            "type": "string",
            "description": "Glob pattern to filter results (optional)",
            "default": null
          }
        },
        "required": ["path"]
      },
      "returns": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "path": {"type": "string"},
            "is_dir": {"type": "boolean"},
            "size": {"type": "integer"},
            "modified": {"type": "string", "format": "date-time"}
          }
        }
      }
    }
  ]
}
```

## Build System Optimization

### Current Cargo.toml

```toml
[package]
name = "mistralrs-agent-tools"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
camino = "1.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
walkdir = "2.4"

[dev-dependencies]
tempfile = "3.8"
```

### Enhanced Cargo.toml

```toml
[package]
name = "mistralrs-agent-tools"
version = "0.2.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
camino = "1.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
walkdir = "2.4"
regex = "1.10"        # For pattern matching
glob = "0.3"          # For glob patterns
filetime = "0.2"      # For touch operations

# Optional dependencies
memmap2 = { version = "0.9", optional = true }  # For large file reading

[dev-dependencies]
tempfile = "3.8"
proptest = "1.4"      # Property-based testing

[features]
default = ["search", "textproc"]
search = ["regex", "glob"]    # File/content search
textproc = []                  # Text processing tools
mmap = ["memmap2"]             # Memory-mapped file I/O
advanced = ["search", "textproc", "mmap"]

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
strip = true
panic = "abort"
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_file() {
        let dir = TempDir::new().unwrap();
        let config = SandboxConfig {
            root: dir.path().to_path_buf(),
            readonly_paths: vec![],
            enforce: true,
        };
        let toolkit = AgentToolkit::new(config);
        
        // Write test file
        let content = "Hello, World!";
        toolkit.write_file("test.txt", content).unwrap();
        
        // Read and verify
        let read_content = toolkit.read_file("test.txt").unwrap();
        assert_eq!(read_content, content);
    }
    
    #[test]
    fn test_list_directory() {
        // Test directory listing with various options
    }
    
    #[test]
    fn test_search_files() {
        // Test file search with patterns
    }
    
    // ... more tests
}
```

### Integration Tests

```rust
#[test]
fn test_agent_workflow() {
    // Simulate agent workflow:
    // 1. Search for config files
    // 2. Read and parse them
    // 3. Create backup directory
    // 4. Copy modified files
    // 5. Verify results
}
```

## Migration Path

### Step 1: Setup (Week 1)

1. ✅ Update .gitignore
1. ✅ Create architecture documentation
1. ⏳ Create module structure
1. ⏳ Extract pathlib from winpath

### Step 2: Core Operations (Week 2)

5. ⏳ Implement enhanced read operations
1. ⏳ Implement directory listing
1. ⏳ Add search capabilities
1. ⏳ Create unit tests

### Step 3: File Manipulation (Week 3)

9. ⏳ Implement copy/move operations
1. ⏳ Add directory creation
1. ⏳ Add touch functionality
1. ⏳ Integration tests

### Step 4: Text Processing (Week 4)

13. ⏳ Implement head/tail
01. ⏳ Add word count
01. ⏳ Add sort functionality
01. ⏳ Performance testing

### Step 5: Integration & Polish (Week 5)

17. ⏳ Create tool schemas
01. ⏳ Update agent_mode.rs integration
01. ⏳ Write API documentation
01. ⏳ Final testing and optimization

## Success Criteria

### Functionality

- ✅ All essential file operations working
- ✅ Search capabilities (files and content)
- ✅ Text processing tools
- ✅ Sandbox enforcement maintained
- ✅ All tests passing

### Performance

- ⏳ Build time < 30 seconds (vs 5+ minutes for full winutils)
- ⏳ File operations < 10ms for small files
- ⏳ Directory listing < 100ms for 1000 files
- ⏳ Search < 1 second for 10,000 files

### Code Quality

- ⏳ Test coverage > 80%
- ⏳ No unsafe code (except where necessary)
- ⏳ Comprehensive error handling
- ⏳ Clear documentation
- ⏳ Clippy warnings = 0

## Summary

This integration plan transforms the CLI-focused winutils framework into a library-based agent toolkit. By selectively extracting essential utilities and refactoring them into a unified API, we achieve:

1. **Better Performance**: Direct Rust API instead of subprocess overhead
1. **Enhanced Security**: Sandbox integrated at API level
1. **Simpler Builds**: Single crate vs 95+ crates
1. **Type Safety**: Structured data vs string parsing
1. **LLM-Friendly**: JSON schemas and structured responses

The result is a comprehensive toolkit that empowers LLM agents with safe, efficient filesystem operations while maintaining the security guarantees of the existing sandbox framework.
