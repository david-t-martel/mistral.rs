# WinPath API Migration - Detailed Changes

## Executive Summary

Fixed all wrapper utilities to use the correct `shared/winpath` API instead of the outdated `derive-utils/winpath` API. The main change is removing `PathContext` references (which don't exist in the production API) and using automatic context detection.

## Files Modified (7 files)

### 1. derive-utils/bash-wrapper/src/lib.rs

**Line 30-40**: Removed `target_context` field

```diff
  pub struct BashOptions {
      pub normalize_paths: bool,
-     pub target_context: PathContext,
      pub working_directory: Option<PathBuf>,
      ...
  }
```

**Line 47**: Removed `target_context` default value

```diff
  impl Default for BashOptions {
      fn default() -> Self {
          Self {
              normalize_paths: true,
-             target_context: PathContext::GitBash,
              working_directory: None,
              ...
          }
      }
  }
```

**Line 77**: Changed PathNormalizer initialization

```diff
  impl BashWrapper {
      pub fn new(options: BashOptions) -> Result<Self> {
-         let normalizer = PathNormalizer::with_context(PathContext::GitBash);
+         let normalizer = PathNormalizer::new();
          ...
      }
  }
```

### 2. derive-utils/cmd-wrapper/src/lib.rs

**Line 39**: Updated use statement

```diff
- use winpath::{PathNormalizer, PathContext};
+ use winpath::PathNormalizer;
```

**Line 71**: Removed `target_context` field

```diff
  pub struct CmdOptions {
-     pub target_context: PathContext,
      ...
  }
```

**Line 106**: Removed `target_context` default

```diff
  impl Default for CmdOptions {
      fn default() -> Self {
          Self {
-             target_context: PathContext::Windows,
              ...
          }
      }
  }
```

**Line 135+**: Removed `target_context()` builder method

```diff
- pub fn target_context(mut self, context: PathContext) -> Self {
-     self.target_context = context;
-     self
- }
```

### 3. derive-utils/pwsh-wrapper/src/lib.rs

**Line 15**: Updated use statement

```diff
- use winpath::{PathNormalizer, PathContext};
+ use winpath::PathNormalizer;
```

**Line 32**: Removed `target_context` field

```diff
  pub struct PwshOptions {
-     pub target_context: PathContext,
      ...
  }
```

**Line 47**: Removed `target_context` default

```diff
  impl Default for PwshOptions {
      fn default() -> Self {
          Self {
-             target_context: PathContext::Windows,
              ...
          }
      }
  }
```

### 4. derive-utils/fd-wrapper/src/lib.rs

**Line 45**: Updated use statement

```diff
- use winpath::{PathNormalizer, PathContext};
+ use winpath::PathNormalizer;
```

**Line 214**: Removed `output_context` field

```diff
  pub struct FdOptions {
-     pub output_context: PathContext,
      ...
  }
```

**Line 243**: Removed `output_context` default

```diff
  impl Default for FdOptions {
      fn default() -> Self {
          Self {
-             output_context: PathContext::Auto,
              ...
          }
      }
  }
```

**Line 317+**: Removed `output_context()` builder method

```diff
- pub fn output_context(mut self, context: PathContext) -> Self {
-     self.output_context = context;
-     self
- }
```

### 5. derive-utils/fd-wrapper/src/main.rs

**Line 17**: Removed PathContext import

```diff
- use winpath::PathContext;
```

**Lines 184-192**: Removed From impl

```diff
- impl From<ContextArg> for PathContext {
-     fn from(arg: ContextArg) -> Self {
-         match arg {
-             ContextArg::Windows => PathContext::Windows,
-             ContextArg::Gitbash => PathContext::GitBash,
-             ContextArg::Wsl => PathContext::WSL,
-             ContextArg::Cygwin => PathContext::Cygwin,
-             ContextArg::Auto => PathContext::Auto,
-         }
-     }
- }
```

### 6. derive-utils/rg-wrapper/src/lib.rs

**Line 51**: Updated use statement

```diff
- use winpath::{PathNormalizer, PathContext};
+ use winpath::PathNormalizer;
```

**Line 154**: Removed `output_context` field

```diff
  pub struct RgOptions {
-     pub output_context: PathContext,
      ...
  }
```

**Line 201**: Removed `output_context` default

```diff
  impl Default for RgOptions {
      fn default() -> Self {
          Self {
-             output_context: PathContext::Auto,
              ...
          }
      }
  }
```

**Line 273+**: Removed `output_context()` builder method

```diff
- pub fn output_context(mut self, context: PathContext) -> Self {
-     self.output_context = context;
-     self
- }
```

### 7. derive-utils/rg-wrapper/src/main.rs

**Line 17**: Removed PathContext import

```diff
- use winpath::PathContext;
```

**Lines 241-249**: Removed From impl

```diff
- impl From<ContextArg> for PathContext {
-     fn from(arg: ContextArg) -> Self {
-         match arg {
-             ContextArg::Windows => PathContext::Windows,
-             ContextArg::Gitbash => PathContext::GitBash,
-             ContextArg::Wsl => PathContext::WSL,
-             ContextArg::Cygwin => PathContext::Cygwin,
-             ContextArg::Auto => PathContext::Auto,
-         }
-     }
- }
```

## API Migration Guide

### Old API (derive-utils/winpath)

```rust
use winpath::{PathNormalizer, PathContext};

let normalizer = PathNormalizer::with_context(PathContext::GitBash);
let result = normalizer.normalize_to_context(path, PathContext::Windows)?;
```

### New API (shared/winpath)

```rust
use winpath::PathNormalizer;

let normalizer = PathNormalizer::new();  // Auto-detects context
let result = normalizer.normalize(path)?;  // Auto-converts to Windows
```

## Key Differences

| Feature               | Old API                           | New API                           |
| --------------------- | --------------------------------- | --------------------------------- |
| Context enum          | `PathContext`                     | `PathFormat`                      |
| Context specification | Manual (`with_context()`)         | Automatic (environment detection) |
| Target format         | Manual (`normalize_to_context()`) | Automatic (always Windows)        |
| Configuration         | Via constructor params            | Via `NormalizerConfig`            |
| Caching               | Manual                            | Built-in with LRU cache           |

## Testing Recommendations

1. **Compilation Test**:

   ```bash
   cd T:/projects/coreutils/winutils
   make clean && make release
   ```

1. **Runtime Test**:

   ```bash
   make test
   make validate-all-77
   ```

1. **Path Normalization Tests**:

   - Test WSL paths: `/mnt/c/users/david`
   - Test Cygwin paths: `/cygdrive/c/users/david`
   - Test Git Bash paths: `/c/users/david`
   - Test Windows paths: `C:\users\david`

## Impact Assessment

### Positive

- ✅ Uses production-quality winpath library
- ✅ Automatic context detection (less error-prone)
- ✅ Consistent API across all wrappers
- ✅ Better performance with built-in caching
- ✅ Thread-safe operations

### Neutral

- ⚠️ Removes manual context selection (auto-detection only)
- ⚠️ Always normalizes to Windows format (not configurable)

### Negative

- ❌ None identified

## Verification Status

- ✅ All PathContext references removed from wrapper code
- ✅ All wrappers use new PathNormalizer::new() API
- ✅ No compilation errors expected
- ⏳ Pending: Build verification
- ⏳ Pending: Runtime testing

## Recommendation

The deprecated `derive-utils/winpath/` directory should be removed once all wrappers are confirmed working with the new API to prevent future confusion.

______________________________________________________________________

**Generated**: 2025-01-30
**Author**: Claude AI (code migration)
**Status**: Ready for testing
