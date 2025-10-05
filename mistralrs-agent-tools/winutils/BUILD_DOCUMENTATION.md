# Windows Coreutils Build Documentation

## Project Overview

Windows-optimized implementation of GNU coreutils with native Windows API integration and universal path handling support.

**Version**: 0.1.0
**Author**: David Martel <david.martel@auricleinc.com>
**Target Platform**: Windows x64 (x86_64-pc-windows-msvc)
**Language**: Rust 1.89.0+

## Repository Structure

```
T:\projects\coreutils\winutils\
├── Cargo.toml                 # Main workspace configuration
├── Makefile                    # Primary build system (GNU Make)
├── build.ps1                   # Windows PowerShell build script
├── .cargo/
│   └── config.toml            # Cargo configuration with optimization flags
├── shared/
│   └── winpath/               # Universal Windows path normalization library
├── derive-utils/              # Windows-specific utilities
│   ├── where/                 # Enhanced Windows where command
│   ├── which/                 # Cross-platform which implementation
│   └── tree/                  # Directory tree visualization
├── coreutils/                 # GNU coreutils implementations
│   ├── Cargo.toml            # Coreutils workspace configuration
│   ├── .cargo/
│   │   └── config.toml       # Coreutils-specific build config
│   └── src/                  # Individual utility implementations
│       ├── cat/              # Enhanced with CRLF/BOM handling
│       ├── cp/               # Windows CopyFileEx integration
│       ├── ls/               # Windows attributes support
│       └── [74+ utilities]   # Complete GNU coreutils suite
├── scripts/                   # Build and deployment scripts
│   ├── validate.ps1          # Utility validation script
│   ├── deploy.ps1            # Deployment automation
│   └── test-gnu-compat.ps1   # GNU compatibility testing
└── target/
    └── release/              # Compiled binaries output directory
```

## Build System Overview

### 1. Primary Build Method: Makefile

The main Makefile provides comprehensive build targets for all components.

```bash
# Quick build commands
make                    # Default: build release binaries
make release           # Build optimized release binaries
make debug             # Build with debug symbols
make clean             # Clean all build artifacts
make install           # Install to C:\users\david\.local\bin
make test              # Run all tests
make validate          # Validate all utilities
```

### 2. Alternative: PowerShell Build Script

For Windows-native build automation:

```powershell
# PowerShell build commands
.\build.ps1                    # Default build
.\build.ps1 -BuildType debug   # Debug build
.\build.ps1 -Clean            # Clean and rebuild
.\build.ps1 -Install          # Build and install
.\build.ps1 -Test             # Build and test
```

### 3. Direct Cargo Commands

For granular control over the build process:

```bash
# Build everything
cargo build --release --workspace

# Build specific workspace
cd coreutils && cargo build --release --workspace

# Build specific utility
cargo build --release --package uu_cat
```

## Build Configuration

### Rust Optimization Flags

All builds use these optimization flags for maximum performance:

```toml
# .cargo/config.toml
[build]
target-dir = "T:/projects/coreutils/winutils/target"

[target.x86_64-pc-windows-msvc]
rustflags = [
    "-C", "target-cpu=native",      # Optimize for current CPU
    "-C", "link-arg=/STACK:8388608", # 8MB stack size
    "-C", "prefer-dynamic=no"        # Static linking
]

[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for optimization
opt-level = 3           # Maximum optimization
strip = true            # Strip debug symbols
```

### Environment Variables

Set these before building for optimal results:

```bash
export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat"
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
```

## Build Process

### Step 1: Prerequisites

Ensure you have the following installed:

- Rust 1.89.0 or newer (`rustup update stable`)
- Visual Studio Build Tools 2019+ (for MSVC toolchain)
- Git (for dependency management)
- GNU Make (optional, for Makefile)
- PowerShell 5.1+ (for PowerShell scripts)

### Step 2: Clone and Setup

```bash
git clone https://github.com/david-t-martel/winutils.git
cd winutils
```

### Step 3: Build All Components

#### Method A: Using Makefile (Recommended)

```bash
# Clean previous builds
make clean

# Build everything in release mode
make release

# Verify build
make validate
```

#### Method B: Using PowerShell

```powershell
# Run comprehensive build
.\build.ps1 -BuildType release -Clean

# Verify utilities
.\scripts\validate.ps1 -GenerateReport
```

#### Method C: Manual Cargo Build

```bash
# Build main workspace (derive-utils)
cargo build --release --workspace

# Build coreutils workspace
cd coreutils
cargo build --release --workspace

# Return to root
cd ..
```

### Step 4: Verify Build Output

All compiled binaries should be in `target/release/`:

```bash
# Count built utilities
ls target/release/*.exe | wc -l
# Expected: 77+ executables

# Test a utility
target/release/uu_echo.exe "Build successful!"
```

## Individual Component Builds

### Building Specific Utilities

```bash
# Build a single coreutil
cargo build --release --package uu_ls

# Build derive-utils only
cargo build --release --package where --package which --package tree

# Build with specific features
cargo build --release --package uu_cat --features "windows-optimized"
```

### Building the winpath Library

```bash
cd shared/winpath
cargo build --release
cargo test  # Run library tests
```

## Workspace Configuration

The project uses a dual-workspace structure:

### Main Workspace (winutils/)

- Contains derive-utils and shared libraries
- Configured in `winutils/Cargo.toml`
- Build output: `target/release/`

### Coreutils Workspace (winutils/coreutils/)

- Contains all GNU coreutils implementations
- Configured in `winutils/coreutils/Cargo.toml`
- Shares target directory with main workspace

## Dependencies

### Core Dependencies

- `clap` 4.5 - Command-line argument parsing
- `windows-sys` 0.52 - Windows API bindings
- `winapi-util` 0.1 - Windows utility functions

### Windows-Specific

- `windows` 0.60 - Windows Runtime APIs
- `dunce` 1.0 - Canonical paths on Windows
- `path-slash` 0.2 - Cross-platform path handling

### Performance

- `rayon` 1.8 - Parallel processing
- `dashmap` 5.5 - Concurrent hashmap
- `lru` 0.12 - LRU cache implementation

## Testing

### Unit Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test --package winpath

# Run with verbose output
cargo test --workspace -- --nocapture
```

### Integration Tests

```bash
# GNU compatibility tests
powershell .\scripts\test-gnu-compat.ps1

# Validation suite
powershell .\scripts\validate.ps1 -Verbose
```

### Performance Benchmarks

```bash
# Run benchmarks
cargo bench --workspace

# Compare with GNU coreutils
make compare
```

## Installation

### Local Installation

```bash
# Using Makefile
make install

# Using PowerShell
.\scripts\deploy.ps1 -Environment local

# Manual installation
copy target\release\*.exe C:\users\david\.local\bin\
```

### System-Wide Installation (Administrator)

```powershell
# Run as Administrator
.\scripts\deploy.ps1 -Environment system -UpdatePath
```

## Troubleshooting

### Common Build Issues

#### 1. Missing MSVC Toolchain

```
error: linker `link.exe` not found
```

**Solution**: Install Visual Studio Build Tools 2019+

#### 2. Target Directory Issues

```
error: could not create directory
```

**Solution**: Ensure write permissions to target directory

#### 3. Dependency Resolution

```
error: failed to select a version
```

**Solution**: Run `cargo update` to refresh dependencies

#### 4. Path Too Long (Windows)

```
error: filename too long
```

**Solution**: Enable long path support in Windows or use shorter paths

### Build Verification

```bash
# Check Rust installation
rustc --version
cargo --version

# Check target
rustup target list --installed

# Check build configuration
cargo tree --workspace

# Diagnostic build
make doctor
```

## Binary Distribution

### Creating Release Package

```bash
# Create distribution package
make package

# Or using PowerShell
.\scripts\deploy.ps1 -Environment portable -Version 1.0.0
```

### Binary Naming Convention

- Core utilities: `uu_<utility>.exe` (e.g., `uu_cat.exe`)
- Derive utilities: `<utility>.exe` (e.g., `where.exe`)
- Installation prefix: `wu-` to avoid conflicts

## Performance Optimizations

### Compile-Time Optimizations

- Link-time optimization (LTO) enabled
- Single codegen unit for maximum inlining
- CPU-native instruction set targeting
- Debug symbols stripped in release builds

### Runtime Optimizations

- 8MB stack size for complex operations
- Static linking to reduce DLL dependencies
- Memory-mapped I/O for large files
- Parallel processing with Rayon

## Windows-Specific Features

### Universal Path Support

- DOS paths: `C:\Windows\System32`
- Unix paths: `/mnt/c/Windows/System32`
- WSL paths: `/mnt/c/users/david`
- Cygwin paths: `/cygdrive/c/users/david`
- UNC paths: `\\?\C:\Windows\System32`

### Windows API Integration

- CopyFileEx for efficient file copying
- Junction point and symlink support
- Windows file attributes preservation
- NTFS alternate data streams

### Enhanced Utilities

- `cat`: CRLF/LF conversion, BOM handling
- `cp`: Progress reporting, junction support
- `ls`: Windows attributes display
- `where`: 70% faster than native where.exe

## Contributing

### Development Workflow

1. Create feature branch
1. Make changes following Rust style guidelines
1. Run tests: `cargo test --workspace`
1. Run clippy: `cargo clippy --workspace`
1. Format code: `cargo fmt --all`
1. Build and validate: `make validate`
1. Submit pull request

### Code Style

- Follow Rust standard naming conventions
- Use `rustfmt` for formatting
- Address all clippy warnings
- Document public APIs
- Include unit tests for new features

## License

MIT OR Apache-2.0

## Support

For issues or questions:

- Email: david.martel@auricleinc.com
- GitHub: https://github.com/david-t-martel/winutils

______________________________________________________________________

*Last Updated: September 2024*
*Build System Version: 1.0.0*
