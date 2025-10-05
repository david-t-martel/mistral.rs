# Winutils Workspace Optimization Plan

## Architectural Impact Assessment: **HIGH**

The current structure has significant architectural issues requiring comprehensive reorganization to eliminate duplication and establish clean boundaries.

## Current Structure Analysis

### Issues Identified

1. **Duplicate Utilities**

   - `where` exists in both `derive-utils/where/` and root `where/`
   - Multiple Cargo.toml files for same utilities
   - Inconsistent naming patterns (uu_where vs where)

1. **Scattered Coreutils**

   - 77+ individual coreutils in `coreutils/src/*/` each with own Cargo.toml
   - No clear separation between standard coreutils and custom utilities
   - Deep nesting causing long path issues on Windows

1. **Shared Library Issues**

   - `winutils-core` has compilation errors
   - `winpath` is duplicated in multiple locations
   - No clear dependency hierarchy

1. **Build Configuration Chaos**

   - Multiple Makefiles (Makefile, Makefile-optimized, Makefile.old)
   - Inconsistent profile configurations
   - Redundant dependency specifications

## Optimized Workspace Structure

```
T:\projects\coreutils\winutils\
├── Cargo.toml                    # Workspace root configuration
├── Cargo.lock
├── rust-toolchain.toml          # Unified Rust version
├── .cargo/
│   └── config.toml              # Shared build configuration
│
├── crates/                       # All Rust crates
│   ├── libs/                    # Shared libraries
│   │   ├── winutils-core/      # Core functionality
│   │   ├── winpath/            # Path normalization
│   │   └── common/             # Common utilities
│   │
│   ├── utils/                   # Individual utilities
│   │   ├── standard/           # Standard coreutils
│   │   │   ├── cat/
│   │   │   ├── ls/
│   │   │   ├── cp/
│   │   │   └── ...
│   │   │
│   │   └── extended/           # Windows-specific extensions
│   │       ├── where/
│   │       ├── which/
│   │       ├── tree/
│   │       └── wrappers/
│   │           ├── find/
│   │           ├── grep/
│   │           ├── cmd/
│   │           ├── pwsh/
│   │           └── bash/
│   │
│   └── tools/                   # Development tools
│       ├── generator/
│       └── benchmarks/
│
├── docs/                        # Consolidated documentation
│   ├── architecture/
│   ├── api/
│   ├── guides/
│   └── README.md
│
├── scripts/                     # Build and deployment scripts
│   ├── build.ps1
│   ├── deploy.ps1
│   ├── test.ps1
│   └── ci/
│
├── tests/                       # Integration tests
│   └── integration/
│
└── target/                      # Build output
```

## Workspace Cargo.toml Structure

```toml
[workspace]
resolver = "2"
members = [
    # Libraries (build order matters)
    "crates/libs/winpath",
    "crates/libs/common",
    "crates/libs/winutils-core",

    # Standard utilities
    "crates/utils/standard/*",

    # Extended utilities
    "crates/utils/extended/*",
    "crates/utils/extended/wrappers/*",

    # Tools
    "crates/tools/*",
]

# Exclude paths
exclude = ["target", "tests/fixtures"]

[workspace.package]
version = "0.2.0"
authors = ["David Martel <david.martel@auricleinc.com>"]
edition = "2021"
rust-version = "1.85.0"
license = "MIT OR Apache-2.0"
repository = "https://github.com/david-t-martel/winutils"

[workspace.dependencies]
# Core dependencies (version pinned)
anyhow = "1.0.95"
clap = { version = "4.5.32", features = ["derive", "env", "wrap_help", "color"] }
serde = { version = "1.0.217", features = ["derive"] }
serde_json = "1.0.134"
thiserror = "2.0.9"
tokio = { version = "1.42.0", features = ["full"] }

# Internal libraries
winpath = { path = "crates/libs/winpath" }
winutils-core = { path = "crates/libs/winutils-core" }
common = { path = "crates/libs/common" }

# Windows-specific
windows = "0.60.0"
windows-sys = "0.60.0"
winapi-util = "0.1.9"

# Path handling
dunce = "1.0.5"
path-slash = "0.2.1"
normalize-path = "0.2.1"

# Performance
rayon = "1.10.0"
dashmap = "6.1.0"
lru = "0.12.5"
ahash = "0.8.11"

# Common utilities
regex = "1.11.1"
glob = "0.3.2"
walkdir = "2.5.0"
dirs = "5.0.1"
which = "7.0.1"

[workspace.lints.rust]
unsafe_code = "warn"
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }

[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
strip = true
panic = "abort"

[profile.release-windows]
inherits = "release"
# Windows-specific optimizations
overflow-checks = false
```

## Migration Plan

### Phase 1: Foundation (Week 1)

1. Create new directory structure
1. Set up workspace Cargo.toml with proper dependencies
1. Configure shared .cargo/config.toml

### Phase 2: Library Consolidation (Week 1-2)

1. Merge duplicate `winpath` implementations
1. Fix `winutils-core` compilation issues
1. Extract common code into `common` library
1. Update all internal dependencies

### Phase 3: Utility Migration (Week 2-3)

1. Identify and remove duplicate utilities
1. Move standard coreutils to `crates/utils/standard/`
1. Move extended utilities to `crates/utils/extended/`
1. Update all Cargo.toml files to use workspace dependencies

### Phase 4: Build System (Week 3)

1. Create unified build scripts
1. Remove duplicate Makefiles
1. Set up CI/CD pipeline
1. Create deployment scripts

### Phase 5: Documentation (Week 4)

1. Consolidate all documentation
1. Generate API documentation
1. Create migration guide
1. Update README

## Pattern Compliance Checklist

- [x] **Workspace Structure**: Following Cargo workspace best practices
- [x] **Dependency Management**: Centralized in workspace.dependencies
- [x] **Build Profiles**: Unified and optimized
- [x] **Directory Organization**: Clear separation of concerns
- [x] **Path Lengths**: Reduced nesting to avoid Windows issues
- [x] **Feature Flags**: Proper feature organization
- [x] **Version Management**: Workspace-level versioning

## Specific Violations to Fix

1. **DRY Violation**: Multiple `where` implementations
1. **Single Responsibility**: Utilities mixed with libraries
1. **Dependency Inversion**: Direct paths instead of workspace deps
1. **Open/Closed**: No clear extension points

## Recommended Refactoring

### Immediate Actions

1. Remove duplicate `where/` directory at root
1. Consolidate all Makefiles into single build.ps1
1. Fix winutils-core compilation errors

### Short-term (1-2 weeks)

1. Implement new directory structure
1. Migrate to workspace dependencies
1. Standardize all Cargo.toml files

### Long-term (1 month)

1. Complete full migration
1. Set up automated testing
1. Document all APIs

## Performance Optimizations

1. **Build Performance**

   - Use sccache for compilation caching
   - Enable parallel builds with proper dependency ordering
   - Optimize link-time with shared target directory

1. **Runtime Performance**

   - Enable LTO for release builds
   - Use profile-guided optimization
   - Implement lazy loading for large utilities

1. **Development Performance**

   - Fast incremental builds for development
   - Parallel test execution
   - Hot-reload for documentation

## Feature Flag Strategy

```toml
[features]
default = ["standard-utils"]
standard-utils = []
extended-utils = ["where", "which", "tree"]
wrappers = ["find-wrapper", "grep-wrapper"]
all = ["standard-utils", "extended-utils", "wrappers"]
minimal = []  # Only core libraries
```

## Dependency Graph

```
winpath (foundational)
    ↓
common
    ↓
winutils-core
    ↓
individual utilities
```

## Build Commands

```powershell
# Full build
cargo build --workspace --release

# Build specific feature set
cargo build --workspace --features extended-utils

# Build single utility
cargo build -p where --release

# Run all tests
cargo test --workspace

# Generate documentation
cargo doc --workspace --no-deps
```

## Success Metrics

1. **Zero Duplication**: No duplicate implementations
1. **Build Time**: < 2 minutes for full release build
1. **Binary Size**: < 1MB per utility
1. **Test Coverage**: > 80% for all utilities
1. **Documentation**: 100% public API documented

## Long-term Implications

### Positive

- Easier maintenance with clear structure
- Faster builds with proper caching
- Better code reuse through shared libraries
- Simpler dependency management
- Clear extension points for new utilities

### Risks

- Migration may temporarily break existing builds
- Need to update all documentation and scripts
- Developers need to learn new structure

## Conclusion

This optimization will transform the winutils project from a scattered collection of utilities into a well-organized, maintainable Rust workspace following industry best practices. The migration will eliminate all duplication, improve build times, and establish a solid foundation for future development.
