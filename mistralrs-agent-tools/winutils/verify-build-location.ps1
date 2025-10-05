#!/usr/bin/env pwsh
# verify-build-location.ps1
# Build Verification Script for WinUtils Workspace
# =================================================
# This script verifies that ALL binaries are in the correct location
# and NONE have leaked to the global cargo bin directory

param(
    [switch]$Verbose,
    [switch]$Fix
)

$ErrorActionPreference = "Stop"

# ANSI Color Codes
$GREEN = "`e[32m"
$RED = "`e[31m"
$YELLOW = "`e[33m"
$BLUE = "`e[34m"
$RESET = "`e[0m"

function Write-Success { param($msg) Write-Host "${GREEN}✓${RESET} $msg" }
function Write-Error { param($msg) Write-Host "${RED}✗${RESET} $msg" }
function Write-Warning { param($msg) Write-Host "${YELLOW}⚠${RESET} $msg" }
function Write-Info { param($msg) Write-Host "${BLUE}ℹ${RESET} $msg" }

# Expected workspace root
$WORKSPACE_ROOT = "T:\projects\coreutils\winutils"
$TARGET_DIR = Join-Path $WORKSPACE_ROOT "target"
$RELEASE_DIR = Join-Path $TARGET_DIR "release"
$DEBUG_DIR = Join-Path $TARGET_DIR "debug"
$CARGO_BIN = Join-Path $env:USERPROFILE ".cargo\bin"

Write-Info "WinUtils Build Location Verification"
Write-Info "======================================"
Write-Host ""

# Check 1: Verify workspace root exists
Write-Info "Checking workspace root: $WORKSPACE_ROOT"
if (-not (Test-Path $WORKSPACE_ROOT)) {
    Write-Error "Workspace root not found!"
    exit 1
}
Write-Success "Workspace root exists"

# Check 2: Verify .cargo/config.toml exists
$CARGO_CONFIG = Join-Path $WORKSPACE_ROOT ".cargo\config.toml"
Write-Info "Checking .cargo/config.toml"
if (-not (Test-Path $CARGO_CONFIG)) {
    Write-Error ".cargo/config.toml not found! Run cargo build first."
    exit 1
}
Write-Success ".cargo/config.toml exists"

# Check 3: Verify target-dir setting in config
Write-Info "Verifying target-dir configuration"
$configContent = Get-Content $CARGO_CONFIG -Raw
if ($configContent -match 'target-dir\s*=\s*"target"') {
    Write-Success "target-dir is correctly set to 'target'"
} else {
    Write-Error "target-dir is NOT set correctly in .cargo/config.toml"
    exit 1
}

# Check 4: Count binaries in target/release
Write-Info "Counting binaries in target/release/"
if (Test-Path $RELEASE_DIR) {
    $releaseBinaries = Get-ChildItem -Path $RELEASE_DIR -Filter "*.exe" -File -ErrorAction SilentlyContinue
    $releaseCount = $releaseBinaries.Count
    Write-Info "Found $releaseCount binaries in target/release/"

    if ($Verbose -and $releaseCount -gt 0) {
        Write-Host ""
        Write-Info "Release binaries:"
        $releaseBinaries | ForEach-Object {
            $size = [math]::Round($_.Length / 1MB, 2)
            Write-Host "  - $($_.Name) (${size}MB)"
        }
        Write-Host ""
    }

    if ($releaseCount -eq 0) {
        Write-Warning "No binaries found in target/release/ (run 'cargo build --release' first)"
    } elseif ($releaseCount -ne 93) {
        Write-Warning "Expected 93 binaries, found $releaseCount (some may not have compiled)"
    } else {
        Write-Success "All 93 binaries present in target/release/"
    }
} else {
    Write-Warning "target/release/ directory not found (run 'cargo build --release' first)"
    $releaseCount = 0
}

# Check 5: Count binaries in target/debug
Write-Info "Counting binaries in target/debug/"
if (Test-Path $DEBUG_DIR) {
    $debugBinaries = Get-ChildItem -Path $DEBUG_DIR -Filter "*.exe" -File -ErrorAction SilentlyContinue
    $debugCount = $debugBinaries.Count
    Write-Info "Found $debugCount binaries in target/debug/"

    if ($Verbose -and $debugCount -gt 0) {
        Write-Host ""
        Write-Info "Debug binaries:"
        $debugBinaries | Select-Object -First 10 | ForEach-Object {
            $size = [math]::Round($_.Length / 1MB, 2)
            Write-Host "  - $($_.Name) (${size}MB)"
        }
        if ($debugCount -gt 10) {
            Write-Host "  ... and $($debugCount - 10) more"
        }
        Write-Host ""
    }
} else {
    Write-Warning "target/debug/ directory not found (run 'cargo build' first)"
    $debugCount = 0
}

# Check 6: CRITICAL - Check for leaked binaries in ~/.cargo/bin
Write-Info "Checking for leaked binaries in ~/.cargo/bin/"
$leakedBinaries = @()

# List of binary names that should NEVER be in ~/.cargo/bin
$expectedBinaries = @(
    "where.exe", "which.exe", "tree.exe",
    "find-wrapper.exe", "grep-wrapper.exe",
    "cmd-wrapper.exe", "pwsh-wrapper.exe", "bash-wrapper.exe",
    "tac.exe", "cksum.exe", "numfmt.exe", "date.exe", "cut.exe",
    "true.exe", "unlink.exe", "dircolors.exe", "tr.exe", "seq.exe",
    "sync.exe", "rmdir.exe", "du.exe", "vdir.exe", "dd.exe",
    "uniq.exe", "yes.exe", "sort.exe", "cat.exe", "ptx.exe",
    "base64.exe", "realpath.exe", "rm.exe", "nl.exe", "shuf.exe",
    "mkdir.exe", "split.exe", "more.exe", "echo.exe", "shred.exe",
    "readlink.exe", "ln.exe", "env.exe", "fold.exe", "hashsum.exe",
    "truncate.exe", "printf.exe", "base32.exe", "head.exe", "fmt.exe",
    "od.exe", "test.exe", "hostname.exe", "link.exe", "df.exe",
    "false.exe", "csplit.exe", "whoami.exe", "pwd.exe", "comm.exe",
    "dir.exe", "basename.exe", "mv.exe", "factor.exe", "nproc.exe",
    "printenv.exe", "tsort.exe", "unexpand.exe", "sleep.exe", "tail.exe",
    "basenc.exe", "join.exe", "arch.exe", "mktemp.exe", "wc.exe",
    "dirname.exe", "expr.exe", "paste.exe", "sum.exe", "cp.exe",
    "expand.exe", "tee.exe", "touch.exe", "pr.exe", "ls.exe"
)

if (Test-Path $CARGO_BIN) {
    foreach ($binary in $expectedBinaries) {
        $binPath = Join-Path $CARGO_BIN $binary
        if (Test-Path $binPath) {
            $leakedBinaries += $binPath
        }
    }
}

if ($leakedBinaries.Count -eq 0) {
    Write-Success "NO leaked binaries found in ~/.cargo/bin/ (PASS)"
} else {
    Write-Error "Found $($leakedBinaries.Count) LEAKED binaries in ~/.cargo/bin/ (FAIL)"
    Write-Host ""
    Write-Warning "Leaked binaries:"
    $leakedBinaries | ForEach-Object {
        Write-Host "  - $_"
    }
    Write-Host ""

    if ($Fix) {
        Write-Info "Removing leaked binaries..."
        foreach ($binary in $leakedBinaries) {
            Remove-Item $binary -Force
            Write-Success "Removed: $binary"
        }
        Write-Success "All leaked binaries removed"
    } else {
        Write-Warning "Run with -Fix flag to automatically remove leaked binaries"
    }

    exit 1
}

# Check 7: Verify sccache is being used
Write-Info "Checking sccache usage"
$sccacheStats = sccache --show-stats 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Success "sccache is available and working"
    if ($Verbose) {
        Write-Host ""
        Write-Info "sccache statistics:"
        Write-Host ($sccacheStats -join "`n")
        Write-Host ""
    }
} else {
    Write-Warning "sccache is not available or not working"
}

# Summary
Write-Host ""
Write-Info "======================================"
Write-Info "Verification Summary"
Write-Info "======================================"
Write-Success "Workspace root: $WORKSPACE_ROOT"
Write-Success "Target directory: $TARGET_DIR"
Write-Info "Release binaries: $releaseCount"
Write-Info "Debug binaries: $debugCount"

if ($leakedBinaries.Count -eq 0) {
    Write-Success "NO LEAKED BINARIES - All binaries are in correct location"
    Write-Host ""
    Write-Success "BUILD LOCATION VERIFICATION PASSED ✓"
    exit 0
} else {
    Write-Error "LEAKED BINARIES DETECTED - Build location verification FAILED"
    exit 1
}
