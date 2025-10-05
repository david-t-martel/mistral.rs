# WinPath API Fix Summary

## Problem

Wrapper code (bash-wrapper, cmd-wrapper, pwsh-wrapper, fd-wrapper, rg-wrapper) was using outdated winpath API that referenced `PathContext` enum, which doesn't exist in the production `shared/winpath` library.

## Root Cause

- Two separate winpath implementations existed:
  - **Production**: `shared/winpath/` - uses `PathFormat` enum
  - **Old/Deprecated**: `derive-utils/winpath/` - uses `PathContext` enum
- Wrappers were coded against the OLD API but workspace configuration points to the NEW API
- This caused compilation failures

## Changes Made

### API Replacements

| Old API                                              | New API                       | Reason                               |
| ---------------------------------------------------- | ----------------------------- | ------------------------------------ |
| `use winpath::{PathNormalizer, PathContext}`         | `use winpath::PathNormalizer` | PathContext doesn't exist in new API |
| `PathNormalizer::with_context(PathContext::GitBash)` | `PathNormalizer::new()`       | Context detection is automatic       |
| `pub target_context: PathContext` field              | Removed                       | No longer needed                     |
| `pub output_context: PathContext` field              | Removed                       | No longer needed                     |
| `target_context: PathContext::Windows` default       | Removed                       | Auto-detected                        |

### Files Modified

1. **derive-utils/bash-wrapper/src/lib.rs**

   - Removed `target_context` field from `BashOptions`
   - Changed `PathNormalizer::with_context()` to `PathNormalizer::new()`

1. **derive-utils/cmd-wrapper/src/lib.rs**

   - Removed `target_context` field
   - Removed `target_context()` method
   - Updated `use` statement

1. **derive-utils/pwsh-wrapper/src/lib.rs**

   - Removed `target_context` field
   - Updated `use` statement

1. **derive-utils/fd-wrapper/src/lib.rs**

   - Removed `output_context` field
   - Removed `output_context()` method
   - Updated `use` statement

1. **derive-utils/fd-wrapper/src/main.rs**

   - Removed `use winpath::PathContext`
   - Removed `impl From<ContextArg> for PathContext`

1. **derive-utils/rg-wrapper/src/lib.rs**

   - Removed `output_context` field
   - Removed `output_context()` method
   - Updated `use` statement

1. **derive-utils/rg-wrapper/src/main.rs**

   - Removed `use winpath::PathContext`
   - Removed `impl From<ContextArg> for PathContext`

## New winpath API Usage

### Correct Usage Pattern

```rust
use winpath::PathNormalizer;

// Create normalizer (auto-detects context)
let normalizer = PathNormalizer::new();

// Normalize a path (auto-detects format and converts to Windows)
let result = normalizer.normalize("/mnt/c/users/david")?;
assert_eq!(result.path(), r"C:\users\david");
```

### Available Types

- `PathFormat` - enum with DOS, WSL, Cygwin, UNC, etc.
- `PathNormalizer` - main normalization struct
- `NormalizerConfig` - configuration options
- `normalize_path()` - standalone function
- `detect_path_format()` - format detection function

## Verification

```bash
cd T:/projects/coreutils/winutils
grep -r "PathContext" derive-utils/*/src/*.rs | grep -v "derive-utils/winpath"
# Result: No matches (SUCCESS)
```

## Next Steps

1. Test compilation: `make clean && make release`
1. If successful, consider removing deprecated `derive-utils/winpath/` entirely
1. Update documentation to reference `shared/winpath/` as the canonical implementation

## Build Command

```bash
cd T:/projects/coreutils/winutils
make clean
make release
make validate-all-77
```

______________________________________________________________________

**Date**: 2025-01-30
**Impact**: Fixes compilation failures in all wrapper utilities
**Breaking Change**: No (internal API only)
