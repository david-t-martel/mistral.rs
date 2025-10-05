#!/usr/bin/env bash
# Apply WinUtils Build Optimizations
# This script applies all 5 critical fixes automatically

set -e

echo "🚀 Applying WinUtils Build Optimizations..."
echo ""

# Backup original files
echo "📦 Creating backups..."
cp Makefile.toml Makefile.toml.backup
cp .cargo/config.toml .cargo/config.toml.backup
cp Makefile Makefile.backup
echo "✅ Backups created"
echo ""

# Fix 1: sccache directory path
echo "🔧 Fix 1/5: Updating sccache directory path..."
sed -i 's|SCCACHE_DIR = "T:/projects/.sccache"|SCCACHE_DIR = "T:/projects/coreutils/sccache-cache"|' Makefile.toml
echo "✅ sccache directory fixed"
echo ""

# Fix 2: Increase CARGO_BUILD_JOBS
echo "🔧 Fix 2/5: Optimizing CARGO_BUILD_JOBS..."
sed -i 's|CARGO_BUILD_JOBS = "18"|CARGO_BUILD_JOBS = "20"|' Makefile.toml
echo "✅ CARGO_BUILD_JOBS set to 20"
echo ""

# Fix 3: Increase job counts in build tasks
echo "🔧 Fix 3/5: Optimizing build task job counts..."
# build-winpath: 4 → 8
sed -i 's|"--jobs", "4"  # Increased from 12|"--jobs", "8"  # Optimized for 22-core system|' Makefile.toml
# build-derive-parallel: first occurrence
sed -i '0,/"--jobs", "4",/{s/"--jobs", "4",/"--jobs", "12",/}' Makefile.toml
# build-core-utilities: 8 → 20
sed -i 's|"--jobs", "8",  # Increased for parallel derive builds|"--jobs", "20",  # Maximized for 22-core system|' Makefile.toml
# build-coreutils-workspace: 8 → 20
sed -i 's|"--jobs", "8",.*# Increased for parallel derive builds|"--jobs", "20",  # Maximized for 22-core system|' Makefile.toml
echo "✅ Build task job counts optimized"
echo ""

# Fix 4: Add jobs to .cargo/config.toml
echo "🔧 Fix 4/5: Adding jobs setting to Cargo config..."
if ! grep -q "^jobs = " .cargo/config.toml; then
    sed -i '/^target-dir = /a jobs = 20  # Optimized for 22-core system' .cargo/config.toml
    echo "✅ Added jobs = 20 to Cargo config"
else
    echo "⚠️  jobs setting already exists, skipping"
fi
echo ""

# Fix 5: Remove sccache bypass in Makefile
echo "🔧 Fix 5/5: Removing sccache bypass in Makefile..."
sed -i 's|@RUSTC_WRAPPER="" $(CARGO) clean|@$(CARGO) clean|g' Makefile
echo "✅ sccache bypass removed"
echo ""

echo "🎉 All optimizations applied successfully!"
echo ""
echo "📋 Changes made:"
echo "  1. Fixed sccache directory path in Makefile.toml"
echo "  2. Increased CARGO_BUILD_JOBS to 20"
echo "  3. Optimized build task job counts (8/12/20)"
echo "  4. Added jobs=20 to .cargo/config.toml"
echo "  5. Removed sccache bypass in Makefile"
echo ""
echo "🧪 Next steps:"
echo "  1. Test: make clean && make release"
echo "  2. Verify: sccache --show-stats"
echo "  3. Test cache: make clean && make release (should be faster)"
echo ""
echo "💾 Backups saved as:"
echo "  - Makefile.toml.backup"
echo "  - .cargo/config.toml.backup"
echo "  - Makefile.backup"
echo ""
echo "⏮️  To revert: mv *.backup (without .backup extension)"
