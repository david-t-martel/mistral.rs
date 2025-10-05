# Git Bash Path Normalization Tests for derive-utils

This directory contains comprehensive tests to verify that the derive-utils (`where.exe`, `which.exe`, `tree.exe`) properly handle Git Bash path normalization and return correctly formatted Windows paths.

## Test Coverage

The tests verify handling of various path formats:

- **Git Bash mangled paths**: `/c/Program Files/Git/usr/bin/...`
- **WSL paths**: `/mnt/c/Users/...`
- **Mixed separators**: `C:\Users/David\Documents`
- **Regular Windows paths**: `C:\Windows\System32`
- **Paths with spaces and special characters**
- **Unicode paths**
- **Long paths**

## Test Files

### Unit Tests (Rust)

1. **`where/tests/git_bash_path_tests.rs`**

   - Tests WHERE.EXE with various Git Bash path formats
   - Verifies wildcard pattern matching works correctly
   - Tests performance with path normalization
   - Validates error handling with invalid paths

1. **`which/tests/git_bash_path_tests.rs`**

   - Tests WHICH.EXE with Git Bash PATH entries
   - Verifies PATH environment variable parsing
   - Tests current directory lookup
   - Validates PATHEXT handling with Git Bash paths

1. **`tree/tests/git_bash_path_tests.rs`**

   - Tests TREE.EXE with various path formats
   - Verifies depth limiting works with normalized paths
   - Tests pattern matching and output formats
   - Validates Unicode and special character handling

### Integration Test Scripts

1. **`test_all_git_bash_paths.sh`** (Bash script for Git Bash/WSL)

   - Comprehensive integration testing
   - Performance benchmarking
   - Cross-platform compatibility testing
   - Detailed JSON report generation

1. **`test_all_git_bash_paths_simple.ps1`** (PowerShell for Windows)

   - Quick validation tests
   - Simple pass/fail reporting
   - Easy to run from Windows PowerShell

## Running the Tests

### Prerequisites

1. **Build the derive-utils first**:

   ```bash
   cd T:\projects\coreutils\winutils\derive-utils
   cargo build --release
   ```

1. **Ensure winpath library is properly linked** (required for path normalization)

### Unit Tests

Run individual utility tests:

```bash
# Test where.exe
cargo test --manifest-path T:\projects\coreutils\winutils\derive-utils\where\Cargo.toml git_bash_path

# Test which.exe
cargo test --manifest-path T:\projects\coreutils\winutils\derive-utils\which\Cargo.toml git_bash_path

# Test tree.exe
cargo test --manifest-path T:\projects\coreutils\winutils\derive-utils\tree\Cargo.toml git_bash_path

# Run all tests
cargo test --workspace --manifest-path T:\projects\coreutils\winutils\derive-utils\Cargo.toml git_bash_path
```

Run with verbose output:

```bash
cargo test git_bash_path -- --nocapture
```

Run benchmark tests:

```bash
cargo test git_bash_path -- --ignored
```

### Integration Tests

**From Git Bash or WSL**:

```bash
cd /c/projects/coreutils/winutils/derive-utils
./test_all_git_bash_paths.sh --verbose
```

**From Windows PowerShell**:

```powershell
cd T:\projects\coreutils\winutils\derive-utils
.\test_all_git_bash_paths_simple.ps1 -Verbose
```

### Test Options

**Bash script options**:

- `-v, --verbose`: Enable detailed output
- `-b, --benchmark`: Run performance benchmarks only
- `-o, --output DIR`: Specify output directory for reports
- `-t, --timeout SEC`: Set timeout for individual tests
- `-h, --help`: Show help message

**PowerShell script options**:

- `-Verbose`: Show detailed test output
- `-Help`: Show usage information

## Expected Results

### Success Criteria

All tests should pass with these validations:

1. **Path Normalization**: Git Bash paths (`/c/...`) are converted to Windows paths (`C:\...`)
1. **Output Format**: All outputs use proper Windows path format with drive letters
1. **Functionality**: Core functionality works the same regardless of input path format
1. **Performance**: Git Bash path normalization adds minimal overhead (\<50% increase)
1. **Error Handling**: Invalid paths are properly rejected with appropriate error messages

### Sample Output

```
[PASS] where.exe basic search (Git Bash)
[PASS] where.exe wildcard search (WSL)
[PASS] which.exe basic lookup (Mixed)
[PASS] tree.exe basic tree (Windows)

Overall Results: 32/32 tests passed (100.0%)
✓ All tests passed! Git Bash path normalization is working correctly.
```

## Troubleshooting

### Common Issues

1. **Missing binaries**:

   ```
   ERROR: Missing binaries: where, which, tree
   ```

   **Solution**: Run `cargo build --release` first

1. **Path normalization failures**:

   ```
   [FAIL] where.exe basic search (Git Bash)
   ```

   **Solution**: Check winpath library integration and ensure it's properly linked

1. **Performance issues**:

   ```
   Git Bash overhead: 150%
   ```

   **Solution**: Review path normalization implementation for optimization opportunities

1. **Unicode/special character failures**:

   ```
   [FAIL] tree.exe special chars (Git Bash)
   ```

   **Solution**: Verify UTF-8 handling in path normalization

### Debug Steps

1. **Check binary existence**:

   ```bash
   ls -la T:\projects\coreutils\winutils\derive-utils\*/target/release/*.exe
   ```

1. **Test individual path normalization**:

   ```bash
   ./where.exe "cmd.exe" -R "/c/Windows/System32"
   ```

1. **Verify winpath library**:

   ```bash
   cargo tree --manifest-path where/Cargo.toml | grep winpath
   ```

1. **Run with debug output**:

   ```bash
   RUST_LOG=debug ./where.exe "test.exe" -R "/c/temp"
   ```

## Test Data

### Path Format Examples

The tests use these path format transformations:

| Format   | Example                            |
| -------- | ---------------------------------- |
| Windows  | `C:\Users\Developer\Documents`     |
| Git Bash | `/c/Users/Developer/Documents`     |
| WSL      | `/mnt/c/Users/Developer/Documents` |
| Mixed    | `C:\Users/Developer\Documents`     |

### Test Directory Structure

```
test_root/
├── bin/
│   ├── testapp.exe
│   ├── python.exe
│   └── node.exe
├── usr/local/bin/
│   └── gcc.exe
├── Program Files (x86)/
│   └── Microsoft/Application/
│       └── app.exe
├── git-repos/
│   └── project with spaces/
│       └── src/
│           └── main.c
└── unicode/
    └── café/
        └── 中文/
            └── test/
                └── file.txt
```

## Contributing

When adding new tests:

1. **Follow existing patterns** in the test files
1. **Test both success and failure cases**
1. **Include performance benchmarks** for new features
1. **Update this README** with any new test categories
1. **Ensure cross-platform compatibility** (Git Bash, WSL, PowerShell)

## Performance Targets

- **Path normalization overhead**: \<50% increase over Windows paths
- **Test execution time**: \<30 seconds for full test suite
- **Memory usage**: \<100MB during testing
- **File operations**: \<10ms per path normalization

______________________________________________________________________

**Last Updated**: January 2025
**Compatibility**: Windows 10/11, Git Bash, WSL, PowerShell Core
