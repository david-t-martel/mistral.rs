# Testing & Coverage Framework - WinUtils

This document describes the comprehensive testing and coverage framework for the Windows Utilities project.

## Overview

The winutils project implements a **6-phase pre-commit quality gate** with integrated code coverage analysis to ensure high code quality and maintain test standards across all 80 utilities.

## Code Coverage

### Configuration

Coverage is configured via `.cargo/llvm-cov.toml` with the following settings:

- **Tool**: `cargo-llvm-cov` (LLVM-based, faster and more accurate than tarpaulin)
- **Minimum Coverage**: 70% (enforced as warning)
- **Target Coverage**: 85% (recommended goal)
- **Report Formats**: HTML, LCOV, JSON, Text

### Running Coverage

```bash
# Generate all coverage reports
make coverage

# Check coverage against thresholds (warning-only)
make coverage-check

# Generate HTML report and open in browser
make coverage-html

# Clean coverage artifacts
make coverage-clean
```

### Coverage Locations

After running coverage, reports are generated at:

- **HTML Report**: `target/coverage/html/index.html` (interactive, browse by file)
- **LCOV Report**: `target/coverage/lcov.info` (for CI/CD integration)
- **JSON Report**: `target/coverage/coverage.json` (programmatic access)
- **Text Summary**: Printed to stdout

### Coverage Exclusions

The following patterns are automatically excluded from coverage:

- Test code (`tests/`)
- Benchmark code (`benches/`)
- Derive macros (`#[derive(..)]`)
- Unreachable code (`unreachable!()`)
- Unimplemented code (`unimplemented!()`)
- Panic calls (`panic!()`)
- Platform-specific code not matching current platform
- Debug assertions

### Coverage Thresholds

| Metric        | Minimum | Target | Description            |
| ------------- | ------- | ------ | ---------------------- |
| **Lines**     | 70%     | 85%    | Line coverage          |
| **Functions** | 60%     | 80%    | Function coverage      |
| **Regions**   | 65%     | 80%    | Branch/region coverage |

## Pre-Commit Hooks

### Overview

The pre-commit hook runs a **6-phase quality pipeline** before each commit to ensure code quality. This process typically completes in < 2 minutes.

### Installation

```bash
# Install the pre-commit hook
make install-hooks

# Test the hook without committing
make test-hooks

# Uninstall the hook
make uninstall-hooks
```

**Hook Location**: `.git/hooks/pre-commit`

### Bypassing the Hook (Emergency Only)

```bash
# Only use in emergencies - NOT recommended
git commit --no-verify
```

### The 6-Phase Quality Pipeline

#### Phase 1: Code Formatting

- **Tool**: `cargo fmt`
- **Action**: Auto-formats code if not formatted
- **Failure**: Auto-fixes and stages changes
- **Duration**: < 5 seconds

#### Phase 2: Linting (Clippy)

- **Tool**: `cargo clippy`
- **Action**: Runs linting with `-D warnings` (warnings treated as errors)
- **Failure**: Blocks commit, must fix manually
- **Duration**: 10-20 seconds

#### Phase 3: Functional Tests

- **Tool**: `cargo test`
- **Action**: Runs library and binary tests
- **Failure**: Blocks commit, must fix failing tests
- **Duration**: 30-60 seconds

#### Phase 4: Code Coverage Analysis

- **Tool**: `cargo llvm-cov`
- **Action**: Collects coverage and checks thresholds
- **Failure**: Warning only - does NOT block commit
- **Duration**: 40-80 seconds
- **Note**: Coverage below minimum triggers warning but allows commit

#### Phase 5: Utility Validation

- **Tool**: `make validate-all-77`
- **Action**: Verifies all 77 utilities are functional
- **Failure**: Blocks commit if any utility is missing/broken
- **Duration**: 5-10 seconds

#### Phase 6: Security Audit (Optional)

- **Tool**: `cargo audit`
- **Action**: Checks for known vulnerabilities in dependencies
- **Failure**: Warning only - does NOT block commit
- **Duration**: 5-10 seconds

### Pre-Commit Hook Output

The hook provides color-coded output for each phase:

- ✓ **Green** - Phase passed
- ⚠ **Yellow** - Warning (doesn't block)
- ✗ **Red** - Phase failed (blocks commit)
- ℹ **Blue** - Information message

Example successful output:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  PRE-COMMIT QUALITY GATE - WinUtils Project
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

▶ Phase 1: Code Formatting
✓ Code formatting check passed

▶ Phase 2: Linting (clippy)
✓ Clippy linting passed (no warnings or errors)

▶ Phase 3: Functional Tests
✓ All functional tests passed (25s)

▶ Phase 4: Code Coverage Analysis
✓ Coverage is 78.3% (meets target 85%!) 🎉
ℹ HTML report: target/coverage/html/index.html
ℹ Duration: 45s

▶ Phase 5: Utility Validation
✓ All 77 utilities validated successfully (8s)

▶ Phase 6: Security Audit (Quick Check)
✓ No known security vulnerabilities

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  PRE-COMMIT VALIDATION COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✓ All required checks passed!
ℹ Total validation time: 95s

Summary:
  ✓ Code formatting     : PASSED
  ✓ Clippy linting      : PASSED
  ✓ Functional tests    : PASSED
  ⚠ Coverage analysis   : COMPLETED (78.3%)
  ✓ Utility validation  : PASSED (77/77)

ℹ Proceeding with commit...
```

## Manual Testing

### Running Tests

```bash
# Run all tests (includes all workspaces)
make test

# Run only unit tests
make test-unit

# Run only integration tests
make test-integration

# Run functional tests (used by pre-commit)
make test-functional

# Run specific utility tests
make test-util-ls
make test-util-cat

# Run Git Bash path normalization tests
make test-git-bash-paths

# Run derive utilities tests
make test-derive-utils
```

### Test Organization

Tests are organized as follows:

```
winutils/
├── shared/winpath/
│   └── src/
│       └── tests/              # Unit tests for winpath
├── coreutils/src/*/
│   └── tests/                  # Per-utility unit tests
├── derive-utils/*/
│   └── src/
│       └── tests/              # Derive utility tests
└── tests/                      # Integration tests
```

### Test Types

| Test Type             | Location       | Command                    | Coverage                    |
| --------------------- | -------------- | -------------------------- | --------------------------- |
| **Unit Tests**        | `src/*/tests/` | `make test-unit`           | Individual functions        |
| **Integration Tests** | `tests/`       | `make test-integration`    | Multi-component interaction |
| **Functional Tests**  | Inline         | `make test-functional`     | Binary execution            |
| **Path Tests**        | Root           | `make test-git-bash-paths` | Path normalization          |
| **Validation Tests**  | Scripts        | `make validate-all-77`     | Binary existence            |

## Continuous Integration

### GitHub Actions Integration

The coverage framework integrates seamlessly with GitHub Actions:

```yaml
# .github/workflows/test.yml
- name: Run tests with coverage
  run: |
    make coverage

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: target/coverage/lcov.info
    fail_ci_if_error: false  # Coverage is warning-only
```

### CI Pipeline

The full CI pipeline (`make ci`) includes:

1. `make clean` - Clean all artifacts
1. `make check` - Format, clippy, audit
1. `make build-winpath` - Build critical dependency
1. `make release` - Build all utilities
1. `make test` - Run all tests
1. `make coverage` - Generate coverage reports
1. `make validate` - Validate all utilities
1. `make package` - Create distribution

## Best Practices

### Writing Tests

1. **Test behavior, not implementation**

   ```rust
   // Good - tests behavior
   #[test]
   fn test_normalize_dos_path() {
       let result = normalize_path("C:\\Windows\\System32");
       assert_eq!(result, "C:/Windows/System32");
   }

   // Bad - tests implementation details
   #[test]
   fn test_uses_path_replace() {
       let result = normalize_path("C:\\test");
       assert!(result.contains("/"));  // Too implementation-specific
   }
   ```

1. **Use descriptive test names**

   ```rust
   #[test]
   fn test_wsl_path_converts_to_windows_drive() { ... }  // ✓ Clear

   #[test]
   fn test_path() { ... }  // ✗ Unclear
   ```

1. **Test edge cases**

   - Empty inputs
   - Very long paths (>260 chars)
   - Unicode characters
   - Special characters (spaces, symbols)
   - Path separators (/, , mixed)

1. **Keep tests fast**

   - Avoid sleeping/waiting
   - Mock external dependencies
   - Use in-memory file systems when possible

### Improving Coverage

To improve coverage in a specific area:

1. **Identify uncovered code**

   ```bash
   make coverage-html
   # Open target/coverage/html/index.html
   # Navigate to specific file
   # Red/orange lines are uncovered
   ```

1. **Write targeted tests**

   - Focus on red (uncovered) lines
   - Add tests for error paths
   - Test boundary conditions

1. **Verify improvement**

   ```bash
   make coverage-check
   # Check if coverage increased
   ```

## Troubleshooting

### Coverage Issues

**Issue**: Coverage reports are empty

- **Solution**: Ensure tests are actually running (`make test-functional`)
- **Solution**: Check that llvm-tools-preview is installed: `rustup component add llvm-tools-preview`

**Issue**: Coverage is lower than expected

- **Solution**: Check `.cargo/llvm-cov.toml` exclusions
- **Solution**: Verify tests are in correct locations (`src/*/tests/` or `tests/`)

**Issue**: "cargo-llvm-cov not found"

- **Solution**: Install with `cargo install cargo-llvm-cov`

### Pre-Commit Hook Issues

**Issue**: Hook doesn't run

- **Solution**: Verify installation: `ls -la .git/hooks/pre-commit`
- **Solution**: Check permissions: `chmod +x .git/hooks/pre-commit`
- **Solution**: Reinstall: `make install-hooks`

**Issue**: Hook runs but git commit still fails

- **Solution**: Check git status for untracked changes
- **Solution**: Review hook output for specific failure

**Issue**: Hook is too slow

- **Solution**: Coverage phase is the slowest - consider optimizing tests
- **Solution**: Run `make test-fast` to see baseline test speed
- **Solution**: Ensure sccache is enabled for faster compilation

**Issue**: Tests pass locally but fail in pre-commit

- **Solution**: Run hook manually: `make test-hooks`
- **Solution**: Check for uncommitted changes affecting tests
- **Solution**: Ensure all dependencies are up to date

## Performance Targets

| Operation                | Target Time | Typical Time |
| ------------------------ | ----------- | ------------ |
| **make coverage**        | < 90s       | 60-80s       |
| **make coverage-check**  | < 30s       | 15-25s       |
| **make test-functional** | < 60s       | 30-50s       |
| **Pre-commit hook**      | < 2m        | 90-120s      |
| **Full CI pipeline**     | < 10m       | 5-8m         |

## Additional Resources

- **Coverage Config**: `.cargo/llvm-cov.toml`
- **Pre-Commit Script**: `scripts/pre-commit-hook.sh`
- **Makefile Targets**: `Makefile.coverage`
- **CI Configuration**: `.github/workflows/test.yml`

## Summary

The testing and coverage framework provides:

- ✅ **Automated quality gates** via pre-commit hooks
- ✅ **Comprehensive coverage reporting** (HTML, LCOV, JSON)
- ✅ **Fast feedback** (\<2 minutes for pre-commit)
- ✅ **CI/CD integration** (GitHub Actions, Codecov)
- ✅ **Coverage tracking** (70% minimum, 85% target)
- ✅ **Multi-phase validation** (6 phases: format, lint, test, coverage, validate, audit)

This ensures high code quality while maintaining rapid development velocity.
