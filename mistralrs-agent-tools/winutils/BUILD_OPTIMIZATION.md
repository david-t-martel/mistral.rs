# Build Performance Optimization Guide for WinUtils

## Executive Summary

This guide provides comprehensive build optimization strategies for the WinUtils workspace containing 93 workspace members (2 libraries + 91 utilities). The optimization focuses on reducing build times, improving incremental compilation, and optimizing CI/CD pipelines.

## Current Architecture

- **Workspace Members**: 93 (winpath + winutils-core libs + 91 utilities)
- **Target Platform**: x86_64-pc-windows-msvc
- **Build Profile**: release with LTO=true, codegen-units=1, opt-level=3
- **Build Directory**: T:/projects/coreutils/winutils/target

## Optimization Implementations

### 1. sccache - Compilation Caching (IMPLEMENTED)

**Purpose**: Cache compilation artifacts to speed up rebuilds

**Setup**:

```powershell
# Install sccache (if not already installed)
cargo install sccache --locked

# Or using cargo-binstall (faster)
cargo binstall sccache -y

# Configure environment
$env:RUSTC_WRAPPER = "sccache"
$env:SCCACHE_DIR = "T:\projects\coreutils\sccache-cache"
$env:SCCACHE_CACHE_SIZE = "10G"
```

**Configuration in `.cargo/config.toml`**:

```toml
[build]
rustc-wrapper = "sccache"
target-dir = "T:/projects/coreutils/winutils/target"
```

**Expected Performance Gains**:

- Clean builds after cache warm-up: 40-60% faster
- Incremental builds: 20-30% faster
- CI builds with cache restore: 50-70% faster

**Monitoring**:

```powershell
# Check cache statistics
sccache --show-stats

# Monitor during build
.\monitor-sccache.ps1
```

### 2. cargo-nextest - Parallel Test Execution

**Purpose**: Run tests in parallel with better output formatting

**Setup**:

```powershell
# Install nextest
cargo binstall cargo-nextest -y

# Or via cargo install
cargo install cargo-nextest --locked
```

**Usage**:

```powershell
# Run tests with nextest
cargo nextest run --release

# With specific configuration
cargo nextest run --release --retries 2 --failure-output immediate
```

**Expected Performance Gains**:

- Test execution: 2-3x faster on multi-core systems
- Better test isolation and failure reporting
- Automatic retry for flaky tests

### 3. Profile Optimization

**Current Release Profile**:

```toml
[profile.release]
lto = true           # Link-Time Optimization
codegen-units = 1    # Single codegen unit for max optimization
opt-level = 3        # Maximum optimization
```

**Recommended Development Profile**:

```toml
[profile.dev-fast]
inherits = "dev"
incremental = true
codegen-units = 256  # Maximum parallelism
opt-level = 0

[profile.release-fast]
inherits = "release"
lto = "thin"         # Faster than "fat" LTO
codegen-units = 16   # Balance between speed and optimization
```

**Trade-offs**:

- `codegen-units=1`: Best runtime performance, slowest compilation
- `codegen-units=16`: Good balance for release builds
- `codegen-units=256`: Fastest compilation for development

### 4. Incremental Compilation Optimization

**Enable for development**:

```toml
[build]
incremental = true

[profile.dev]
incremental = true
split-debuginfo = "packed"  # Faster on Windows
```

**Reduce recompilation triggers**:

- Use workspace dependencies to share common dependencies
- Avoid `path = "../"` dependencies when possible
- Use feature flags to conditionally compile heavy dependencies

### 5. Dependency Optimization

**Analyze heavy dependencies**:

```powershell
# Install cargo-bloat
cargo install cargo-bloat

# Check binary sizes
cargo bloat --release --crates

# Analyze build times
cargo build --release --timings
```

**Optimization strategies**:

- Replace heavy dependencies with lighter alternatives
- Use feature flags to exclude unnecessary functionality
- Consider vendoring and optimizing critical dependencies

### 6. CI/CD Pipeline Optimization

**GitHub Actions Configuration**:

```yaml
name: Optimized Build

on: [push, pull_request]

env:
  CARGO_TERM_COLOR: always
  SCCACHE_GHA_ENABLED: "true"
  RUSTC_WRAPPER: "sccache"

jobs:
  build:
    runs-on: windows-latest
    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Setup sccache
      uses: mozilla-actions/sccache-action@v0.0.3

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Build
      run: cargo build --release

    - name: Test with nextest
      run: |
        cargo install cargo-nextest --locked
        cargo nextest run --release
```

**Expected CI improvements**:

- First run: Baseline time
- Subsequent runs: 50-70% faster with cache hits
- Parallel jobs: Use matrix builds for independent components

### 7. Build Script Optimization

**Parallel workspace builds**:

```powershell
# Build specific packages in parallel
cargo build -p winpath -p winutils-core --release

# Build all with job control
cargo build --release -j 8
```

### 8. Monitoring and Benchmarking

**Use the provided benchmark script**:

```powershell
# Full benchmark suite
.\benchmark-build.ps1 -All

# Specific benchmarks
.\benchmark-build.ps1 -CleanBuild
.\benchmark-build.ps1 -IncrementalBuild
.\benchmark-build.ps1 -TestExecution
```

**Key metrics to track**:

- Clean build time
- Incremental build time
- Test execution time
- Cache hit rates
- Binary sizes

## Performance Targets

Based on workspace size (93 members):

| Build Type  | Current (est.) | Target    | Optimization       |
| ----------- | -------------- | --------- | ------------------ |
| Clean Build | 8-12 min       | 4-6 min   | sccache + parallel |
| Incremental | 30-60 sec      | 10-20 sec | Better profiles    |
| Tests       | 2-3 min        | 45-60 sec | nextest            |
| CI Build    | 15-20 min      | 7-10 min  | Cache + sccache    |

## Quick Start Checklist

1. ✅ **Install build tools**:

   ```powershell
   cargo install sccache cargo-nextest cargo-bloat --locked
   ```

1. ✅ **Configure environment**:

   ```powershell
   .\setup-sccache.ps1
   ```

1. ✅ **Run baseline benchmark**:

   ```powershell
   .\benchmark-build.ps1 -All
   ```

1. ✅ **Update cargo config**:

   - Add `rustc-wrapper = "sccache"`
   - Configure build profiles

1. ✅ **Test optimizations**:

   ```powershell
   # Clean build with sccache
   cargo clean
   cargo build --release

   # Check cache hits
   sccache --show-stats
   ```

## Advanced Optimizations

### Distributed Compilation (Future)

- Consider `sccache` with S3/Redis backend for team builds
- Use `distcc` or similar for distributed compilation

### Module Boundaries

- Split large crates into smaller, focused modules
- Use workspace inheritance for common dependencies
- Consider using `cargo-hakari` for shared dependencies

### Binary Size Optimization

```toml
[profile.release]
strip = true
panic = "abort"
lto = "fat"
opt-level = "z"  # Optimize for size
```

## Troubleshooting

### sccache Issues

```powershell
# Reset cache
sccache --stop-server
Remove-Item -Recurse $env:SCCACHE_DIR\*
sccache --start-server

# Debug mode
$env:SCCACHE_LOG = "debug"
sccache --show-stats
```

### Incremental Compilation Issues

```powershell
# Clean incremental cache
cargo clean
Remove-Item -Recurse target\debug\incremental
Remove-Item -Recurse target\release\incremental
```

## Measurement and Continuous Improvement

1. **Regular Benchmarking**: Run benchmarks weekly
1. **Track Metrics**: Store benchmark results in git
1. **Profile Analysis**: Use `cargo build --timings` regularly
1. **Dependency Audits**: Review heavy dependencies quarterly

## Conclusion

These optimizations should provide significant build performance improvements:

- **Immediate gains**: 30-50% with sccache
- **Test improvements**: 2-3x with nextest
- **CI optimization**: 50-70% faster with caching
- **Long-term**: Further gains through profile tuning and dependency optimization

Regular monitoring and incremental improvements will ensure sustained performance as the project grows.
