# Contributing to WinUtils

Thank you for your interest in contributing to WinUtils! This guide provides everything you need to know about contributing to the project.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
1. [Getting Started](#getting-started)
1. [Development Setup](#development-setup)
1. [Build System Requirements](#build-system-requirements)
1. [Coding Standards](#coding-standards)
1. [Testing Requirements](#testing-requirements)
1. [Submission Process](#submission-process)
1. [Adding New Utilities](#adding-new-utilities)
1. [Performance Guidelines](#performance-guidelines)
1. [Documentation Standards](#documentation-standards)

## Code of Conduct

### Our Standards

- **Be respectful** and considerate in all interactions
- **Be constructive** in feedback and criticism
- **Be inclusive** and welcoming to all contributors
- **Focus on** technical merit and project goals
- **Maintain** professional communication

### Unacceptable Behavior

- Harassment, discrimination, or personal attacks
- Trolling or inflammatory comments
- Publishing private information without consent
- Any conduct that could be considered inappropriate

## Getting Started

### Prerequisites

1. **Rust Toolchain** (1.75+ required)

   ```bash
   rustup update stable
   rustup target add x86_64-pc-windows-msvc
   ```

1. **GNU Make** (MANDATORY for build system)

   ```bash
   # Install via Git Bash or MSYS2
   pacman -S make
   ```

1. **Windows Build Tools**

   - Visual Studio 2019+ or Build Tools
   - Windows SDK 10.0.19041+

1. **Git** with Git Bash

   ```bash
   git config --global core.autocrlf false
   ```

### Fork and Clone

```bash
# Fork the repository on GitHub first
git clone https://github.com/YOUR_USERNAME/winutils.git
cd winutils

# Add upstream remote
git remote add upstream https://github.com/david-t-martel/uutils-windows.git

# Keep your fork updated
git fetch upstream
git checkout main
git merge upstream/main
```

## Development Setup

### 🚨 CRITICAL: Build System Requirements 🚨

**THE MAKEFILE IS MANDATORY. NEVER USE CARGO DIRECTLY.**

```bash
# ✅ CORRECT - Always use Make
make clean
make release
make test

# ❌ WRONG - Never use these
cargo build    # FORBIDDEN
cargo test     # FORBIDDEN
cargo install  # FORBIDDEN
```

### Why Make is Mandatory

1. **winpath must be built first** - It's a critical dependency
1. **Build order is enforced** - 89 crates in specific sequence
1. **Optimizations are applied** - Platform-specific flags
1. **Integration is validated** - Post-build verification

### Development Workflow

```bash
# 1. Start with a clean build
make clean

# 2. Build the project
make release

# 3. Run tests
make test

# 4. Validate all utilities
make validate-all-77

# 5. Install locally for testing
make install
```

## Coding Standards

### Rust Style Guide

1. **Format all code** with rustfmt

   ```bash
   cargo fmt --all
   ```

1. **Pass clippy lints**

   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```

1. **Use meaningful names**

   ```rust
   // Good
   let file_path = normalize_path(input)?;

   // Bad
   let fp = np(i)?;
   ```

1. **Document public APIs**

   ````rust
   /// Normalizes a path to Windows format.
   ///
   /// # Arguments
   /// * `path` - The input path to normalize
   ///
   /// # Returns
   /// A normalized `PathBuf` or an error
   ///
   /// # Examples
   /// ```
   /// let normalized = normalize_path("/mnt/c/Windows")?;
   /// assert_eq!(normalized, PathBuf::from("C:\\Windows"));
   /// ```
   pub fn normalize_path(path: &str) -> Result<PathBuf> {
       // Implementation
   }
   ````

### Error Handling

1. **Use Result types** for fallible operations

   ```rust
   pub fn read_file(path: &Path) -> Result<String> {
       fs::read_to_string(path)
           .context("Failed to read file")
   }
   ```

1. **Provide context** for errors

   ```rust
   file.read_to_string(&mut contents)
       .with_context(|| format!("Failed to read {}", path.display()))?;
   ```

1. **Never panic** in library code

   ```rust
   // Bad
   let value = map.get(&key).unwrap();

   // Good
   let value = map.get(&key)
       .ok_or_else(|| Error::KeyNotFound(key))?;
   ```

### Performance Requirements

1. **Benchmark changes** that claim performance improvements

   ```rust
   #[bench]
   fn bench_normalize_path(b: &mut Bencher) {
       b.iter(|| {
           normalize_path("/mnt/c/Windows/System32")
       });
   }
   ```

1. **Use zero-copy** operations where possible

   ```rust
   // Use &str instead of String when possible
   pub fn process(input: &str) -> &str {
       // Process without allocation
   }
   ```

1. **Minimize allocations** in hot paths

   ```rust
   // Reuse buffers
   let mut buffer = Vec::with_capacity(8192);
   for item in items {
       buffer.clear();
       process_into(&item, &mut buffer);
   }
   ```

## Testing Requirements

### Test Coverage

- **Minimum 85% code coverage** required
- All public APIs must have tests
- Platform-specific code needs Windows tests

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test implementation
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_specific() {
        // Windows-only test
    }

    #[test]
    fn test_error_cases() {
        // Test error handling
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use winutils::*;

#[test]
fn test_end_to_end() {
    // Full workflow test
}
```

### Git Bash Path Tests

```rust
#[test]
fn test_git_bash_paths() {
    let paths = vec![
        ("/c/Windows", "C:\\Windows"),
        ("/d/Projects", "D:\\Projects"),
        ("/c/Program Files", "C:\\Program Files"),
    ];

    for (input, expected) in paths {
        let result = normalize_path(input).unwrap();
        assert_eq!(result, PathBuf::from(expected));
    }
}
```

## Submission Process

### 1. Branch Naming

```bash
# Feature branches
git checkout -b feature/utility-name

# Bug fixes
git checkout -b fix/issue-description

# Performance improvements
git checkout -b perf/optimization-description

# Documentation
git checkout -b docs/topic
```

### 2. Commit Messages

Follow conventional commit format:

```
type(scope): brief description

Detailed explanation of the change, why it was needed,
and any important implementation details.

Fixes #123
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `docs`: Documentation
- `test`: Testing
- `refactor`: Code refactoring
- `style`: Formatting changes

### 3. Pull Request Process

1. **Update your branch**

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

1. **Run all checks**

   ```bash
   make clean
   make release
   make test
   make validate-all-77
   ```

1. **Create pull request**

   - Clear title describing the change
   - Reference any related issues
   - Include benchmark results for performance changes
   - Add tests for new functionality

1. **PR Checklist**

   - [ ] Code follows style guidelines
   - [ ] Tests pass locally
   - [ ] Documentation updated
   - [ ] Benchmarks included (if performance-related)
   - [ ] No direct cargo commands used
   - [ ] Changes validated with `make validate-all-77`

## Adding New Utilities

### 1. Create Utility Structure

```bash
# Create new utility directory
mkdir -p coreutils/src/newutil
cd coreutils/src/newutil
```

### 2. Implement Cargo.toml

```toml
[package]
name = "wu-newutil"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
winpath.workspace = true
winutils-core.workspace = true
anyhow.workspace = true
clap.workspace = true

[dev-dependencies]
tempfile = "3.15"
assert_cmd = "2.0"
```

### 3. Implement main.rs

```rust
use anyhow::Result;
use clap::Parser;
use winpath::PathNormalizer;
use winutils_core::{Config, setup_utility};

#[derive(Parser)]
#[command(name = "newutil")]
#[command(about = "Brief description")]
struct Args {
    /// Input files
    files: Vec<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    setup_utility("newutil")?;

    let args = Args::parse();
    let normalizer = PathNormalizer::new();

    for file in &args.files {
        let path = normalizer.normalize(file)?;
        process_file(&path, &args)?;
    }

    Ok(())
}

fn process_file(path: &Path, args: &Args) -> Result<()> {
    // Implementation
    Ok(())
}
```

### 4. Add to Workspace

```toml
# winutils/Cargo.toml
[workspace]
members = [
    # ... existing members ...
    "coreutils/src/newutil",  # Add your utility
]
```

### 5. Update Makefile

```makefile
# Add to utility list
UTILITIES += newutil

# Add to build targets
build-newutil:
    cargo build --release -p wu-newutil
```

### 6. Add Tests

```rust
// coreutils/src/newutil/tests/test_newutil.rs
#[test]
fn test_basic_functionality() {
    // Test implementation
}

#[test]
fn test_path_normalization() {
    // Test Git Bash path handling
}
```

## Performance Guidelines

### Benchmarking Requirements

1. **Baseline measurements** before optimization
1. **Comparative benchmarks** against native tools
1. **Memory profiling** for large operations
1. **CPU profiling** for hot paths

### Optimization Checklist

- [ ] Profile first, optimize second
- [ ] Use release mode for benchmarks
- [ ] Test with realistic data sizes
- [ ] Consider memory vs speed tradeoffs
- [ ] Document optimization rationale
- [ ] Maintain readability

### Performance PR Requirements

```markdown
## Performance Improvement

### Benchmark Results
Before: 1.234s (100MB file)
After: 0.456s (100MB file)
Improvement: 63% faster

### Methodology
- Test data: 100MB text file with 1M lines
- Hardware: Intel i7-12700K, 32GB RAM
- OS: Windows 11 23H2
- Measurement: Average of 10 runs

### Code Changes
- Implemented SIMD for byte counting
- Added memory-mapped I/O for large files
- Optimized path caching
```

## Documentation Standards

### Code Documentation

1. **Module documentation**

   ```rust
   //! This module provides path normalization functionality
   //! for cross-platform compatibility.
   ```

1. **Function documentation**

   ```rust
   /// Normalizes a path according to the current platform.
   ///
   /// This function handles DOS, WSL, Cygwin, and Git Bash paths.
   pub fn normalize(path: &str) -> Result<PathBuf>
   ```

1. **Safety documentation**

   ```rust
   // SAFETY: The pointer is valid for the lifetime of the slice
   // and properly aligned for u32 access.
   unsafe {
       // Implementation
   }
   ```

### User Documentation

1. **README.md** for each utility
1. **Examples** in documentation
1. **Error messages** that guide users
1. **Help text** that's comprehensive

### API Documentation

```bash
# Generate and review documentation
cargo doc --no-deps --open
```

## Review Process

### Code Review Criteria

1. **Functionality**: Does it work correctly?
1. **Performance**: Does it meet performance targets?
1. **Testing**: Is it adequately tested?
1. **Documentation**: Is it well documented?
1. **Style**: Does it follow conventions?
1. **Security**: Are there any security concerns?

### Review Response Time

- Initial review: Within 48 hours
- Follow-up reviews: Within 24 hours
- Minor fixes: Same day if possible

## Getting Help

### Resources

- **Documentation**: See `/docs` directory
- **Issues**: GitHub issue tracker
- **Discussions**: GitHub Discussions
- **Email**: david.martel@auricleinc.com

### Common Issues

1. **Build failures**: Always use `make clean` first
1. **Path issues**: Ensure winpath is built
1. **Test failures**: Check Windows version compatibility
1. **Performance**: Profile before optimizing

## Recognition

Contributors are recognized in:

- AUTHORS.md file
- Release notes
- Documentation credits

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).

______________________________________________________________________

*Contributing Guide Version: 1.0.0*
*Last Updated: January 2025*
*Maintained by: David Martel*
