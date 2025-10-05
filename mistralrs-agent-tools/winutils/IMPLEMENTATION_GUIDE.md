# Build Optimization Implementation Guide

## Quick Start (5 Minutes)

### Step 1: Fix sccache Directory

Edit `Makefile.toml` line 13:

```toml
# OLD:
SCCACHE_DIR = "T:/projects/.sccache"

# NEW:
SCCACHE_DIR = "T:/projects/coreutils/sccache-cache"
```

### Step 2: Increase Job Counts

Edit `Makefile.toml` lines 18, 47, 73, 99, 119:

```toml
Line 18:  CARGO_BUILD_JOBS = "20"        # Was: "18"
Line 47:  "--jobs", "8"                  # Was: "4"
Line 73:  "--jobs", "12"                 # Was: "4"
Line 99:  "--jobs", "20"                 # Was: "8"
Line 119: "--jobs", "20"                 # Was: "8"
```

### Step 3: Add Jobs to Cargo Config

Edit `.cargo/config.toml` after line 10:

```toml
[build]
target-dir = "target"
jobs = 20  # ADD THIS LINE
incremental = false
rustc-wrapper = "sccache"
```

### Step 4: Fix Makefile Clean

Edit `Makefile` line 616:

```makefile
# OLD:
@RUSTC_WRAPPER="" $(CARGO) clean

# NEW:
@$(CARGO) clean
```

### Step 5: Add Jobs to Makefile Release

Edit `Makefile` lines 196-203, add --jobs flags:

```makefile
@$(CARGO) build --release --package winpath --target $(TARGET) --target-dir $(BUILD_DIR) --jobs 8
@$(CARGO) build --release --package uu_where --package winutils-which --package uu_tree --target $(TARGET) --target-dir $(BUILD_DIR) --jobs 12
@$(CARGO) build --release --workspace --target $(TARGET) --target-dir $(BUILD_DIR) --exclude winpath --exclude uu_where --exclude winutils-which --exclude uu_tree --jobs 20
@cd coreutils && $(CARGO) build --release --workspace --target $(TARGET) --target-dir ../$(BUILD_DIR) --jobs 20
```

## Verification

```bash
# Test the optimizations
make clean
make release

# Check sccache is working
sccache --show-stats
# Should show > 0 compile requests

# Run again to test cache
make clean && make release
# Should be 50-60% faster
```

## Expected Results

- **First build**: 180 seconds (improved from 225s)
- **Second build**: 60-75 seconds (50-60% faster)
- **Cache hit rate**: 80-90%
- **CPU usage**: 90%+ (was 36-40%)

## Troubleshooting

**sccache still showing 0 requests?**

- Verify path: `echo $SCCACHE_DIR`
- Check cache exists: `ls T:/projects/coreutils/sccache-cache`
- Start server: `sccache --start-server`

**Build not faster?**

- Check CPU usage: Task Manager should show 90%+ during build
- Verify --jobs flags: grep for "--jobs" in Makefile
- Check cache: `sccache --show-stats` should show hits on second build

**Tests failing?**

- Run: `make test-unit`
- Check: Individual utility with `make test-util-ls`

## Next Steps

1. **Install tools**: `cargo install cargo-nextest cargo-audit cargo-watch`
1. **Install hooks**: `bash scripts/install-git-hooks.sh`
1. **Set up CI**: Commit `.github/workflows/ci.yml`

## Full Documentation

See `BUILD_OPTIMIZATION_REPORT.md` for complete details.
