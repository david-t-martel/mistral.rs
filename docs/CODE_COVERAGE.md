# Code Coverage Guide for mistral.rs

## Overview

This project uses `cargo-llvm-cov` for accurate code coverage measurement and Codecov for tracking coverage trends over time.

### Coverage Goals

- **Overall Project**: 70% minimum
- **New Code**: 80% minimum
- **Critical Modules**: 90%+ (engine, inference, safety-critical paths)
- **Public APIs**: 100% (all public functions tested)

______________________________________________________________________

## Installation

### Install Coverage Tools

```bash
# Install llvm-tools-preview component
rustup component add llvm-tools-preview

# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Or use the Makefile
make install-coverage-tools
```

### Verify Installation

```bash
cargo llvm-cov --version
```

______________________________________________________________________

## Generating Coverage Reports Locally

### Quick Summary

```bash
# Text summary in terminal
cargo llvm-cov --workspace --all-features --summary-only

# Or use Makefile
make test-coverage-text
```

**Example Output:**

```
Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
mistralrs-core/src/lib.rs          45                 5    88.89%          12                 1    91.67%         120                12    90.00%          25                 3    88.00%
...
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
TOTAL                            2345               234    90.02%         456                23    94.96%        5678               456    91.97%        1234                89    92.79%
```

### HTML Report (Interactive)

```bash
# Generate and open in browser
cargo llvm-cov --workspace --all-features --open

# Or use Makefile
make test-coverage-open
```

This opens an interactive HTML report showing:

- Line-by-line coverage
- Branch coverage
- Function coverage
- Color-coded source files (green = covered, red = not covered)

### HTML Report (Manual Open)

```bash
# Generate HTML report
cargo llvm-cov --workspace --all-features --html

# Or use Makefile
make test-coverage

# Then open in browser
start target/llvm-cov/html/index.html  # Windows
open target/llvm-cov/html/index.html   # macOS
xdg-open target/llvm-cov/html/index.html  # Linux
```

### LCOV Format (For CI/Tools)

```bash
# Generate lcov.info
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

# Or use Makefile
make test-coverage-lcov
```

LCOV format is used by:

- Codecov
- Coveralls
- IDE plugins (VSCode, IntelliJ)
- Code review tools

### JSON Format (For Tooling)

```bash
# Generate coverage.json
cargo llvm-cov --workspace --all-features --json --output-path coverage.json

# Or use Makefile
make test-coverage-json
```

______________________________________________________________________

## Coverage for Specific Packages

### Single Package

```bash
cargo llvm-cov -p mistralrs-core --all-features --summary-only
cargo llvm-cov -p mistralrs-agent-tools --all-features --summary-only
```

### Multiple Packages

```bash
cargo llvm-cov -p mistralrs-core -p mistralrs-quant --all-features --summary-only
```

### Exclude Packages

```bash
cargo llvm-cov --workspace --exclude mistralrs-server --all-features --summary-only
```

______________________________________________________________________

## Advanced Usage

### Coverage with Feature Flags

```bash
# Specific features
cargo llvm-cov --features "cuda,metal" --summary-only

# No default features
cargo llvm-cov --no-default-features --summary-only

# All features (recommended)
cargo llvm-cov --all-features --summary-only
```

### Coverage for Integration Tests Only

```bash
cargo llvm-cov --test '*' --summary-only
```

### Coverage for Doc Tests

```bash
cargo llvm-cov --doc --summary-only
```

### Coverage with Specific Test

```bash
cargo llvm-cov --test integration_test --summary-only
```

### Clean Coverage Data

```bash
# Clean previous coverage data
cargo llvm-cov clean

# Then run coverage again
cargo llvm-cov --workspace --all-features --summary-only
```

______________________________________________________________________

## CI Integration (GitHub Actions)

### Automatic Coverage Upload

Our CI workflow automatically:

1. Generates coverage on Linux (fastest platform)
1. Uploads to Codecov on every push/PR
1. Comments on PRs with coverage changes

### Workflow Configuration

See `.github/workflows/ci.yml` for the `coverage` job:

```yaml
coverage:
  name: Code Coverage
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: llvm-tools-preview
    - uses: Swatinem/rust-cache@v2
    - uses: taiki-e/install-action@cargo-llvm-cov
    - run: cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
    - uses: codecov/codecov-action@v4
      with:
        files: lcov.info
        token: ${{ secrets.CODECOV_TOKEN }}
```

### Required Secrets

**`CODECOV_TOKEN`**: Required for uploading coverage to Codecov.

**Setup**:

1. Go to https://codecov.io/gh/YOUR_ORG/mistral.rs
1. Get repository upload token
1. Add to GitHub: Settings → Secrets → Actions → New secret
1. Name: `CODECOV_TOKEN`
1. Value: (paste token from Codecov)

______________________________________________________________________

## Codecov Dashboard

### Accessing Reports

**Project Dashboard**: https://codecov.io/gh/YOUR_ORG/mistral.rs

**Features**:

- Overall coverage trends
- Per-commit coverage
- Per-component coverage
- Coverage diffs in PRs
- Coverage sunburst visualization
- File-level coverage browser

### Coverage Badges

Add to README.md:

```markdown
[![codecov](https://codecov.io/gh/YOUR_ORG/mistral.rs/branch/master/graph/badge.svg)](https://codecov.io/gh/YOUR_ORG/mistral.rs)
```

### PR Comments

Codecov automatically comments on PRs with:

- Coverage change (±%)
- Files with coverage changes
- Links to detailed reports
- Pass/fail status based on targets

**Example Comment**:

```
# Codecov Report

> Merging #123 (abc1234) into master (def5678) will increase coverage by 2.34%.

@@            Coverage Diff             @@
##           master     #123      +/-   ##
==========================================
+ Coverage   87.65%   89.99%   +2.34%
==========================================
  Files          42       43       +1
  Lines        5678     5789     +111
==========================================
+ Hits         4978     5210     +232
+ Misses        700      579     -121
```

______________________________________________________________________

## Interpreting Coverage

### Coverage Metrics

**Line Coverage**: Percentage of lines executed

- Good: > 80%
- Acceptable: 70-80%
- Needs improvement: < 70%

**Branch Coverage**: Percentage of branches taken

- Good: > 80%
- Important for: if/else, match, loops

**Function Coverage**: Percentage of functions called

- Target: 100% for public APIs
- Important for: library interfaces

### What to Cover

**MUST Cover**:

- ✅ All public APIs
- ✅ Error handling paths
- ✅ Edge cases and boundary conditions
- ✅ Critical business logic
- ✅ Safety-critical code paths

**SHOULD Cover**:

- ✅ Private helper functions
- ✅ Internal utilities
- ✅ Configuration parsing
- ✅ Data transformations

**MAY Skip** (Low ROI):

- ⚠️ Trivial getters/setters
- ⚠️ Generated code
- ⚠️ External bindings (FFI)
- ⚠️ Platform-specific code (test on that platform)

### Coverage Gaps

Low coverage usually indicates:

1. **Untested error paths**: Add error case tests
1. **Dead code**: Remove or document why it exists
1. **Complex conditionals**: Simplify or add branch tests
1. **Hard-to-test code**: Refactor for testability

______________________________________________________________________

## Best Practices

### 1. Run Coverage Locally Before PR

```bash
# Quick check
make test-coverage-text

# Detailed review
make test-coverage-open
```

### 2. Focus on Critical Paths First

Prioritize coverage for:

- Security-sensitive code
- Data validation and parsing
- Core algorithms
- Public APIs
- Error handling

### 3. Don't Chase 100% Coverage

**Goal**: Meaningful tests, not just coverage

- 100% coverage doesn't mean bug-free
- Focus on testing behavior, not lines
- Quality over quantity

### 4. Review Coverage Reports Regularly

```bash
# Weekly/monthly review
make test-coverage-open

# Check for:
# - Decreasing coverage trends
# - New uncovered code
# - Critical paths without tests
```

### 5. Use Coverage to Find Missing Tests

**Red lines in HTML report = missing tests**

```bash
# Generate HTML report
make test-coverage-open

# Look for:
# - Red lines in critical functions
# - Untested error paths
# - Uncovered branches
```

### 6. Ignore Noise in Coverage

Codecov is configured to ignore:

- `tests/` directories
- `benches/` directories
- `examples/` directories
- Test files (`*_test.rs`, `test_*.rs`)

See `codecov.yml` for full ignore list.

______________________________________________________________________

## Troubleshooting

### "Command not found: cargo-llvm-cov"

**Solution**:

```bash
cargo install cargo-llvm-cov
```

### "Error: llvm-tools-preview not installed"

**Solution**:

```bash
rustup component add llvm-tools-preview
```

### Coverage is 0% or Very Low

**Causes**:

1. No tests ran
1. Tests didn't execute the code
1. Coverage data was cleaned

**Solutions**:

```bash
# Clean and regenerate
cargo llvm-cov clean
cargo llvm-cov --workspace --all-features --summary-only

# Verify tests run
cargo test --workspace --all-features
```

### Coverage Differs Between Local and CI

**Causes**:

1. Different feature flags
1. Platform-specific code
1. Different test execution

**Solutions**:

- Use `--all-features` locally and in CI
- Check platform-specific `#[cfg]` blocks
- Run on same platform as CI (Linux)

### Codecov Upload Fails

**Causes**:

1. Missing `CODECOV_TOKEN` secret
1. Network issues
1. Invalid token

**Solutions**:

```bash
# Check token exists in GitHub Settings → Secrets
# Try manual upload:
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
curl -Os https://uploader.codecov.io/latest/windows/codecov.exe
.\codecov.exe -f lcov.info -t YOUR_TOKEN
```

### "No coverage data found"

**Solution**:

```bash
# Ensure tests actually run
cargo test --workspace --all-features -- --test-threads=1

# Then generate coverage
cargo llvm-cov clean
cargo llvm-cov --workspace --all-features --summary-only
```

______________________________________________________________________

## Coverage Workflow

### Before Opening PR

```bash
# 1. Run tests
cargo test --workspace --all-features

# 2. Check coverage
make test-coverage-text

# 3. Review detailed report
make test-coverage-open

# 4. Add tests for uncovered critical code
# (focus on red lines in HTML report)

# 5. Re-check coverage
make test-coverage-text
```

### During PR Review

1. **Check Codecov comment** on PR
1. **Review coverage diff**: What changed?
1. **Investigate coverage drops**: Why did coverage decrease?
1. **Add tests if needed**: Cover new critical code

### After Merging

1. **Monitor dashboard**: https://codecov.io/gh/YOUR_ORG/mistral.rs
1. **Track trends**: Is coverage increasing or decreasing?
1. **Set goals**: Target 70% overall, 80% new code

______________________________________________________________________

## Quick Reference

### Common Commands

```bash
# Text summary
make test-coverage-text

# HTML report (opens in browser)
make test-coverage-open

# Generate lcov.info for tools
make test-coverage-lcov

# CI format
make test-coverage-ci

# Clean coverage data
cargo llvm-cov clean

# Install tools
make install-coverage-tools
```

### Makefile Targets

```bash
make install-coverage-tools  # Install cargo-llvm-cov
make test-coverage           # Generate HTML report
make test-coverage-open      # Generate and open HTML
make test-coverage-lcov      # Generate LCOV format
make test-coverage-json      # Generate JSON format
make test-coverage-text      # Text summary only
make test-coverage-ci        # CI format (LCOV)
```

### Key Files

- **`codecov.yml`**: Codecov configuration
- **`lcov.info`**: LCOV format coverage data (gitignored)
- **`coverage.json`**: JSON format coverage data (gitignored)
- **`target/llvm-cov/html/`**: HTML coverage reports (gitignored)

______________________________________________________________________

## Coverage Targets by Component

| Component             | Target  | Current | Status |
| --------------------- | ------- | ------- | ------ |
| mistralrs-core        | 80%     | TBD     | ⏳     |
| mistralrs-agent-tools | 85%     | TBD     | ⏳     |
| mistralrs-quant       | 75%     | TBD     | ⏳     |
| mistralrs-vision      | 75%     | TBD     | ⏳     |
| mistralrs-audio       | 75%     | TBD     | ⏳     |
| mistralrs-server      | 70%     | TBD     | ⏳     |
| mistralrs-mcp         | 80%     | TBD     | ⏳     |
| mistralrs-tui         | 70%     | TBD     | ⏳     |
| **Overall**           | **70%** | **TBD** | **⏳** |

______________________________________________________________________

## Resources

- **cargo-llvm-cov**: https://github.com/taiki-e/cargo-llvm-cov
- **Codecov**: https://codecov.io
- **LLVM Coverage**: https://llvm.org/docs/CommandGuide/llvm-cov.html
- **Rust Testing Book**: https://doc.rust-lang.org/book/ch11-00-testing.html

______________________________________________________________________

**Document Version**: 1.0\
**Last Updated**: 2025-01-05\
**Maintained by**: Testing Infrastructure Team
