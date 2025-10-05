# WinUtils Compilation Fixes

## Issue 1: Orphan Rule Violations

### Files Affected:

- `shared/winutils-core/src/diagnostics.rs:614`
- `shared/winutils-core/src/version.rs:614`
- `shared/winutils-core/src/testing.rs:620`
- `shared/winutils-core/src/help.rs:541`

### Problem:

Implementing `fmt::Write` for `StandardStream` (from `termcolor` crate) violates the orphan rule.

### Solution:

Remove these implementations entirely. They are unnecessary because:

1. `StandardStream` already implements `std::io::Write`
1. `writeln!` macro uses `std::io::Write`, not `fmt::Write`
1. All existing code works correctly without this implementation

### Action:

Delete the following blocks from each file:

```rust
use std::io::Write;

impl fmt::Write for StandardStream {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}
```

## Issue 2: Sysinfo API Breaking Changes

### Files Affected:

- `shared/winutils-core/src/diagnostics.rs` (lines 15, 36-206)

### Problem:

Sysinfo 0.30+ removed trait-based extensions (`SystemExt`, `ProcessExt`, `DiskExt`, `NetworkExt`, `ComponentExt`).
Methods are now directly on types.

### Solution:

Update imports and method calls:

#### Old Code:

```rust
use sysinfo::{System, SystemExt, ProcessExt, DiskExt, NetworkExt, ComponentExt};
```

#### New Code:

```rust
use sysinfo::System;
```

All method calls remain the same - they're now inherent methods instead of trait methods.

## Issue 3: Missing Windows API Features

### File Affected:

- `Cargo.toml` (workspace dependencies)

### Problem:

Missing Windows API features causing compilation errors.

### Solution:

Add to `windows-sys` features in workspace dependencies:

```toml
windows-sys = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_Console",
    "Win32_System_SystemInformation",
    "Win32_System_SystemServices",  # ADD THIS
    "Win32_Security",
    "Win32_System_Threading",
    "Win32_System_ProcessStatus",
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_UI_Shell",
]}
```

## Issue 4: Generic Type Constraints

### Files Affected:

- `shared/winutils-core/src/testing.rs` (DiagnosticResult builder methods)

### Problem:

Ambiguous generic type constraints when chaining builder methods.

### Solution:

Ensure all builder methods return `Self` and use consistent type bounds.

Check methods like:

- `with_detail()`
- `with_recommendation()`

Make sure they have signature:

```rust
pub fn with_detail<S: Into<String>>(mut self, key: S, value: S) -> Self {
    self.details.insert(key.into(), value.into());
    self
}
```

## Build Configuration Optimization

### File: Cargo.toml (workspace level)

Add `Win32_System_SystemServices` to features list.

Current optimal profile configuration:

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = 3
strip = true
panic = "unwind"
debug = false

[profile.release-fast]
inherits = "release"
panic = "abort"
lto = "fat"
opt-level = 3
codegen-units = 1
```

These are already correctly configured.
