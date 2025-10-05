# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WinUtils is a Windows-optimized implementation of GNU coreutils with 80 utilities (77 core + 3 modern replacements), achieving 4.68x average performance improvement through Rust optimization and universal path normalization.

**Key Stats**: 80 deployed utilities | 4.68x faster | C:\\users\\david.local\\bin\\wu-\*.exe

## Build System

### Build Order Requirement

The `shared/winpath` library **MUST** be built first - it provides Git Bash path normalization that all utilities depend on. Use the Makefile to enforce correct build order.

### Build Commands

```bash
# Standard workflow
make clean                    # Clean all artifacts
make release                  # Build with proper order (winpath → derive-utils → coreutils)
make test                     # Run tests with winpath integration
make validate-all-77          # Verify all 77 utilities work
make install                  # Install to C:\users\david\.local\bin

# Faster alternative (40% faster with parallelization)
cargo make release            # Optimized build
cargo make test-all           # Comprehensive tests
cargo make validate           # Validate all 80 utilities

# Individual operations
make util-ls                  # Build specific utility
make test-util-cat            # Test specific utility
make check                    # Format, clippy, audit
```

### Configuration

**Binary output location** (`.cargo/config.toml` line 13):

```toml
[build]
target-dir = "target"         # Change to "T:/projects/coreutils/targets" for shared location
```

**Binary naming**:

- Compiled as: `ls.exe`, `cat.exe` (in target/release/)
- Installed as: `wu-ls.exe`, `wu-cat.exe` (to avoid conflicts)
- To change prefix: Edit Makefile line 281

## Architecture

### Project Structure

```
winutils/
├── shared/winpath/          # Path normalization (BUILD FIRST)
│   ├── lib.rs              # Main API with LRU caching
│   ├── detection.rs        # Format detection (DOS/WSL/Cygwin/UNC)
│   └── normalization.rs    # Path conversion algorithms
├── derive-utils/            # Windows-specific utilities (where, which, tree)
├── coreutils/src/*/        # 74 GNU utilities
├── Makefile                # Primary build orchestrator (830 lines)
├── Makefile.toml           # cargo-make optimization (663 lines)
└── .cargo/config.toml      # Cargo configuration
```

### Path Normalization

**Supported formats**:

- DOS: `C:\Windows\System32`
- WSL: `/mnt/c/Windows/System32`
- Cygwin: `/cygdrive/c/Windows/System32`
- UNC: `\\?\C:\Windows\System32`
- Git Bash: Automatic detection/normalization

**Usage pattern**:

```rust
use winpath::normalize_path;
let path = normalize_path("/mnt/c/users/david")?;  // → "C:\users\david"
```

### Binary Naming

**Package naming**:

- Coreutils: `uu_{util}` package → `{util}.exe` binary → `wu-{util}.exe` installed
- Derive-utils: `{util}` package → `{util}.exe` binary → `{util}.exe` installed

## Testing

```bash
# Run tests
make test                     # Full test suite
make test-unit               # Unit tests only
make test-git-bash-paths    # Path normalization tests

# Validation
make validate-all-77          # Verify all 77 utilities
.\scripts\validate.ps1        # PowerShell validation
.\scripts\test-gnu-compat.ps1 # GNU compatibility tests
```

**Test locations**:

- Unit tests: `src/*/tests/`
- Integration tests: `tests/`
- Results: `test-results/`

## Development Workflow

### Adding a New Utility

1. Create structure: `mkdir -p coreutils/src/mynewutil/src`
1. Create `Cargo.toml`:

```toml
[package]
name = "uu_mynewutil"
version.workspace = true
edition.workspace = true

[[bin]]
name = "mynewutil"
path = "src/main.rs"

[dependencies]
winpath = { workspace = true }
clap = { workspace = true }
```

3. Create `src/main.rs` with winpath integration
1. Add to workspace: Edit `winutils/Cargo.toml` members
1. Add to Makefile: Line 75-85 COREUTILS list
1. Build: `make clean && make release`
1. Validate: `make validate-all-77`

### Modifying Utilities

1. Edit utility in `coreutils/src/<utility>/`
1. Ensure winpath dependency exists
1. Build: `make clean && make release`
1. Test: `make test`
1. Validate: `make validate-all-77`

## Performance

### Benchmarks (January 2025)

| Utility     | Improvement | Technique              |
| ----------- | ----------- | ---------------------- |
| hashsum     | 15.6x       | Blake3 SIMD            |
| wc          | 12.3x       | SIMD line counting     |
| sort        | 8.7x        | Parallel sorting       |
| ls          | 5.2x        | Optimized stat()       |
| cat         | 3.8x        | Memory-mapped I/O      |
| **Average** | **4.68x**   | Multiple optimizations |

### Build Performance

- Full rebuild: 2-3 minutes (80 utilities)
- Incremental: \<30 seconds (with sccache)
- Parallel build: 40-50% faster with `cargo make`
- Binary size: ~1.16 MB average per utility

## Troubleshooting

| Issue                     | Solution                                                |
| ------------------------- | ------------------------------------------------------- |
| Build failure             | Always `make clean` before `make release`               |
| Path normalization errors | Verify winpath.exe exists: `winpath.exe "test/path"`    |
| Git Bash compatibility    | Run `make validate-all-77` after build                  |
| Tests failing             | Use `make test`, not `cargo test` (needs winpath setup) |
| Binary not found          | Check `target/x86_64-pc-windows-msvc/release/`          |

**Diagnostics**:

```bash
make doctor                   # Environment check
make count                    # Verify binary count
make verify-winpath-integration  # Check winpath
```

## Quick Reference

### Essential Commands

```bash
make clean release test validate-all-77 install
cargo make release            # 40% faster alternative
make help                     # Show all available targets
```

### Useful Targets

```bash
make stats                    # Build statistics
make bench                    # Run benchmarks
make list                     # List all utilities
make dep-graph               # Show dependencies
make fmt                      # Format code
make clippy                   # Linting
make audit                    # Security audit
```

### Environment Setup

**Required tools**:

- Rust toolchain (x86_64-pc-windows-msvc)
- GNU Make or cargo-make
- PowerShell (for scripts)

**Recommended setup**:

```toml
# .cargo/config.toml optimizations already configured:
# - sccache for build caching (40-90% faster rebuilds)
# - Native CPU optimizations
# - 8MB stack size
# - Static linking (no DLL dependencies)
```

## Important Notes

1. **winpath is critical** - Built first, used by all utilities for Git Bash compatibility
1. **80 utilities deployed** - 77 core GNU + 3 modern replacements (fd, rg wrappers)
1. **Installation prefix** - Uses `wu-` to avoid system conflicts
1. **Target directory** - Local `target/`, configurable in `.cargo/config.toml`
1. **Performance verified** - 4.68x average improvement over GNU coreutils
1. **CI/CD active** - GitHub Actions pipeline operational

The Makefile enforces the critical build order (winpath → derive-utils → coreutils) required for proper Windows path handling across all environments (DOS, WSL, Cygwin, Git Bash).
