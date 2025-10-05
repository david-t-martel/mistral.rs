# WinPath Test Suite

This directory contains comprehensive tests for the WinPath library, focusing on the Git Bash path mangling issue and all supported path formats.

## Test Files

### Git Bash Path Mangling Tests

**File:** `git_bash_tests.rs`

Tests specifically designed to validate the Git Bash path mangling issue where paths like `/mnt/c/users/david/.local/bin/ls.exe` get mangled into `C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe`.

**Key Test Cases:**

- Standard Git Bash mangled paths from common Git installation locations
- Complex nested paths with spaces and special characters
- Edge cases and error conditions
- Performance validation
- Differentiation between WSL and Git Bash mangled paths

### Comprehensive Integration Tests

**File:** `integration_tests.rs`

Extensive test suite covering all path formats supported by the library:

**Path Formats Tested:**

- DOS paths: `C:\Users\David`
- DOS forward slash: `C:/Users/David`
- WSL mount points: `/mnt/c/users/david`
- Cygwin paths: `/cygdrive/c/users/david`
- UNC long paths: `\\?\C:\Users\David`
- Unix-like: `//c/users/david`
- Mixed separators: `C:\Users/David\Documents`

**Special Features Tested:**

- Unicode and special character handling
- Long path handling and UNC prefix insertion
- Zero-copy optimization
- Error conditions and edge cases
- Performance and thread safety
- Path normalization idempotency

### Basic Functionality Tests

**File:** `basic_tests.rs`

Core functionality tests covering:

- Basic path normalization for all formats
- Path format detection
- Zero-copy optimization
- Error handling
- Mixed separator handling
- Long path support

### WSL-Specific Tests

**File:** `wsl_path_tests.rs`

Focused tests for WSL path scenarios:

- WSL path normalization accuracy
- Various drive letter support
- Complex WSL path scenarios
- Edge cases and error conditions
- Performance validation
- Protection against Git Bash pollution

## Executable Path Tests

**File:** `../../tests/executable_path_tests.rs`

Tests that validate executable self-path reporting in different environments:

- Path reporting in CMD, PowerShell, Git Bash, and WSL
- Runtime path normalization validation
- Environment variable path resolution
- File operation path contexts
- Performance in executable context

## Running the Tests

### Individual Test Files

```bash
# Run specific test file
cargo test --test git_bash_tests
cargo test --test integration_tests
cargo test --test basic_tests
cargo test --test wsl_path_tests

# Run with verbose output
cargo test --test git_bash_tests -- --nocapture

# Run specific test function
cargo test test_git_bash_mangled_paths
```

### All Tests

```bash
# Run all library tests
cargo test

# Run all tests including integration tests
cargo test --tests

# Run with release optimizations
cargo test --release
```

### Validation Scripts

Multiple validation scripts are provided for different environments:

#### Python Script (Cross-platform)

```bash
python scripts/validate_paths.py --full --verbose
```

#### Bash Script (Git Bash, WSL, Linux)

```bash
bash scripts/validate.sh
```

#### Batch Script (Windows CMD)

```cmd
scripts\validate.bat
```

## Performance Testing

### Benchmark Suite

```bash
# Run performance benchmarks
cargo bench

# Quick performance validation
cargo test --release test_.*performance
```

### Performance Targets

The test suite validates these performance targets:

- **WSL path normalization**: 1000 normalizations < 10ms
- **Git Bash path normalization**: 1000 normalizations < 15ms
- **Zero-copy optimization**: Already normalized paths should not be modified
- **Thread safety**: Concurrent access should work correctly

## Test Data and Scenarios

### Git Bash Mangling Patterns

The tests cover these Git Bash installation patterns:

```
C:\Program Files\Git\mnt\c\...
C:\Program Files (x86)\Git\mnt\c\...
C:\Git\mnt\c\...
D:\Git\mnt\c\...
C:\Tools\Git\mnt\c\...
C:\Users\<user>\scoop\apps\git\current\mnt\c\...
C:\PortableApps\GitPortable\App\Git\mnt\c\...
```

### Path Format Examples

```rust
// DOS paths
"C:\\Users\\David"
"D:\\Program Files\\App\\tool.exe"

// WSL paths
"/mnt/c/users/david/.local/bin/ls.exe"
"/mnt/d/temp/file.txt"

// Cygwin paths
"/cygdrive/c/users/david"
"/cygdrive/e/backup/data.zip"

// Git Bash mangled paths
"C:\\Program Files\\Git\\mnt\\c\\users\\david\\.local\\bin\\ls.exe"
```

### Unicode and Special Cases

- Unicode characters: `josé`, `文档`, `пользователи`
- Special characters: `@`, `-`, `_`, `.`, spaces
- Hidden files: `.config`, `.local`
- Long paths: >260 characters
- Edge cases: empty paths, single characters, malformed paths

## Error Conditions

The test suite validates proper error handling for:

- Empty or whitespace-only paths
- Invalid drive letters (numeric, special characters)
- Malformed WSL paths (`/mnt/`, `/mnt/z/test`)
- Malformed Cygwin paths (`/cygdrive/`, `/cygdrive/z/test`)
- Paths with null bytes (theoretical)
- Very long paths that exceed system limits

## Validation Criteria

### Correctness

- All path formats normalize to correct Windows paths
- No Git Bash pollution in normalized results
- Drive letters are properly extracted and uppercased
- Path separators are correctly normalized to backslashes

### Performance

- Normalization completes within acceptable time limits
- Zero-copy optimization works for already-normalized paths
- Memory usage remains reasonable for large path sets

### Robustness

- Error conditions are handled gracefully
- Thread safety is maintained under concurrent access
- Edge cases don't cause panics or undefined behavior

## Continuous Integration

The test suite is designed to run in CI/CD environments:

```yaml
# Example GitHub Actions step
- name: Run WinPath Tests
  run: |
    cd winutils/shared/winpath
    cargo test --all-targets --all-features
    cargo bench --no-run
    python ../../scripts/validate_paths.py --full
```

## Troubleshooting

### Common Issues

1. **Tests fail on non-Windows platforms**: Some tests are Windows-specific and will be skipped automatically.

1. **Git Bash tests fail**: Ensure Git for Windows is installed, or run tests that don't require Git Bash.

1. **Performance tests timeout**: Adjust iteration counts or run on faster hardware.

1. **Path access denied**: Ensure tests have appropriate permissions for file system access.

### Debug Mode

Run tests with debug output:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Test-Specific Environment Variables

```bash
# Increase performance test iterations
WINPATH_PERF_ITERATIONS=5000 cargo test test_performance

# Enable verbose test output
RUST_TEST_THREADS=1 cargo test -- --nocapture
```

## Contributing

When adding new tests:

1. Follow the existing test structure and naming conventions
1. Include both positive and negative test cases
1. Add performance validation for new functionality
1. Update this README with new test descriptions
1. Ensure tests pass on both debug and release builds

## Test Coverage

The test suite aims for comprehensive coverage:

- **Path formats**: All supported formats with multiple examples
- **Edge cases**: Boundary conditions and error scenarios
- **Performance**: Speed and memory usage validation
- **Integration**: Real-world usage patterns
- **Regression**: Previous bug scenarios to prevent recurrence

Current test coverage focuses heavily on the Git Bash path mangling issue while maintaining comprehensive validation of all library functionality.
