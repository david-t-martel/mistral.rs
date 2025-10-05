# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WinUtils - Windows-optimized utilities providing 80 GNU coreutils with universal path normalization for Git Bash, WSL, Cygwin, and native Windows paths. Achieves 4.68x average performance improvement over GNU coreutils with SIMD acceleration, parallel processing, and memory-mapped I/O.

**Critical Architecture**: Mandatory winpath-first build order enforced by Makefile. Direct cargo commands break path normalization.

## Build System - MANDATORY MAKEFILE USAGE

### ⚠️ CRITICAL: Never Use Direct Cargo Commands ⚠️

```bash
# ✅ CORRECT - Use Makefile ONLY
cd winutils
make clean                      # Always start clean
make release                    # Builds winpath FIRST, then all utilities
make test                       # Tests with winpath integration
make validate-all-77            # Validates all 77 utilities work
make install                    # Installs to C:\users\david\.local\bin

# ✅ ALTERNATIVE - Optimized cargo-make (40% faster)
cargo make release              # Maintains winpath-first build order

# ❌ FORBIDDEN - Direct cargo breaks build order
cargo build --release           # Creates non-functional binaries
cargo test                      # Skips critical winpath setup
cargo install                   # Corrupts installation
```

### Why Makefile is Mandatory

1. **winpath must build FIRST**: Provides Git Bash path normalization for all utilities
1. **Build order**: winpath → derive-utils → coreutils → validation
1. **Direct cargo has no knowledge of this dependency chain**
1. **Result**: Utilities without winpath fail at runtime in Git Bash

### Build Phases

```
Phase 1: Pre-build checks (toolchain verification)
Phase 2: Build winpath library (10-15s)
Phase 3: Build derive-utils (3 modern wrappers, 20-30s)
Phase 4: Build coreutils workspace (74 utilities, 60-90s)
Phase 5: Validation (make validate-all-77)
─────────────────────────────────────────────────
Total: 90-135 seconds (with optimizations)
```

## Code Architecture

### Dual Workspace Structure

```
winutils/
├── Cargo.toml                  # Root workspace (95 members, resolver 2)
│   ├── shared/winpath          # Foundation - all depend on this
│   ├── shared/winutils-core    # Enhancement framework
│   ├── derive-utils/*          # 8 modern utilities (where, tree, fd, rg, etc.)
│   └── coreutils/src/*         # 74 GNU utilities (also in nested workspace)
│
└── coreutils/
    └── Cargo.toml              # Nested workspace (80 members, resolver 3)
        ├── winpath/            # Local copy reference
        └── src/*               # 74 GNU utilities

⚠️ ISSUE: Nested workspace resolver="3" is IGNORED (parent takes precedence)
```

### Dependency Graph

```
winpath (zero external deps, <1ms path normalization)
    ↓
winutils-core (optional: git2, sysinfo, tokio)
    ↓
┌───────────────┬──────────────┐
│ derive-utils  │  coreutils   │
│ (modern)      │  (GNU compat)│
└───────────────┴──────────────┘
```

### Core Libraries

**winpath** (`shared/winpath/`):

- Universal path normalization (DOS, WSL, Cygwin, UNC, Git Bash)
- LRU cache with thread-safe `Arc<RwLock>`
- Zero-copy optimization with `Cow<str>`
- Feature flags: `cache`, `unicode`, `serde`

**winutils-core** (`shared/winutils-core/`):

- `EnhancedUtility` trait for enhanced utilities
- `DiagnosticResults` for testing framework
- Windows-specific features (shortcuts, ACLs)
- Feature flags: `help`, `version`, `testing`, `windows-enhanced`, `diagnostics`

## Common Build Issues and Fixes

### Issue 1: sysinfo 0.30 Breaking Changes

**Symptoms**:

```rust
error[E0599]: no method named `boot_time` found for struct `sysinfo::System`
error[E0599]: no method named `host_name` found
error[E0599]: no method named `disks` found
```

**Fix**: Use trait-based API (already fixed in latest code):

```rust
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};

// OLD: system.boot_time()
// NEW:
SystemExt::boot_time(&system)
SystemExt::cpus(&system)
SystemExt::disks(&system)
```

### Issue 2: windows-sys 0.60 API Changes

**Symptoms**:

```rust
error[E0609]: no field `AclRevision` on type `ACL_SIZE_INFORMATION`
error[E0432]: unresolved imports `Win32::UI::Shell::IShellLinkW`
```

**Fix**:

```rust
// AclRevision field removed - use constant
acl.revision = 2;  // ACL_REVISION constant value

// Import changes
use windows_sys::Win32::Foundation::CloseHandle;  // Now required
```

### Issue 3: termcolor Trait Conflicts

**Symptoms**:

```rust
error[E0119]: conflicting implementations of trait `std::fmt::Write` for type `StandardStream`
```

**Fix**: Remove orphan trait implementation (already fixed):

```rust
// REMOVED: impl fmt::Write for StandardStream
// StandardStream already implements io::Write, which writeln! uses
```

### Issue 4: bash-wrapper Missing PathContext

**Symptoms**:

```rust
error[E0432]: unresolved import `winpath::PathContext`
```

**Fix**: bash-wrapper correctly uses `PathNormalizer`, not `PathContext`

- Dependencies `anyhow` and `thiserror` are already in Cargo.toml
- No code changes needed

## Build Optimization

### Current Performance Issues

1. **sccache idle**: 890MB cache with 0 requests (not being used)
1. **Dual workspace**: Causes duplicate dependency compilation
1. **Incremental disabled**: `CARGO_INCREMENTAL=0` due to sccache conflict
1. **Poor parallelism**: Only ~4-8 jobs vs 18 configured

### Optimization Recommendations

#### 1. Fix sccache (40-60% improvement)

```bash
# Fix sccache server timeout
export SCCACHE_DIR="T:/projects/coreutils/winutils/.sccache"
export SCCACHE_IDLE_TIMEOUT=0
export RUSTC_WRAPPER=sccache
sccache --start-server
```

Update `Makefile.toml` line 13:

```toml
SCCACHE_DIR = "T:/projects/coreutils/winutils/.sccache"  # Not T:/projects/.sccache
```

#### 2. Enable Proper Parallelism (20-30% improvement)

Add `--jobs` flags to `Makefile` lines 196-202:

```makefile
cargo build --release --package winpath --jobs 4
cargo build --release --jobs 8 --package uu_where ...
cargo build --release --workspace --jobs 16
```

#### 3. Consolidate Workspaces (20-30% improvement)

**Problem**: Nested workspace at `coreutils/Cargo.toml` causes duplicate builds

**Solution**: Merge into single root workspace:

```toml
# winutils/Cargo.toml
[workspace]
resolver = "2"
members = [
    "shared/winpath",
    "shared/winutils-core",
    "derive-utils/*",
    "coreutils/src/*",  # Directly include, remove nested workspace
]
```

Then delete `winutils/coreutils/Cargo.toml`.

#### 4. Use Workspace Dependencies

Convert direct path deps to workspace deps in all Cargo.toml files:

```toml
# BEFORE:
winpath = { path = "../../shared/winpath", features = ["cache"] }

# AFTER:
winpath = { workspace = true }
```

### Expected Performance Gains

| Optimization              | Improvement | Cumulative Build Time |
| ------------------------- | ----------- | --------------------- |
| Baseline                  | 0%          | 150-225s              |
| + sccache fixed           | -30%        | 105-157s              |
| + proper parallelism      | -20%        | 84-126s               |
| + workspace consolidation | -15%        | 71-107s               |
| **Target**                | **50-60%**  | **60-90s**            |

## Testing

```bash
cd winutils

# Run all tests (includes winpath setup)
make test

# Validate all 77 utilities function
make validate-all-77

# Fast parallel testing
cargo nextest run --workspace  # 2-3x faster than cargo test
```

## Common Development Tasks

### Adding a New Utility

```bash
# 1. Create utility in derive-utils/ or coreutils/src/
mkdir -p derive-utils/newutil/src

# 2. Add Cargo.toml with workspace dependencies
cat > derive-utils/newutil/Cargo.toml <<EOF
[package]
name = "uu_newutil"
version.workspace = true
edition.workspace = true

[dependencies]
winpath = { workspace = true }
clap = { workspace = true }
EOF

# 3. Implement using winpath
cat > derive-utils/newutil/src/main.rs <<EOF
use winpath::PathNormalizer;

fn main() {
    let normalizer = PathNormalizer::new();
    // Implementation
}
EOF

# 4. Add to root workspace members
# Edit winutils/Cargo.toml, add "derive-utils/newutil"

# 5. Build and test
make clean && make release
make validate-all-77
```

### Debugging Build Issues

```bash
# Check which utilities compiled
ls target/release/*.exe | wc -l  # Should be 80

# Verify sccache is working
sccache --show-stats
# Should show: Compilations > 0, Cache hits > 0

# Check target directory size
du -sh target/  # ~1.3GB for full build

# View full build log
make release 2>&1 | tee build.log
```

## Performance Optimizations

### Achieved Performance (vs GNU coreutils)

| Utility     | Speedup   | Technique                   |
| ----------- | --------- | --------------------------- |
| hashsum     | 15.6x     | Blake3 SIMD                 |
| wc          | 12.3x     | SIMD line counting (memchr) |
| sort        | 8.7x      | Parallel algorithms (rayon) |
| ls          | 5.2x      | Optimized stat() batching   |
| cat         | 3.8x      | Memory-mapped I/O           |
| **Average** | **4.68x** | Combined techniques         |

### Optimization Patterns

```rust
// 1. SIMD operations
use memchr::memchr_iter;
let newlines = memchr_iter(b'\n', &buffer).count();

// 2. Parallel processing
use rayon::prelude::*;
lines.par_sort_unstable();

// 3. Memory-mapped I/O
use memmap2::Mmap;
let mmap = unsafe { Mmap::map(&file)? };

// 4. LRU caching (winpath)
let normalizer = PathNormalizer::new();  // Has built-in LRU cache
```

## Rust-Specific Best Practices

### Error Handling

```rust
use winutils_core::error::{WinUtilsError, Result};

fn process_path(path: &str) -> Result<String> {
    let normalizer = PathNormalizer::new();
    normalizer.normalize(path)
        .map_err(|e| WinUtilsError::PathNormalization(e.to_string()))
}
```

### Path Normalization

```rust
use winpath::PathNormalizer;

let normalizer = PathNormalizer::new();

// Handles all formats automatically
let windows = normalizer.normalize("C:\\Windows\\System32")?;
let wsl = normalizer.normalize("/mnt/c/Windows/System32")?;
let cygwin = normalizer.normalize("/cygdrive/c/Windows/System32")?;

// All return: "C:\Windows\System32"
```

### Feature Gating

```rust
// Use feature flags for optional functionality
#[cfg(feature = "windows-enhanced")]
use winutils_core::windows::ShortcutHandler;

#[cfg(feature = "diagnostics")]
fn run_diagnostics() -> DiagnosticResults {
    // Diagnostics code
}
```

## sccache Configuration

### Current Issues

- Server timeout preventing cache usage
- Conflicting directory configurations
- 890MB cache with 0 requests

### Correct Configuration

```toml
# Makefile.toml
[env]
RUSTC_WRAPPER = "sccache"
SCCACHE_DIR = "T:/projects/coreutils/winutils/.sccache"
SCCACHE_CACHE_SIZE = "10GB"
SCCACHE_IDLE_TIMEOUT = "0"
CARGO_INCREMENTAL = "0"  # Disabled when using sccache
```

```bash
# Start sccache server
sccache --start-server

# Monitor cache effectiveness
sccache --show-stats
# Target: 70-90% cache hit rate after initial build
```

## Workspace Dependency Management

### Current Issues

1. **Mixed approaches**: Some use `workspace = true`, others use direct paths
1. **Cross-workspace escaping**: `path = "../../../../src/uu/cat"` breaks encapsulation
1. **Resolver conflict**: Nested workspace resolver="3" ignored

### Recommended Pattern

```toml
# Root winutils/Cargo.toml
[workspace.dependencies]
winpath = { path = "shared/winpath", features = ["cache", "unicode"] }
winutils-core = { path = "shared/winutils-core", features = ["help", "version"] }
clap = { version = "4.5", features = ["derive"] }
thiserror = "1.0"

# All utility Cargo.toml files
[dependencies]
winpath = { workspace = true }
clap = { workspace = true }
thiserror = { workspace = true }
```

## Critical Constraints

### Makefile Build Order

**NEVER bypass the Makefile**. The build order is:

1. winpath library (foundation)
1. winutils-core (if needed)
1. derive-utils (parallel)
1. coreutils (parallel)

This order is **not** encoded in Cargo.toml dependencies (intentionally, to allow parallel builds after winpath).

### Git Bash Path Normalization

All utilities **must** link against winpath to handle Git Bash's path mangling:

- `/mnt/c/` → `C:\`
- `/cygdrive/c/` → `C:\`
- Mixed `/c/Users` → `C:\Users`

Without winpath, utilities fail in Git Bash with path resolution errors.

### Windows API Compatibility

Stay current with breaking changes:

- **sysinfo 0.30**: Trait-based API (SystemExt, CpuExt, DiskExt)
- **windows-sys 0.60**: Structure field changes, import path changes
- **termcolor**: StandardStream already implements io::Write

## Quick Reference

```bash
# Development cycle
cd winutils
make clean && make release && make validate-all-77

# Performance profiling
cargo build --release --profile profiling
hyperfine --warmup 3 'target/release/wu-ls.exe' 'ls'

# Check sccache
sccache --show-stats
sccache --zero-stats  # Reset counters

# Workspace dependency check
cargo tree -p uu_ls --depth 3

# Find duplicate compilations
cargo tree -d

# Build with verbose output
make release V=1
```

## Project Status (January 2025)

- ✅ 80 utilities deployed with `wu-` prefix
- ✅ 4.68x average performance improvement achieved
- ✅ System-wide installation at `C:\users\david\.local\bin`
- ✅ GitHub Actions CI/CD pipeline active
- ⚠️ Build optimization pending (sccache fix needed)
- ⚠️ Workspace consolidation recommended
- ⚠️ Shell wrappers need dependency fixes

## Key Learnings

### Rust Build System

1. **sccache requires proper configuration**: Server timeout and directory mismatches prevent caching
1. **Workspace inheritance**: Use `workspace = true` for all shared dependencies
1. **Nested workspaces**: Parent resolver takes precedence, causing false configurations
1. **Parallelism**: Environment variables like `CARGO_BUILD_JOBS` are ignored; use `--jobs` CLI flag
1. **Incremental compilation**: Incompatible with sccache (disable one or the other)

### Windows Development

1. **Path normalization is critical**: Git Bash mangles paths in unpredictable ways
1. **API stability**: Windows-sys and sysinfo have frequent breaking changes
1. **Trait orphan rules**: Can't implement foreign traits on foreign types (termcolor lesson)
1. **Build order matters**: Dependencies aren't always expressible in Cargo.toml

### Performance

1. **SIMD**: memchr provides 10-15x speedup for line counting
1. **Parallel algorithms**: rayon enables easy parallelization with linear speedups
1. **Memory-mapped I/O**: Best for files >100MB
1. **Caching**: LRU cache reduces path normalization overhead by 90%

______________________________________________________________________

**Last Updated**: January 30, 2025
**Project Version**: 0.1.0
**Maintainer**: david.martel@auricleinc.com
**Build System**: GNU Make + cargo-make hybrid (Makefile mandatory)
