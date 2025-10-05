# Windows Coreutils - Quick Context Reference

## Project Location

**Main Directory:** `T:\projects\coreutils\winutils\`

## Essential Commands

```bash
# Build everything
make release

# Test binaries
make test

# Install with wu- prefix
make install

# Clean build artifacts
make clean

# Direct Rust build
cargo build --release --workspace
```

## Binary Locations

- **Release binaries:** `T:\projects\coreutils\winutils\target\release\`
- **Some utilities:** `T:\projects\coreutils\winutils\target\release\deps\`

## Key Files to Edit

1. **Main workspace:** `Cargo.toml` (root)
1. **Coreutils workspace:** `coreutils\Cargo.toml`
1. **Build system:** `Makefile`
1. **Path library:** `shared\winpath\src\lib.rs`

## Current Status

- ✅ 77 functional binaries (74 coreutils + 3 derive)
- ✅ Universal path handling
- ✅ Windows optimizations
- ✅ Complete documentation
- ⚠️ Some binaries in deps folder
- ⚠️ External utilities (rg, fd) excluded

## Performance Highlights

- **where.exe:** 70% faster than Windows native
- **cat:** Optimized for CRLF/BOM handling
- **cp:** Uses native CopyFileEx API

## Agent Responsibilities

- **rust-pro:** Rust code and compilation
- **debugger:** Binary testing and validation
- **devops-troubleshooter:** Build system issues
- **docs-architect:** Documentation updates

## Next Steps

1. Consolidate binaries from deps folder
1. Fix remaining clippy warnings
1. Add more test coverage
1. Create Windows installer
1. Add shell completions

## Build Flags

```bash
RUSTFLAGS="-C target-feature=+crt-static -C target-cpu=native -C link-arg=/STACK:8388608"
```

## Installation Prefix

All utilities installed with `wu-` prefix to avoid conflicts.

______________________________________________________________________

*Context saved: 2025-01-22*
*Version: 1.0.0*
