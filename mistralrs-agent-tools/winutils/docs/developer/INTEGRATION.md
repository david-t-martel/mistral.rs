# WinUtils Integration Guide

This guide covers how to add new utilities to the WinUtils project and integrate with existing systems.

## Table of Contents

1. [Adding a New Utility](#adding-a-new-utility)
1. [Project Structure](#project-structure)
1. [Integration with WinPath](#integration-with-winpath)
1. [Using WinUtils-Core](#using-winutils-core)
1. [Build System Integration](#build-system-integration)
1. [Testing Integration](#testing-integration)
1. [Documentation Requirements](#documentation-requirements)
1. [CI/CD Integration](#cicd-integration)

## Adding a New Utility

### Step 1: Planning

Before implementing, consider:

1. **GNU Compatibility**: Does this match GNU coreutils behavior?
1. **Windows Enhancement**: What Windows-specific features to add?
1. **Performance Target**: What's the baseline to beat?
1. **Dependencies**: What shared libraries are needed?

### Step 2: Create Utility Structure

```bash
# Create directory structure
mkdir -p coreutils/src/newutil/{src,tests,benches}
cd coreutils/src/newutil
```

### Step 3: Configure Cargo.toml

```toml
[package]
name = "wu-newutil"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
# CRITICAL: Always include winpath for path normalization
winpath = { workspace = true, features = ["cache", "unicode"] }
winutils-core = { workspace = true, features = ["help", "version", "testing"] }

# Common dependencies
anyhow.workspace = true
clap = { workspace = true, features = ["derive", "env"] }
thiserror.workspace = true

# Windows-specific
windows-sys = { workspace = true, features = ["Win32_Foundation", "Win32_Storage_FileSystem"] }

# Utility-specific dependencies
# Add as needed...

[dev-dependencies]
tempfile.workspace = true
assert_cmd = "2.0"
predicates = "3.0"
```

### Step 4: Implement Main Entry Point

```rust
// src/main.rs
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use winpath::PathNormalizer;
use winutils_core::{Config, setup_utility, HelpBuilder};

/// Brief description of the utility
#[derive(Parser, Debug)]
#[command(name = "newutil")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Brief description", long_about = None)]
struct Args {
    /// Input files to process
    #[arg(required = true)]
    files: Vec<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Recursive operation
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Force operation without prompts
    #[arg(short, long)]
    force: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    format: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Csv,
}

fn main() -> Result<()> {
    // Setup utility environment
    setup_utility("newutil")?;

    // Parse arguments
    let args = Args::parse();

    // Create path normalizer
    let normalizer = PathNormalizer::new();

    // Process each file
    for file in &args.files {
        // CRITICAL: Always normalize paths
        let path = normalizer.normalize(file)
            .with_context(|| format!("Failed to normalize path: {}", file))?;

        process_file(&path, &args)?;
    }

    Ok(())
}

fn process_file(path: &PathBuf, args: &Args) -> Result<()> {
    if args.verbose {
        eprintln!("Processing: {}", path.display());
    }

    // Core utility logic here
    match args.format {
        OutputFormat::Text => process_as_text(path, args),
        OutputFormat::Json => process_as_json(path, args),
        OutputFormat::Csv => process_as_csv(path, args),
    }
}

fn process_as_text(path: &PathBuf, args: &Args) -> Result<()> {
    // Text processing implementation
    Ok(())
}

fn process_as_json(path: &PathBuf, args: &Args) -> Result<()> {
    // JSON output implementation
    Ok(())
}

fn process_as_csv(path: &PathBuf, args: &Args) -> Result<()> {
    // CSV output implementation
    Ok(())
}

// GNU compatibility function
pub fn gnu_compatible_behavior(input: &str) -> String {
    // Ensure GNU-compatible behavior
    input.to_string()
}
```

### Step 5: Add Library Interface

```rust
// src/lib.rs
use anyhow::Result;
use std::path::Path;

pub mod core;
pub mod utils;

/// Main processing function for library users
pub fn process(path: &Path, options: ProcessOptions) -> Result<ProcessResult> {
    let data = read_input(path)?;
    let processed = apply_transformations(data, &options)?;
    Ok(ProcessResult::new(processed))
}

/// Options for processing
#[derive(Debug, Default)]
pub struct ProcessOptions {
    pub verbose: bool,
    pub recursive: bool,
    pub format: Format,
}

/// Processing result
#[derive(Debug)]
pub struct ProcessResult {
    pub data: Vec<u8>,
    pub metadata: Metadata,
}

impl ProcessResult {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            metadata: Metadata::from(&data),
            data,
        }
    }
}

#[derive(Debug)]
pub struct Metadata {
    pub size: usize,
    pub lines: usize,
    pub checksum: u32,
}
```

## Project Structure

### Standard Utility Layout

```
coreutils/src/newutil/
├── Cargo.toml           # Package configuration
├── src/
│   ├── main.rs         # CLI entry point
│   ├── lib.rs          # Library interface
│   ├── core.rs         # Core functionality
│   ├── utils.rs        # Helper functions
│   ├── platform/
│   │   ├── mod.rs      # Platform abstraction
│   │   ├── windows.rs  # Windows-specific code
│   │   └── unix.rs     # Unix compatibility layer
│   └── tests.rs        # Unit tests
├── tests/
│   ├── integration.rs  # Integration tests
│   └── fixtures/       # Test data
├── benches/
│   └── benchmark.rs    # Performance benchmarks
└── README.md           # Utility documentation
```

## Integration with WinPath

### Basic Path Normalization

```rust
use winpath::{PathNormalizer, PathType, NormalizationOptions};

fn normalize_input_paths(paths: &[String]) -> Result<Vec<PathBuf>> {
    // Create normalizer with caching enabled
    let options = NormalizationOptions {
        use_cache: true,
        cache_size: 256,
        canonicalize: true,
        long_path_support: true,
        ..Default::default()
    };

    let normalizer = PathNormalizer::with_options(options);

    paths.iter()
        .map(|p| normalizer.normalize(p))
        .collect::<Result<Vec<_>>>()
}
```

### Advanced Path Handling

```rust
use winpath::{PathType, detect_path_type};

fn process_path_intelligently(path: &str) -> Result<PathBuf> {
    let path_type = detect_path_type(path);

    match path_type {
        PathType::Dos => handle_dos_path(path),
        PathType::Wsl => handle_wsl_path(path),
        PathType::GitBash => handle_git_bash_path(path),
        PathType::Unc => handle_unc_path(path),
        _ => PathBuf::from(path),
    }
}

fn handle_unc_path(path: &str) -> Result<PathBuf> {
    // Special handling for long paths
    if path.len() > 260 {
        // Use \\?\ prefix for long paths
        Ok(PathBuf::from(format!("\\\\?\\{}", path)))
    } else {
        Ok(PathBuf::from(path))
    }
}
```

## Using WinUtils-Core

### Help System Integration

```rust
use winutils_core::help::{HelpBuilder, HelpFormat};

fn create_help() -> String {
    HelpBuilder::new("newutil")
        .version(env!("CARGO_PKG_VERSION"))
        .description("Process files with advanced options")
        .usage("newutil [OPTIONS] <FILES>...")
        .options(vec![
            ("-v, --verbose", "Enable verbose output"),
            ("-r, --recursive", "Process directories recursively"),
            ("-f, --force", "Force operation without prompts"),
            ("--format <FMT>", "Output format: text, json, csv"),
        ])
        .examples(vec![
            ("newutil file.txt", "Process a single file"),
            ("newutil -r dir/", "Process directory recursively"),
            ("newutil --format json *.log", "Output as JSON"),
        ])
        .build(HelpFormat::Terminal)
}
```

### Version Management

```rust
use winutils_core::version::{Version, get_version_info};

fn display_version() {
    let info = get_version_info();
    println!("{} {}", env!("CARGO_PKG_NAME"), info.version);

    if let Some(git) = info.git {
        println!("Git: {} ({})", git.commit, git.branch);
    }

    println!("Built: {}", info.build_date);
    println!("Rust: {}", info.rust_version);
}
```

### Windows Enhancement Features

```rust
use winutils_core::windows::{WindowsAttributes, Registry, ProcessInfo};

fn get_file_attributes(path: &Path) -> Result<WindowsAttributes> {
    WindowsAttributes::from_path(path)
}

fn check_registry_setting() -> Result<String> {
    Registry::read_value(
        r"HKEY_CURRENT_USER\Software\WinUtils",
        "ConfigPath"
    )
}

fn is_running_elevated() -> bool {
    ProcessInfo::current().is_elevated()
}
```

## Build System Integration

### Add to Workspace

```toml
# winutils/Cargo.toml
[workspace]
members = [
    # ... existing members ...
    "coreutils/src/newutil",  # Add your utility here
]
```

### Update Makefile

```makefile
# Add to UTILITIES list
UTILITIES += newutil

# Add build target
.PHONY: build-newutil
build-newutil: build-winpath
	@echo "Building newutil..."
	@cargo build --release -p wu-newutil

# Add to release target dependencies
release: build-winpath build-winutils-core build-newutil

# Add to test target
test-newutil:
	@cargo test -p wu-newutil

# Add to validation
validate: validate-newutil

validate-newutil:
	@echo "Validating newutil..."
	@./target/release/wu-newutil --version
	@./target/release/wu-newutil --help
```

### Build Configuration

```toml
# .cargo/config.toml
[target.'cfg(windows)']
rustflags = [
    "-C", "target-cpu=native",
    "-C", "link-arg=/STACK:8388608",
]

[build]
target-dir = "target"
incremental = true
```

## Testing Integration

### Unit Tests

```rust
// src/tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let input = "test data";
        let result = process_data(input);
        assert_eq!(result, expected_output());
    }

    #[test]
    fn test_path_normalization() {
        let paths = vec![
            ("/c/Windows", "C:\\Windows"),
            ("/mnt/c/Users", "C:\\Users"),
            ("C:/Program Files", "C:\\Program Files"),
        ];

        for (input, expected) in paths {
            let normalized = normalize_path(input).unwrap();
            assert_eq!(normalized, PathBuf::from(expected));
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_specific() {
        // Windows-only tests
    }
}
```

### Integration Tests

```rust
// tests/integration.rs
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_interface() {
    let mut cmd = Command::cargo_bin("wu-newutil").unwrap();

    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("USAGE"));
}

#[test]
fn test_file_processing() {
    let temp = tempdir().unwrap();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "test content").unwrap();

    let mut cmd = Command::cargo_bin("wu-newutil").unwrap();
    cmd.arg(file_path.to_str().unwrap());
    cmd.assert().success();
}

#[test]
fn test_error_handling() {
    let mut cmd = Command::cargo_bin("wu-newutil").unwrap();
    cmd.arg("nonexistent.txt");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
```

### Benchmark Tests

```rust
// benches/benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_process_small_file(c: &mut Criterion) {
    let data = vec![0u8; 1024];

    c.bench_function("process_1kb", |b| {
        b.iter(|| process_data(black_box(&data)))
    });
}

fn bench_process_large_file(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];

    c.bench_function("process_1mb", |b| {
        b.iter(|| process_data(black_box(&data)))
    });
}

criterion_group!(benches, bench_process_small_file, bench_process_large_file);
criterion_main!(benches);
```

## Documentation Requirements

### README.md Template

````markdown
# wu-newutil

Windows-optimized implementation of the `newutil` utility.

## Features

- Full GNU compatibility
- Windows path normalization via winpath
- 70% faster than native implementation
- Unicode support
- Long path support (>260 chars)

## Usage

\```bash
wu-newutil [OPTIONS] <FILES>...
\```

### Options

- `-v, --verbose`: Enable verbose output
- `-r, --recursive`: Process directories recursively
- `-f, --force`: Force operation without prompts
- `--format <FMT>`: Output format (text, json, csv)

### Examples

\```bash
# Process single file
wu-newutil file.txt

# Process directory recursively
wu-newutil -r directory/

# JSON output
wu-newutil --format json *.log
\```

## Performance

| Operation | WinUtils | Native | Improvement |
|-----------|----------|--------|-------------|
| Small files | 12ms | 45ms | 73% faster |
| Large files | 234ms | 890ms | 74% faster |

## Windows-Specific Features

- Handles all path formats (DOS, WSL, Git Bash, UNC)
- Windows file attributes support
- ACL integration
- Registry configuration
````

### API Documentation

````rust
//! # NewUtil Library
//!
//! This library provides the core functionality for the newutil utility.
//!
//! ## Examples
//!
//! ```rust
//! use wu_newutil::{process, ProcessOptions};
//! use std::path::Path;
//!
//! let options = ProcessOptions::default();
//! let result = process(Path::new("file.txt"), options)?;
//! println!("Processed {} bytes", result.data.len());
//! ```

/// Main processing function
///
/// # Arguments
///
/// * `path` - Path to the file to process
/// * `options` - Processing options
///
/// # Returns
///
/// Returns `ProcessResult` on success
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - Processing fails
/// - Invalid options provided
pub fn process(path: &Path, options: ProcessOptions) -> Result<ProcessResult> {
    // Implementation
}
````

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/newutil.yml
name: NewUtil CI

on:
  push:
    paths:
      - 'coreutils/src/newutil/**'
      - 'Cargo.toml'
  pull_request:
    paths:
      - 'coreutils/src/newutil/**'

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: make build-newutil

      - name: Test
        run: cargo test -p wu-newutil

      - name: Benchmark
        run: cargo bench -p wu-newutil

      - name: Integration Test
        run: |
          make build-newutil
          ./scripts/test-newutil.ps1
```

### Validation Script

```powershell
# scripts/test-newutil.ps1
$ErrorActionPreference = "Stop"

# Test basic functionality
& "./target/release/wu-newutil.exe" --version
if ($LASTEXITCODE -ne 0) { throw "Version check failed" }

# Test file processing
$testFile = New-TemporaryFile
"Test content" | Out-File $testFile
& "./target/release/wu-newutil.exe" $testFile
if ($LASTEXITCODE -ne 0) { throw "File processing failed" }

# Test error handling
& "./target/release/wu-newutil.exe" "nonexistent.txt" 2>$null
if ($LASTEXITCODE -eq 0) { throw "Should fail on nonexistent file" }

Write-Host "✓ All tests passed" -ForegroundColor Green
```

## Best Practices

### 1. Always Use WinPath

```rust
// WRONG
let path = PathBuf::from(input);

// RIGHT
let normalizer = PathNormalizer::new();
let path = normalizer.normalize(input)?;
```

### 2. Handle Windows-Specific Cases

```rust
#[cfg(windows)]
fn windows_specific_feature() {
    // Windows-only code
}

#[cfg(not(windows))]
fn windows_specific_feature() {
    // Compatibility stub
}
```

### 3. Performance First

```rust
// Use zero-copy operations
fn process(data: &[u8]) -> &[u8] {
    // Process without allocation
}

// Use SIMD when available
#[cfg(target_feature = "avx2")]
fn simd_process(data: &[u8]) -> Vec<u8> {
    // SIMD implementation
}
```

### 4. GNU Compatibility

```rust
// Match GNU behavior exactly
fn gnu_compatible_sort(data: &mut [String]) {
    data.sort_by(|a, b| {
        // GNU-compatible comparison
    });
}
```

## Checklist for New Utilities

- [ ] Create directory structure
- [ ] Configure Cargo.toml with dependencies
- [ ] Implement main.rs with CLI parsing
- [ ] Integrate winpath for path normalization
- [ ] Add library interface in lib.rs
- [ ] Write comprehensive unit tests
- [ ] Add integration tests
- [ ] Create benchmarks
- [ ] Update workspace Cargo.toml
- [ ] Update Makefile
- [ ] Write README.md
- [ ] Document API with rustdoc
- [ ] Add CI/CD configuration
- [ ] Run validation tests
- [ ] Update main documentation

______________________________________________________________________

*Integration Guide Version: 1.0.0*
*Last Updated: January 2025*
*Follow these guidelines for seamless integration*
