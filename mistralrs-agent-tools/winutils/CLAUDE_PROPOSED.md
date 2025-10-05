# CLAUDE.md - Winutils Development Guide

This guide provides essential information for developing the Windows-optimized GNU coreutils implementation.

## Quick Start

```bash
# Complete build and validation (5 commands)
make clean              # Start fresh
make release            # Build all (winpath first)
make test               # Run tests
make install            # Install to ~/.local/bin
make validate-all-77    # Verify all work
```

## Critical Build Requirement

**The winpath library MUST be built before utilities.** This provides Git Bash path normalization that all utilities depend on.

### Allowed Build Commands

```bash
make release            # Standard build
cargo make release      # Optimized build (40% faster)
make test               # Run tests with winpath
cargo make test-all     # Comprehensive tests
```

### Forbidden Commands

```bash
cargo build             # Breaks build order
cargo test              # Skips winpath setup
cargo install           # Corrupts installation
```

## Project Structure

```
winutils/
├── shared/winpath/     # Path normalization (builds FIRST)
├── derive-utils/       # Modern utilities (fd, rg wrappers)
├── coreutils/          # 74 GNU coreutils
├── Makefile            # Primary build system (800+ lines)
└── Makefile.toml       # cargo-make configuration
```

**Build Order**: winpath → derive-utils → coreutils → validation

## Development Workflow

### Modifying a Utility

```bash
# 1. Edit the utility
vim coreutils/src/ls/src/ls.rs

# 2. Rebuild (always clean first for safety)
make clean && make release

# 3. Test the specific utility
./target/release/wu-ls -la

# 4. Run tests
make test

# 5. Install if working
make install
```

### Testing Individual Utilities

```bash
# Direct execution
./target/release/wu-ls --help

# With path normalization test
echo "/mnt/c/Windows" | ./target/release/winpath
./target/release/wu-ls /mnt/c/Windows

# Benchmark specific utility
hyperfine "./target/release/wu-wc large_file.txt"
```

### Adding a New Utility

1. Create in `coreutils/src/<name>/`
1. Add winpath dependency in Cargo.toml
1. Register in workspace Cargo.toml
1. Build via `make release` (never cargo directly)
1. Add tests in `coreutils/tests/`

## Common Issues & Solutions

### Build Failures

| Error                   | Solution                                                                 |
| ----------------------- | ------------------------------------------------------------------------ |
| "winpath not found"     | Run `make clean && make release`                                         |
| "undefined reference"   | Cargo.toml missing winpath dependency                                    |
| "Git Bash paths broken" | Ensure winpath.exe exists in target/release                              |
| Link errors             | Check `[dependencies]` has `winpath = { path = "../../shared/winpath" }` |

### Runtime Issues

| Problem                | Fix                                                |
| ---------------------- | -------------------------------------------------- |
| Path not normalized    | Verify winpath.exe is in same directory as utility |
| Slow performance       | Build with `make release PROFILE=release-fast`     |
| Binary too large       | Use `make release PROFILE=release-small`           |
| Git Bash compatibility | Test with: `./wu-ls /c/Windows` (should work)      |

## Environment Setup

### Required Tools

- Rust 1.70+ with MSVC toolchain
- GNU Make 4.0+
- Git Bash or MSYS2 (for testing)
- Optional: cargo-make (`cargo install cargo-make`)

### Environment Variables

```bash
# Faster builds
export CARGO_TARGET_DIR="C:/tmp/rust-target"
export RUSTFLAGS="-C target-cpu=native"

# Debug issues
export RUST_BACKTRACE=full
export RUST_LOG=debug
```

### VS Code Configuration

```json
// .vscode/tasks.json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Build Winutils",
      "type": "shell",
      "command": "make release",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "Test Winutils",
      "type": "shell",
      "command": "make test",
      "group": "test"
    }
  ]
}
```

## Performance Optimization

### Current Benchmarks (vs GNU coreutils)

| Utility | Speedup | Technique          |
| ------- | ------- | ------------------ |
| hashsum | 15.6x   | Blake3 SIMD        |
| wc      | 12.3x   | SIMD line counting |
| sort    | 8.7x    | Parallel sorting   |
| ls      | 5.2x    | Stat batching      |
| cat     | 3.8x    | Memory-mapped I/O  |

### Optimization Techniques

```rust
// Use winpath for all paths
use winpath::normalize_path;
let path = normalize_path(input)?;

// SIMD operations
use memchr::memchr_iter;

// Parallel processing
use rayon::prelude::*;

// Memory-mapped files
use memmap2::Mmap;
```

### Build Profiles

```bash
make release PROFILE=release-fast   # Max speed, panic=abort
make release PROFILE=release-small  # Min size
make release                        # Balanced (default)
```

## Project Status

- **Utilities**: 80 deployed (77 GNU core + 3 modern)
- **Installation**: C:\\users\\david.local\\bin\\wu-\*.exe
- **Performance**: 4.68x average improvement
- **Compatibility**: Works in DOS, WSL, Cygwin, Git Bash
- **Binary Size**: ~1.16 MB per utility

## Key Files

| File                      | Purpose                              |
| ------------------------- | ------------------------------------ |
| Makefile                  | Primary build orchestrator           |
| Makefile.toml             | cargo-make configuration             |
| shared/winpath/src/lib.rs | Path normalization API               |
| Cargo.toml                | Workspace configuration (89 members) |
| scripts/validate.ps1      | PowerShell validation                |

## Path Normalization Support

Winpath handles these formats automatically:

- DOS: `C:\Windows\System32`
- WSL: `/mnt/c/Windows/System32`
- Cygwin: `/cygdrive/c/Windows/System32`
- UNC: `\\?\C:\Windows\System32`
- Git Bash: `/c/Windows` → `C:\Windows`

## Tips for Faster Development

1. **Incremental builds**: After initial `make release`, subsequent builds are faster
1. **Test single utility**: `cargo test -p uu_ls` (only after full make build)
1. **Skip validation**: `make release && make install` (skip test for quick iteration)
1. **Use ramdisk**: Set `CARGO_TARGET_DIR` to ramdisk for 2x faster builds
1. **Parallel make**: `make -j8 release` for parallel compilation
1. **sccache**: Install and use for caching Rust compilations

## Getting Help

- Check build output carefully - winpath errors appear early
- Validate with `make validate-all-77` to ensure utilities work
- Use `./target/release/wu-<utility> --help` for utility-specific help
- PowerShell validation: `.\scripts\validate.ps1`

______________________________________________________________________

*80 utilities deployed | 4.68x performance | Universal path support*
