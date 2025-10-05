# WinUtils Build System Optimization

## 📊 Current Status

**Problem**: Build system severely underperforming

- sccache: **0 compile requests** (broken configuration)
- CPU usage: **36-40%** (only 4-8 of 22 cores active)
- Build time: **150-225 seconds** (target: 60-90s)

**Root Causes Identified**:

1. Wrong sccache cache directory path
1. Conservative job counts (4-8 vs optimal 8-20)
1. Makefile actively disables sccache during clean
1. No explicit jobs configuration in Cargo config

## 🚀 Quick Fix (Choose One Method)

### Method 1: Automated Script (Recommended)

```bash
# Apply all fixes automatically
bash APPLY_OPTIMIZATIONS.sh

# Test the changes
make clean && make release
sccache --show-stats  # Should show activity
```

### Method 2: Manual Implementation

Follow the step-by-step guide:

```bash
cat IMPLEMENTATION_GUIDE.md
```

## 📈 Expected Results

| Metric     | Before | After      | Improvement          |
| ---------- | ------ | ---------- | -------------------- |
| Cold build | 225s   | 180s       | 20% faster           |
| Warm build | 150s   | **60-75s** | **2-3x faster**      |
| CPU usage  | 40%    | 90%+       | 2.5x utilization     |
| Cache hits | 0%     | 80-90%     | Infinite improvement |

## 🔍 Verification Steps

```bash
# 1. Baseline before changes
sccache --show-stats  # Record initial state
time make clean && time make release  # Record time

# 2. Apply optimizations (choose method above)

# 3. Verify sccache is working
make clean
make release
sccache --show-stats  # Should show compile requests

# 4. Test cache effectiveness
make clean && make release  # Should be 50-60% faster

# 5. Validate all utilities
make validate-all-77  # Should pass
```

## 📁 Documentation Files

- **OPTIMIZATION_SUMMARY.md** - Executive overview (this file)
- **IMPLEMENTATION_GUIDE.md** - Step-by-step manual instructions
- **BUILD_OPTIMIZATION_REPORT.md** - Complete technical analysis
- **APPLY_OPTIMIZATIONS.sh** - Automated fix script
- **scripts/install-git-hooks.sh** - Pre-commit quality gates
- **.github/workflows/ci.yml** - GitHub Actions CI/CD pipeline

## 🎯 What Gets Changed

### File 1: Makefile.toml (5 changes)

- Line 13: Fix sccache directory path
- Line 18: Increase CARGO_BUILD_JOBS to 20
- Line 47: Increase winpath jobs to 8
- Line 73: Increase derive jobs to 12
- Lines 99, 119: Increase core/coreutils jobs to 20

### File 2: .cargo/config.toml (1 change)

- Line 11: Add `jobs = 20`

### File 3: Makefile (2 changes)

- Line 616: Remove RUSTC_WRAPPER="" bypass
- Lines 196-203: Add --jobs flags to cargo commands

## 🔧 Troubleshooting

**sccache still showing 0 requests?**

```bash
# Check configuration
echo $SCCACHE_DIR
ls -la T:/projects/coreutils/sccache-cache

# Restart sccache server
sccache --stop-server
sccache --start-server
sccache --show-stats
```

**Build not faster?**

```bash
# Check CPU usage during build (should be 90%+)
# Open Task Manager and watch during: make release

# Verify job counts were applied
grep -n "jobs" Makefile.toml .cargo/config.toml Makefile
```

**Need to revert changes?**

```bash
# If you used the automated script
mv Makefile.toml.backup Makefile.toml
mv .cargo/config.toml.backup .cargo/config.toml
mv Makefile.backup Makefile
```

## 🎓 Key Technical Insights

1. **sccache Cache Miss**: Directory path was `T:/projects/.sccache` but actual cache is at `T:/projects/coreutils/sccache-cache`

1. **Parallelism Bottleneck**: Only using 18% of available CPU cores due to conservative job counts (4-8 jobs on 22-core system)

1. **Cache Sabotage**: Makefile explicitly disabled sccache with `RUSTC_WRAPPER=""` during clean operations

1. **Optimal Job Distribution**:

   - winpath (1 package): 8 jobs
   - derive-utils (3 packages): 12 jobs
   - core utilities (80 packages): 20 jobs
   - Leave 2 cores for system overhead

1. **Build Order Preservation**: All optimizations maintain critical `winpath → derive-utils → coreutils` dependency chain

## 📊 Performance Monitoring

```bash
# Monitor sccache effectiveness
watch -n 1 'sccache --show-stats'

# Track build times
echo "$(date): $(time make clean && make release 2>&1 | grep real)" >> build-times.log

# CPU usage during build
# Use Task Manager (Windows) or htop (Linux/WSL)
```

## 🚀 Advanced Tools (Optional)

After applying core optimizations, consider:

```bash
# Install testing tools
cargo install cargo-nextest  # 2-3x faster tests

# Install security tools
cargo install cargo-audit cargo-deny

# Install pre-commit hooks
bash scripts/install-git-hooks.sh

# Set up CI/CD
# Commit .github/workflows/ci.yml to enable GitHub Actions
```

## 📞 Support

Issues with optimization?

1. Check IMPLEMENTATION_GUIDE.md for detailed steps
1. Review BUILD_OPTIMIZATION_REPORT.md for technical details
1. Verify all 77 utilities: `make validate-all-77`

______________________________________________________________________

**Status**: Ready for Implementation
**Time Required**: 10 minutes to apply, 3-5 minutes to test
**Risk Level**: Low (configuration only, easily reversible)
**Expected Improvement**: 2-3x faster builds (150s → 60-75s)

*Generated by DevOps Troubleshooting Agent - January 30, 2025*
