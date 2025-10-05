# CI/CD Pipeline Documentation

## Table of Contents

1. [Overview](#overview)
2. [Workflow Architecture](#workflow-architecture)
3. [Pipeline Stages](#pipeline-stages)
4. [Job Descriptions](#job-descriptions)
5. [Caching Strategy](#caching-strategy)
6. [Environment Variables & Secrets](#environment-variables--secrets)
7. [Debugging Failed CI Runs](#debugging-failed-ci-runs)
8. [Adding New CI Checks](#adding-new-ci-checks)
9. [Performance Optimization](#performance-optimization)
10. [Best Practices](#best-practices)

---

## Overview

The mistral.rs project uses GitHub Actions for Continuous Integration and Continuous Deployment (CI/CD). The CI pipeline ensures code quality, correctness, and compatibility across multiple platforms before merging changes.

### Pipeline Objectives

1. **Quality Assurance**: Catch bugs, formatting issues, and code quality problems
2. **Cross-Platform Compatibility**: Verify code works on Linux, macOS, and Windows
3. **Performance Monitoring**: Track and prevent performance regressions
4. **Security**: Detect vulnerabilities and security issues
5. **Documentation**: Ensure documentation builds correctly

### Key Features

- ✅ **Multi-platform testing** (Ubuntu, macOS, Windows)
- ✅ **Parallel job execution** for faster feedback
- ✅ **Intelligent caching** to reduce build times
- ✅ **Code coverage reporting** with Codecov
- ✅ **Security auditing** with cargo-audit
- ✅ **MSRV (Minimum Supported Rust Version) checking**
- ✅ **Fail-fast quick checks** for rapid feedback

---

## Workflow Architecture

### Main Workflow File

**Location**: `.github/workflows/ci.yml`

**Triggers**:
- `push` to `master` branch
- `pull_request` targeting `master` branch
- Weekly `schedule` (Monday at midnight UTC)
- Manual `workflow_dispatch`

### Pipeline Dependency Graph

```
quick-check (fail-fast)
    |
    ├─→ check (matrix: ubuntu, macos, windows)
    ├─→ test (matrix: ubuntu, macos, windows)
    ├─→ coverage (ubuntu only)
    ├─→ docs
    ├─→ msrv
    ├─→ integration
    |
typos (independent)
security-audit (independent)
    |
    └─→ ci-complete (final gate)
```

**Parallel Execution**: Most jobs run in parallel after `quick-check` passes, reducing total CI time.

---

## Pipeline Stages

### Stage 1: Quick Check (Fail Fast)

**Purpose**: Catch obvious issues immediately without wasting resources.

**Jobs**:
- `quick-check`: Formatting and linting

**Runtime**: ~2-3 minutes

**Strategy**: If this fails, all subsequent jobs are cancelled (fail-fast behavior).

### Stage 2: Core Validation (Parallel)

**Purpose**: Verify code correctness across platforms and configurations.

**Jobs**:
- `check`: Build verification
- `test`: Test suite execution
- `coverage`: Code coverage collection
- `docs`: Documentation build
- `msrv`: Minimum Rust version check

**Runtime**: ~10-15 minutes (parallel)

### Stage 3: Additional Checks (Parallel)

**Purpose**: Quality and security verification.

**Jobs**:
- `typos`: Spell checking
- `security-audit`: Dependency vulnerability scanning

**Runtime**: ~1-2 minutes

### Stage 4: Integration Tests

**Purpose**: Test cross-module interactions.

**Jobs**:
- `integration`: Integration test suite

**Runtime**: ~5-10 minutes

**Dependencies**: Runs after `check` and `test` pass.

### Stage 5: Final Gate

**Purpose**: Ensure all critical checks passed.

**Jobs**:
- `ci-complete`: Status aggregation

**Runtime**: < 1 minute

**Behavior**: Fails if any required job failed.

---

## Job Descriptions

### `quick-check`

**Runs on**: `ubuntu-latest`

**Purpose**: Fast formatting and linting checks.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable) with rustfmt and clippy
3. Run `cargo fmt --all -- --check`
4. Run `cargo clippy --workspace --all-targets -- -D warnings`

**Key Configuration**:
- No caching (too fast to benefit)
- Denies all clippy warnings (`-D warnings`)
- Checks workspace and all targets (lib, bins, tests, examples)

**Typical Failures**:
- Formatting issues (run `cargo fmt` locally)
- Clippy warnings (run `cargo clippy --fix`)

---

### `check`

**Runs on**: `ubuntu-latest`, `windows-latest`, `macOS-latest`

**Purpose**: Verify code compiles on all platforms.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable)
3. Setup Rust cache
4. Run `cargo check --workspace --all-targets --all-features`

**Key Configuration**:
- Matrix strategy: 3 platforms × 1 Rust version = 3 jobs
- `fail-fast: false`: Continue even if one platform fails
- Rust cache with platform-specific keys
- Checks all features and all targets

**Typical Failures**:
- Platform-specific compilation errors
- Missing feature flags
- Dependency issues

---

### `test`

**Runs on**: `ubuntu-latest`, `windows-latest`, `macOS-latest`

**Purpose**: Run the complete test suite on all platforms.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable)
3. Setup Rust cache
4. Run `cargo test --workspace --all-features -- --nocapture --test-threads=4`
5. Run `cargo test --workspace --doc`

**Key Configuration**:
- Matrix strategy: 3 platforms
- Environment variables:
  - `TESTS_HF_TOKEN`: HuggingFace token for integration tests
  - `RUST_TEST_THREADS=4`: Parallel test execution
- Captures test output with `--nocapture`
- Separate doc tests execution

**Typical Failures**:
- Test failures (unit, integration, or doc tests)
- Platform-specific test issues
- Timing-dependent test flakiness
- Missing test dependencies or data

---

### `coverage`

**Runs on**: `ubuntu-latest` (only)

**Purpose**: Collect code coverage and upload to Codecov.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable) with llvm-tools-preview
3. Setup Rust cache
4. Install cargo-llvm-cov
5. Generate coverage data: `cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info`
6. Upload to Codecov

**Key Configuration**:
- Runs only on Linux (llvm-cov is fastest on Linux)
- Uses `cargo-llvm-cov` for accurate coverage
- Uploads to Codecov with token authentication
- `fail_ci_if_error: false`: Don't fail CI if upload fails

**Typical Failures**:
- Coverage collection errors
- Codecov upload issues (transient network errors)
- Token authentication problems

**Coverage Metrics**:
- **Target**: 70% overall, 80% for new code
- **View Reports**: https://codecov.io/gh/your-org/mistral.rs

---

### `docs`

**Runs on**: `ubuntu-latest`

**Purpose**: Ensure documentation builds without errors or warnings.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable)
3. Setup Rust cache
4. Run `cargo doc --workspace --all-features --no-deps`

**Key Configuration**:
- `RUSTDOCFLAGS: "-D warnings"`: Treat doc warnings as errors
- `--no-deps`: Only build docs for workspace crates
- `--all-features`: Build docs for all feature combinations

**Typical Failures**:
- Broken doc links
- Invalid doc syntax
- Missing doc comments on public items
- Intra-doc link errors

---

### `typos`

**Runs on**: `ubuntu-latest`

**Purpose**: Check for typos in code, comments, and documentation.

**Steps**:
1. Checkout code
2. Run typos checker with `.typos.toml` config

**Key Configuration**:
- Uses `crate-ci/typos` action
- Configuration file: `.typos.toml`
- Checks all text files (Rust, Markdown, YAML, etc.)

**Typical Failures**:
- Misspelled words in comments or docs
- False positives (add to `.typos.toml` ignore list)

**To fix**:
1. Correct the typo, or
2. Add to ignore list in `.typos.toml` if it's a technical term or false positive

---

### `msrv`

**Runs on**: `ubuntu-latest`

**Purpose**: Verify code compiles with the minimum supported Rust version.

**Steps**:
1. Checkout code
2. Install Rust toolchain (1.86.0 - MSRV)
3. Setup Rust cache
4. Run `cargo check --workspace --all-features`

**Key Configuration**:
- MSRV: Rust 1.86.0
- Checks all features to ensure compatibility
- Fails if newer Rust features are used

**Typical Failures**:
- Using features from newer Rust versions
- Dependencies requiring newer Rust
- New syntax not available in MSRV

**To fix**:
1. Either increase MSRV (update in `Cargo.toml` and CI), or
2. Use alternative approach compatible with MSRV

---

### `security-audit`

**Runs on**: `ubuntu-latest`

**Purpose**: Check for known security vulnerabilities in dependencies.

**Steps**:
1. Checkout code
2. Run `cargo-audit` via rustsec/audit-check action

**Key Configuration**:
- Uses RustSec Advisory Database
- Authenticated with GITHUB_TOKEN
- Fails on any vulnerability

**Typical Failures**:
- Known CVEs in dependencies
- Unmaintained dependencies
- Yanked crate versions

**To fix**:
1. Update vulnerable dependencies: `cargo update`
2. If no fix available, consider alternatives or temporary allowlist

---

### `integration`

**Runs on**: `ubuntu-latest`

**Purpose**: Run integration tests that test cross-module interactions.

**Steps**:
1. Checkout code
2. Install Rust toolchain (stable)
3. Setup Rust cache
4. Run `cargo test --test '*' --all-features` (if tests/ directory exists)

**Key Configuration**:
- Depends on `check` and `test` passing
- Only runs if `tests/` directory exists
- Tests cross-crate and cross-module interactions

**Typical Failures**:
- Integration test failures
- Module interface mismatches
- Missing test fixtures or data

---

### `ci-complete`

**Runs on**: `ubuntu-latest`

**Purpose**: Final gate ensuring all required checks passed.

**Steps**:
1. Check status of all dependent jobs
2. Fail if any required job failed
3. Success if all required jobs passed

**Key Configuration**:
- `needs`: Lists all critical jobs
- `if: always()`: Always runs, even if dependencies fail
- Explicit status checking for each job

**Required Jobs**:
- `quick-check`
- `check`
- `test`
- `docs`
- `typos`
- `msrv`

**Optional Jobs** (don't block merge):
- `coverage` (informational only)
- `security-audit` (can be temporarily ignored)

---

## Caching Strategy

### Rust Build Cache

**Tool**: `Swatinem/rust-cache@v2`

**What's cached**:
- Cargo registry index
- Downloaded crate sources
- Compiled dependencies
- Build artifacts

**Cache keys**:
- Platform-specific: `${{ matrix.os }}-<job-name>`
- Automatically invalidates on `Cargo.lock` changes
- Automatically invalidates on toolchain changes

**Benefits**:
- **~5-10x faster builds** after first run
- Reduced GitHub Actions minutes usage
- Faster feedback for developers

**Example Configuration**:
```yaml
- name: Setup Rust cache
  uses: Swatinem/rust-cache@v2
  with:
    key: ${{ matrix.os }}-test
```

### Cache Behavior

1. **Cache Hit**: Uses cached dependencies and artifacts
2. **Cache Miss**: Downloads and builds everything, then caches
3. **Cache Invalidation**: Automatic on dependency or toolchain changes

**Manual Cache Clearing** (if needed):
1. Go to Actions tab
2. Click "Caches" in sidebar
3. Delete specific or all caches

---

## Environment Variables & Secrets

### Environment Variables

#### `CARGO_TERM_COLOR`
**Value**: `always`  
**Purpose**: Colorized cargo output in CI logs  
**Set in**: Workflow-level `env`

#### `RUST_BACKTRACE`
**Value**: `1`  
**Purpose**: Full backtraces on panics for better debugging  
**Set in**: Workflow-level `env`

#### `RUST_TEST_THREADS`
**Value**: `4`  
**Purpose**: Parallel test execution  
**Set in**: `test` job `env`

#### `RUSTDOCFLAGS`
**Value**: `"-D warnings"`  
**Purpose**: Treat doc warnings as errors  
**Set in**: `docs` job `env`

### Secrets

#### `HF_TOKEN`
**Purpose**: HuggingFace API token for model downloading in tests  
**Usage**: Set as `TESTS_HF_TOKEN` in `test` and `coverage` jobs  
**Setup**:  
1. Go to repository Settings → Secrets → Actions
2. Add new secret: `HF_TOKEN`
3. Value: Your HuggingFace access token

**How to generate**:
1. Go to https://huggingface.co/settings/tokens
2. Create new token with read access
3. Copy and paste into GitHub secret

#### `CODECOV_TOKEN`
**Purpose**: Authentication for uploading coverage to Codecov  
**Usage**: Set in `coverage` job for codecov-action  
**Setup**:  
1. Go to https://codecov.io, link your GitHub repo
2. Get repository upload token
3. Add as GitHub secret: `CODECOV_TOKEN`

#### `GITHUB_TOKEN`
**Purpose**: Automatic token for GitHub API access  
**Usage**: Used by `security-audit` job  
**Setup**: Automatically provided by GitHub Actions, no setup needed

---

## Debugging Failed CI Runs

### Step-by-Step Debugging Guide

#### 1. Identify the Failing Job

1. Go to the Pull Request or commit
2. Click "Checks" or "Details" next to the failed check
3. Identify which job(s) failed

#### 2. Read the Error Logs

1. Click on the failed job name
2. Expand the failed step
3. Read the error message and stack trace

#### 3. Reproduce Locally

```bash
# For quick-check failures
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# For test failures
cargo test --workspace --all-features

# For specific job
make ci-full  # Runs full CI pipeline locally
```

#### 4. Common Failure Patterns

**Formatting Issues**:
```bash
# Check formatting
cargo fmt --all -- --check

# Fix formatting
cargo fmt --all
```

**Clippy Warnings**:
```bash
# Check lints
cargo clippy --workspace --all-targets -- -D warnings

# Auto-fix where possible
cargo clippy --workspace --all-targets --fix
```

**Test Failures**:
```bash
# Run specific test
cargo test test_name -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run single-threaded (for debugging race conditions)
cargo test -- --test-threads=1
```

**Platform-Specific Failures**:
- Check for hardcoded paths (use `std::path`)
- Check for Unix-only commands
- Use conditional compilation:
  ```rust
  #[cfg(target_os = "windows")]
  fn windows_impl() { ... }
  
  #[cfg(not(target_os = "windows"))]
  fn unix_impl() { ... }
  ```

#### 5. Re-run Failed Jobs

1. Go to the failed CI run
2. Click "Re-run failed jobs" (GitHub Actions)
3. Or push a new commit to trigger full re-run

### Debugging Tips

**Enable debug logging**:
```yaml
- name: Run tests with debug
  env:
    RUST_LOG: debug
  run: cargo test -- --nocapture
```

**Add diagnostic steps**:
```yaml
- name: Debug environment
  run: |
    rustc --version
    cargo --version
    env | sort
```

**Test with exact CI environment**:

Use Docker to replicate CI environment locally:
```bash
# Ubuntu CI environment
docker run -it -v $(pwd):/workspace -w /workspace rust:latest bash

# Inside container
apt-get update && apt-get install -y build-essential
cargo test --workspace --all-features
```

---

## Adding New CI Checks

### Adding a New Job

1. **Edit `.github/workflows/ci.yml`**

2. **Add job definition**:
```yaml
my-new-check:
  name: My New Check
  runs-on: ubuntu-latest
  needs: quick-check  # Optional: run after quick-check passes
  steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable
    
    - name: Setup cache
      uses: Swatinem/rust-cache@v2
      with:
        key: my-check
    
    - name: Run my check
      run: cargo my-command
```

3. **Update `ci-complete` dependencies**:
```yaml
ci-complete:
  needs: [...existing..., my-new-check]
```

4. **Test locally**:
```bash
# Run the equivalent command locally
cargo my-command
```

5. **Commit and push**:
```bash
git add .github/workflows/ci.yml
git commit -m "ci: add new check for X"
git push
```

### Example: Adding Benchmark Regression Check

```yaml
benchmark-regression:
  name: Benchmark Regression
  runs-on: ubuntu-latest
  needs: [check, test]
  steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable
    
    - name: Setup cache
      uses: Swatinem/rust-cache@v2
      with:
        key: bench
    
    - name: Run benchmarks
      run: cargo bench --workspace
    
    - name: Store benchmark result
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: target/criterion/baseline/estimates.json
        fail-on-alert: true
        alert-threshold: '150%'  # Fail if 50% slower
```

---

## Performance Optimization

### Current Optimizations

1. **Parallel Job Execution**
   - Most jobs run in parallel after `quick-check`
   - Reduces total CI time from ~30min sequential to ~15min parallel

2. **Rust Build Caching**
   - Uses `Swatinem/rust-cache@v2`
   - ~5-10x faster builds after first run

3. **Fail-Fast Quick Check**
   - Catches obvious issues in 2-3 minutes
   - Cancels subsequent jobs if fails

4. **Platform-Specific Caching**
   - Separate caches for Linux, macOS, Windows
   - Prevents cache invalidation across platforms

5. **Minimal Job Dependencies**
   - Only essential dependencies specified
   - Maximizes parallelism

6. **Linux-Only Coverage**
   - Coverage collection is fastest on Linux
   - Avoids redundant coverage on other platforms

### Further Optimizations (Future)

1. **Conditional Job Execution**
   - Skip tests if only docs changed
   - Skip benchmarks on draft PRs

   ```yaml
   test:
     if: "!contains(github.event.head_commit.message, '[skip tests]')"
   ```

2. **Cargo Nextest** (Faster Test Runner)
   ```yaml
   - name: Install nextest
     uses: taiki-e/install-action@nextest
   
   - name: Run tests
     run: cargo nextest run --workspace
   ```

3. **Sparse Registry** (Faster Dependency Resolution)
   ```yaml
   env:
     CARGO_REGISTRIES_CRATES_IO_PROTOCOL: sparse
   ```

4. **Incremental Compilation** (Faster Rebuilds)
   ```yaml
   env:
     CARGO_INCREMENTAL: 1
   ```

5. **Matrix Sharding** (Parallel Test Execution)
   ```yaml
   test:
     strategy:
       matrix:
         shard: [1, 2, 3, 4]
     steps:
       - run: cargo test --workspace -- --test-threads=1 \
               --shard-index ${{ matrix.shard }} \
               --shard-total 4
   ```

### Performance Metrics

**Current CI Times** (approximate):
- **Quick Check**: 2-3 minutes
- **Check (per platform)**: 5-7 minutes
- **Test (per platform)**: 8-12 minutes
- **Coverage**: 10-15 minutes
- **Total (parallel)**: 12-18 minutes

**Target CI Times**:
- **Quick Check**: < 2 minutes
- **Check**: < 5 minutes
- **Test**: < 8 minutes
- **Coverage**: < 10 minutes
- **Total**: < 12 minutes

---

## Best Practices

### For Contributors

✅ **DO**:
1. Run tests locally before pushing: `cargo test --workspace`
2. Run formatting before committing: `cargo fmt --all`
3. Fix clippy warnings: `cargo clippy --workspace --all-targets --fix`
4. Check CI status before requesting review
5. Re-run failed jobs if transient (network errors, etc.)
6. Add tests for new features and bug fixes
7. Update documentation for CI changes

❌ **DON'T**:
1. Don't ignore CI failures
2. Don't push unformatted code
3. Don't disable clippy warnings without good reason
4. Don't add dependencies without considering CI impact
5. Don't merge PRs with failing CI
6. Don't skip tests with `#[ignore]` to make CI pass

### For Maintainers

✅ **DO**:
1. Monitor CI performance metrics
2. Keep GitHub Actions up to date
3. Review and merge dependabot updates
4. Investigate flaky tests immediately
5. Keep MSRV updated (but not too aggressively)
6. Document new CI requirements in this file
7. Use branch protection rules to require CI checks

❌ **DON'T**:
1. Don't add jobs without caching
2. Don't add redundant checks
3. Don't ignore security audit failures
4. Don't merge with failing CI
5. Don't add excessive matrix dimensions

### Branch Protection Rules

Recommended settings for `master` branch:

```yaml
Required status checks:
  - quick-check
  - check (ubuntu-latest)
  - check (windows-latest)
  - check (macOS-latest)
  - test (ubuntu-latest)
  - test (windows-latest)
  - test (macOS-latest)
  - docs
  - typos
  - msrv
  - ci-complete

Require branches to be up to date: true
Require review from Code Owners: true
Require signed commits: false (optional)
```

---

## Troubleshooting Guide

### "Caching is slow or not working"

**Symptoms**: CI takes as long as without cache.

**Causes**:
1. Cache was invalidated (Cargo.lock changed)
2. Cache size limit exceeded (10GB per repo)
3. Cache key collision

**Solutions**:
1. Check cache hit rate in CI logs
2. Clear old caches: Actions → Caches → Delete
3. Use more specific cache keys

### "Tests pass locally but fail in CI"

**Common causes**:
1. **Missing environment variables**
   - Check if test needs `TESTS_HF_TOKEN` or other vars
2. **Platform differences**
   - Test on the same OS as CI (use Docker)
3. **Timing issues**
   - Tests may be flaky due to race conditions
4. **Different Rust version**
   - CI uses stable, you might be on nightly
5. **Missing dependencies**
   - Check if system dependencies are needed

**Solutions**:
1. Run `cargo test --workspace --all-features` locally
2. Check environment variables in CI logs
3. Use conditional compilation for platform-specific code
4. Run tests with `--test-threads=1` to check for race conditions

### "CI is too slow"

**Diagnostics**:
1. Check job timing in GitHub Actions UI
2. Identify bottleneck jobs
3. Check cache hit rates

**Solutions**:
1. Enable more aggressive caching
2. Split large test suites
3. Use matrix sharding for tests
4. Move expensive tests to separate job
5. Consider cargo-nextest for faster test execution

### "Security audit fails"

**Symptoms**: `security-audit` job fails with vulnerability warnings.

**Actions**:
1. Read the security advisory: https://rustsec.org/advisories/
2. Update vulnerable dependency: `cargo update -p <crate>`
3. If no fix available:
   - Consider alternative crate
   - Temporarily add to allowlist (use caution)
   - Contact crate maintainer

**Allowlist** (if absolutely necessary):
```toml
# .cargo/audit.toml
[advisories]
ignore = [
    "RUSTSEC-YYYY-NNNN",  # Reason: <explanation>
]
```

---

## Summary

**Key Takeaways**:

1. **CI runs on every push and PR** to catch issues early
2. **Quick-check job fails fast** (2-3 min) for rapid feedback
3. **Tests run on 3 platforms** (Linux, macOS, Windows) in parallel
4. **Caching reduces build time** by ~5-10x after first run
5. **Coverage tracked on Codecov** for visibility
6. **Security audits** run automatically
7. **All CI checks must pass** before merging

**For Questions or Issues**:
- See detailed logs in GitHub Actions UI
- Run `make ci-full` to replicate CI locally
- Check this document for troubleshooting
- Ask in project discussions or issues

---

*Document Version*: 1.0  
*Last Updated*: 2025-01-05  
*Maintained by*: DevOps Team