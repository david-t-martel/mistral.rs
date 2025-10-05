# WinUtils Build Automation Script Inventory

**Generated**: 2025-09-30
**Purpose**: Complete inventory of all automation scripts before consolidation

______________________________________________________________________

## Summary Statistics

| Category                    | Count        | Total Size                |
| --------------------------- | ------------ | ------------------------- |
| PowerShell Scripts (\*.ps1) | 25           | ~95 KB                    |
| Shell Scripts (\*.sh)       | 15           | ~45 KB                    |
| Makefiles                   | 3            | 1,784 lines               |
| Batch/CMD Files             | 2            | ~2 KB                     |
| **TOTAL**                   | **45 files** | **~142 KB + 1,784 lines** |

______________________________________________________________________

## Categorized Inventory

### 1. BUILD SCRIPTS (Primary Build System)

#### Active Build Scripts

```
✅ build-all.ps1 (11 KB, 340 lines)
   Purpose: Unified build for 78+ utilities
   Features: Parallel, profiles, clean, test
   Status: CANONICAL (will be renamed to build.ps1)
   Keep: YES

⚠️ build.ps1 (5.1 KB, 180 lines)
   Purpose: Simple build with winpath order
   Features: 8 actions, clean interface
   Status: SUPERSEDED by build-all.ps1
   Keep: NO (delete after consolidation)

❌ build-simplified.ps1 (1 byte)
   Purpose: Unknown (empty file)
   Status: BROKEN/ABANDONED
   Keep: NO (delete immediately)

⚠️ temp-build.ps1 (1.5 KB, 45 lines)
   Purpose: Debug/temporary build script
   Status: TEMPORARY
   Keep: NO (archive or delete)

✅ simple-build.cmd (CMD wrapper)
   Purpose: Minimal fallback build
   Status: USEFUL for CMD-only environments
   Keep: YES (move to scripts/build/)
```

#### Build System Core

```
✅ Makefile (832 lines)
   Purpose: PRIMARY build system (mandatory)
   Features: Complete build orchestration
   Status: PRODUCTION, enforces winpath-first order
   Keep: YES (unchanged)

✅ Makefile.toml (663 lines, cargo-make)
   Purpose: Performance optimization layer
   Features: Parallel builds, sccache, PGO
   Status: PRODUCTION, 40% faster builds
   Keep: YES (unchanged)

✅ derive-utils/Makefile (289 lines)
   Purpose: Derive utilities sub-build
   Status: Required by main Makefile
   Keep: YES (unchanged)
```

______________________________________________________________________

### 2. DEPLOYMENT SCRIPTS

#### Production Deployment (Keep All)

```
✅ deploy/WinUtils-Deployment-Framework.ps1 (1,193 lines)
   Purpose: PRODUCTION deployment system
   Features: Deploy, Rollback, Update, Status, Health, Benchmark, Validate
   Modes: Individual, Monolithic, Hybrid
   Status: CANONICAL deployment framework
   Keep: YES

✅ deploy/WinUtils-Manager.ps1 (590 lines)
   Purpose: Management and monitoring companion
   Features: Monitor, Update, Switch, Clean, Report, Interactive
   Status: PRODUCTION management tool
   Keep: YES

✅ deploy/deploy-unix.sh (unknown size)
   Purpose: Unix/WSL/Linux deployment
   Status: UNIQUE functionality (cross-platform)
   Keep: YES

✅ deploy-to-local.sh (5.4 KB, 162 lines)
   Purpose: Local deployment to ~/.local/bin
   Features: Backup, archive, canonical naming
   Status: CANONICAL Unix deployment
   Keep: YES (root level)
```

#### Redundant Deployment (Delete)

```
❌ scripts/deploy-to-local.sh (5.4 KB, IDENTICAL to root version)
   Status: EXACT DUPLICATE
   Keep: NO (delete immediately)

❌ scripts/deploy.ps1 (383 lines)
   Purpose: Basic deployment (local/enterprise/ci)
   Status: SUPERSEDED by WinUtils-Deployment-Framework.ps1
   Keep: NO (delete after consolidation)

❌ scripts/deploy/deploy-windows.ps1 (101 lines)
   Purpose: Minimal Windows deployment
   Status: SUPERSEDED by WinUtils-Deployment-Framework.ps1
   Keep: NO (delete after consolidation)
```

______________________________________________________________________

### 3. VALIDATION AND TESTING SCRIPTS

#### Active Validation (Keep)

```
✅ scripts/validate.ps1 (299 lines)
   Purpose: PRIMARY validation suite
   Features: All 77 coreutils + derive-utils testing
   Coverage: Comprehensive functionality validation
   Status: PRODUCTION testing framework
   Keep: YES (move to scripts/validation/)

✅ scripts/test-gnu-compat.ps1 (unknown size)
   Purpose: GNU compatibility testing
   Status: SPECIALIZED test suite
   Keep: YES (move to scripts/validation/)

✅ scripts/verify-gitbash-integration.sh (unknown size)
   Purpose: Git Bash path integration tests
   Status: CRITICAL for winpath validation
   Keep: YES (already in scripts/)

✅ derive-utils/test_all_git_bash_paths.sh (unknown size)
   Purpose: Unit tests for winpath normalization
   Status: UNIT test suite
   Keep: YES (unchanged, used by Makefile)

✅ scripts/validate.sh (unknown size)
   Purpose: Unix validation script
   Status: Cross-platform testing
   Keep: YES (already in scripts/)
```

#### Validation Tools (Consolidate)

```
⚠️ verify-build-location.ps1 (7.3 KB)
   Purpose: Build output verification (Windows)
   Status: DIAGNOSTIC tool
   Keep: YES (consolidate → scripts/validation/verify-build-output.ps1)

⚠️ verify-build-location.sh (7.4 KB)
   Purpose: Build output verification (Unix)
   Status: REDUNDANT on Windows (same as .ps1)
   Keep: NO (delete, PowerShell version sufficient)
```

#### Obsolete Validation (Archive)

```
📦 validate-workspace.ps1 (337 lines)
   Purpose: Workspace structure validation
   Status: ONE-TIME migration tool, likely obsolete
   Keep: NO (archive to scripts/archive/)
```

______________________________________________________________________

### 4. SCCACHE MANAGEMENT

#### Active sccache Scripts (Keep)

```
✅ scripts/setup-sccache.sh (356 lines)
   Purpose: COMPREHENSIVE sccache setup
   Features: Install, configure, test, optimize, guide
   Status: PRODUCTION-GRADE
   Keep: YES (canonical sccache setup)

✅ monitor-sccache.ps1 (12 lines)
   Purpose: Real-time cache statistics monitoring
   Status: USEFUL Windows utility
   Keep: YES (move to scripts/cache/)

✅ reset-sccache.ps1 (23 lines)
   Purpose: Cache clearing with confirmation
   Status: USEFUL Windows utility
   Keep: YES (move to scripts/cache/)

✅ configure-sccache.cmd (35 lines)
   Purpose: CMD session environment setup
   Status: NICHE but useful for CMD users
   Keep: YES (move to scripts/cache/)
```

#### Broken sccache Scripts (Delete)

```
❌ setup-sccache.ps1 (1 byte)
   Status: BROKEN/EMPTY
   Keep: NO (delete immediately)
```

______________________________________________________________________

### 5. INSTALLATION SCRIPTS

#### Active Installation (Keep All)

```
✅ scripts/install/install.ps1 (unknown size)
   Purpose: Windows installation
   Status: PRODUCTION installer
   Keep: YES (unchanged)

✅ scripts/install/install.sh (unknown size)
   Purpose: Unix/WSL installation
   Status: PRODUCTION installer
   Keep: YES (unchanged)

✅ scripts/install/uninstall.ps1 (unknown size)
   Purpose: Windows uninstallation
   Status: PRODUCTION utility
   Keep: YES (unchanged)

✅ scripts/install/update.ps1 (unknown size)
   Purpose: Update installed utilities
   Status: PRODUCTION utility
   Keep: YES (unchanged)

✅ scripts/install/shell-integration.sh (unknown size)
   Purpose: Shell integration (PATH, completions)
   Status: PRODUCTION utility
   Keep: YES (unchanged)
```

______________________________________________________________________

### 6. PACKAGE CREATION AND RELEASE

#### Active Packaging (Keep)

```
✅ scripts/deployment/build-installer.ps1 (unknown size)
   Purpose: Installer creation
   Status: PRODUCTION packaging
   Keep: YES (unchanged location)

✅ scripts/deployment/release-automation.ps1 (unknown size)
   Purpose: Automated release workflow
   Status: PRODUCTION release process
   Keep: YES (unchanged location)

✅ scripts/packages/chocolatey/tools/chocolateyinstall.ps1
   Purpose: Chocolatey package install
   Status: PRODUCTION package
   Keep: YES (unchanged)

✅ scripts/packages/chocolatey/tools/chocolateyuninstall.ps1
   Purpose: Chocolatey package uninstall
   Status: PRODUCTION package
   Keep: YES (unchanged)
```

______________________________________________________________________

### 7. PERFORMANCE AND BENCHMARKING

#### Active Performance Tools (Keep)

```
✅ benchmark-build.ps1 (9.3 KB)
   Purpose: Build performance benchmarking
   Status: USEFUL diagnostic tool
   Keep: YES (move to scripts/performance/)

✅ benchmarks/scripts/benchmark-runner.sh (unknown size)
   Purpose: Runtime performance benchmarking
   Status: PRODUCTION benchmarking
   Keep: YES (unchanged, used by benchmarks/)
```

______________________________________________________________________

### 8. CI/CD SCRIPTS

#### Active CI/CD (Keep)

```
✅ scripts/ci/install-dependencies.sh (unknown size)
   Purpose: CI/CD dependency installation
   Status: PRODUCTION CI script
   Keep: YES (unchanged)
```

______________________________________________________________________

### 9. COREUTILS-SPECIFIC SCRIPTS

#### Coreutils Sub-Project (Keep All)

```
✅ coreutils/scripts/build-all.sh (unknown size)
   Purpose: Coreutils workspace build
   Status: Used by main Makefile
   Keep: YES (unchanged)

✅ coreutils/scripts/install.sh (unknown size)
   Purpose: Coreutils installation
   Status: Used by main Makefile
   Keep: YES (unchanged)

✅ coreutils/scripts/test.sh (unknown size)
   Purpose: Coreutils testing
   Status: Used by main Makefile
   Keep: YES (unchanged)
```

______________________________________________________________________

### 10. MAINTENANCE AND DEBUG SCRIPTS

#### Archive/Delete

```
📦 fix-compilation-errors.ps1 (241 lines)
   Purpose: Automated error fixing during development
   Status: DEVELOPMENT tool, likely obsolete
   Keep: NO (archive to scripts/archive/)

📦 migrate-workspace.ps1 (501 lines)
   Purpose: One-time workspace migration
   Status: OBSOLETE (migration complete)
   Keep: NO (archive to scripts/archive/ or delete)
```

______________________________________________________________________

## Consolidation Action Items

### IMMEDIATE DELETIONS (Phase 1)

- ❌ `build-simplified.ps1` (empty)
- ❌ `setup-sccache.ps1` (broken)
- ❌ `scripts/deploy-to-local.sh` (duplicate)

### CONSOLIDATIONS (Phase 2)

- ♻️ `build-all.ps1` → `build.ps1` (rename as primary)
- ♻️ `verify-build-location.ps1` → `scripts/validation/verify-build-output.ps1`
- ❌ `verify-build-location.sh` (delete, redundant)
- ❌ `build.ps1` (old version, delete)

### ARCHIVING (Phase 3)

- 📦 `fix-compilation-errors.ps1` → `scripts/archive/`
- 📦 `migrate-workspace.ps1` → `scripts/archive/`
- 📦 `validate-workspace.ps1` → `scripts/archive/`
- 📦 `temp-build.ps1` → `scripts/archive/`

### REORGANIZATIONS (Phase 4)

- 📁 `monitor-sccache.ps1` → `scripts/cache/`
- 📁 `reset-sccache.ps1` → `scripts/cache/`
- 📁 `configure-sccache.cmd` → `scripts/cache/`
- 📁 `benchmark-build.ps1` → `scripts/performance/`
- 📁 `simple-build.cmd` → `scripts/build/`

### DELETIONS AFTER CONSOLIDATION (Phase 2 cleanup)

- ❌ `scripts/deploy.ps1` (superseded)
- ❌ `scripts/deploy/deploy-windows.ps1` (superseded)

______________________________________________________________________

## File Count Reduction

| Phase         | Before               | After   | Reduction         |
| ------------- | -------------------- | ------- | ----------------- |
| Current State | 45 files             | -       | -                 |
| After Phase 1 | 42 files             | -3      | 7%                |
| After Phase 2 | 38 files             | -7      | 16%               |
| After Phase 3 | 34 files             | -11     | 24%               |
| After Phase 4 | 34 files (organized) | -11     | 24%               |
| **Final**     | **34 active files**  | **-11** | **24% reduction** |

**Plus**: 4 archived scripts preserved for reference

**Note**: Reduction is conservative because we're preserving all unique functionality and cross-platform scripts. The main benefit is **organization** and **elimination of duplication**, not aggressive deletion.

______________________________________________________________________

## Scripts That MUST NOT Change

These scripts are critical and must remain unchanged:

1. ✅ **Makefile** - Mandatory build system (per CLAUDE.md)
1. ✅ **Makefile.toml** - Performance optimization
1. ✅ **derive-utils/Makefile** - Required by main Makefile
1. ✅ **deploy/WinUtils-Deployment-Framework.ps1** - Production deployment
1. ✅ **deploy/WinUtils-Manager.ps1** - Production management
1. ✅ **scripts/install/**\* - All installation scripts
1. ✅ **scripts/packages/**\* - All packaging scripts
1. ✅ **coreutils/scripts/**\* - Coreutils sub-project scripts
1. ✅ **benchmarks/scripts/**\* - Benchmarking framework

______________________________________________________________________

## Summary

- **Total Scripts**: 45 files (142 KB + 1,784 lines)
- **To Delete**: 7 files (empty, broken, duplicates)
- **To Archive**: 4 files (obsolete development tools)
- **To Reorganize**: 5 files (better directory structure)
- **To Keep Unchanged**: 29 files (production systems)

**Result**: Cleaner, more maintainable build automation with zero loss of functionality.
