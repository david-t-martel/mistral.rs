#!/usr/bin/env bash
# verify-build-location.sh
# Build Verification Script for WinUtils Workspace (Bash version)
# ================================================================
# This script verifies that ALL binaries are in the correct location
# and NONE have leaked to the global cargo bin directory

set -euo pipefail

# ANSI Color Codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

VERBOSE=false
FIX=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --fix|-f)
            FIX=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--verbose] [--fix]"
            exit 1
            ;;
    esac
done

write_success() { echo -e "${GREEN}✓${RESET} $1"; }
write_error() { echo -e "${RED}✗${RESET} $1"; }
write_warning() { echo -e "${YELLOW}⚠${RESET} $1"; }
write_info() { echo -e "${BLUE}ℹ${RESET} $1"; }

# Expected workspace root
WORKSPACE_ROOT="/mnt/t/projects/coreutils/winutils"
TARGET_DIR="$WORKSPACE_ROOT/target"
RELEASE_DIR="$TARGET_DIR/release"
DEBUG_DIR="$TARGET_DIR/debug"
CARGO_BIN="$HOME/.cargo/bin"

write_info "WinUtils Build Location Verification"
write_info "======================================"
echo ""

# Check 1: Verify workspace root exists
write_info "Checking workspace root: $WORKSPACE_ROOT"
if [[ ! -d "$WORKSPACE_ROOT" ]]; then
    write_error "Workspace root not found!"
    exit 1
fi
write_success "Workspace root exists"

# Check 2: Verify .cargo/config.toml exists
CARGO_CONFIG="$WORKSPACE_ROOT/.cargo/config.toml"
write_info "Checking .cargo/config.toml"
if [[ ! -f "$CARGO_CONFIG" ]]; then
    write_error ".cargo/config.toml not found! Run cargo build first."
    exit 1
fi
write_success ".cargo/config.toml exists"

# Check 3: Verify target-dir setting in config
write_info "Verifying target-dir configuration"
if grep -q 'target-dir.*=.*"target"' "$CARGO_CONFIG"; then
    write_success "target-dir is correctly set to 'target'"
else
    write_error "target-dir is NOT set correctly in .cargo/config.toml"
    exit 1
fi

# Check 4: Count binaries in target/release
write_info "Counting binaries in target/release/"
RELEASE_COUNT=0
if [[ -d "$RELEASE_DIR" ]]; then
    RELEASE_COUNT=$(/usr/bin/find "$RELEASE_DIR" -maxdepth 1 -name "*.exe" -type f 2>/dev/null | wc -l)
    write_info "Found $RELEASE_COUNT binaries in target/release/"

    if [[ "$VERBOSE" == "true" && $RELEASE_COUNT -gt 0 ]]; then
        echo ""
        write_info "Release binaries:"
        /usr/bin/find "$RELEASE_DIR" -maxdepth 1 -name "*.exe" -type f | head -20 | sed 's|.*/||; s/^/  - /'
        if [[ $RELEASE_COUNT -gt 20 ]]; then
            echo "  ... and $((RELEASE_COUNT - 20)) more"
        fi
        echo ""
    fi

    if [[ $RELEASE_COUNT -eq 0 ]]; then
        write_warning "No binaries found in target/release/ (run 'cargo build --release' first)"
    elif [[ $RELEASE_COUNT -ne 93 ]]; then
        write_warning "Expected 93 binaries, found $RELEASE_COUNT (some may not have compiled)"
    else
        write_success "All 93 binaries present in target/release/"
    fi
else
    write_warning "target/release/ directory not found (run 'cargo build --release' first)"
fi

# Check 5: Count binaries in target/debug
write_info "Counting binaries in target/debug/"
DEBUG_COUNT=0
if [[ -d "$DEBUG_DIR" ]]; then
    DEBUG_COUNT=$(/usr/bin/find "$DEBUG_DIR" -maxdepth 1 -name "*.exe" -type f 2>/dev/null | wc -l)
    write_info "Found $DEBUG_COUNT binaries in target/debug/"

    if [[ "$VERBOSE" == "true" && $DEBUG_COUNT -gt 0 ]]; then
        echo ""
        write_info "Debug binaries (first 10):"
        /usr/bin/find "$DEBUG_DIR" -maxdepth 1 -name "*.exe" -type f | head -10 | sed 's|.*/||; s/^/  - /'
        if [[ $DEBUG_COUNT -gt 10 ]]; then
            echo "  ... and $((DEBUG_COUNT - 10)) more"
        fi
        echo ""
    fi
else
    write_warning "target/debug/ directory not found (run 'cargo build' first)"
fi

# Check 6: CRITICAL - Check for leaked binaries in ~/.cargo/bin
write_info "Checking for leaked binaries in ~/.cargo/bin/"

# Array of binary names that should NEVER be in ~/.cargo/bin
EXPECTED_BINARIES=(
    "where.exe" "which.exe" "tree.exe"
    "find-wrapper.exe" "grep-wrapper.exe"
    "cmd-wrapper.exe" "pwsh-wrapper.exe" "bash-wrapper.exe"
    "tac.exe" "cksum.exe" "numfmt.exe" "date.exe" "cut.exe"
    "true.exe" "unlink.exe" "dircolors.exe" "tr.exe" "seq.exe"
    "sync.exe" "rmdir.exe" "du.exe" "vdir.exe" "dd.exe"
    "uniq.exe" "yes.exe" "sort.exe" "cat.exe" "ptx.exe"
    "base64.exe" "realpath.exe" "rm.exe" "nl.exe" "shuf.exe"
    "mkdir.exe" "split.exe" "more.exe" "echo.exe" "shred.exe"
    "readlink.exe" "ln.exe" "env.exe" "fold.exe" "hashsum.exe"
    "truncate.exe" "printf.exe" "base32.exe" "head.exe" "fmt.exe"
    "od.exe" "test.exe" "hostname.exe" "link.exe" "df.exe"
    "false.exe" "csplit.exe" "whoami.exe" "pwd.exe" "comm.exe"
    "dir.exe" "basename.exe" "mv.exe" "factor.exe" "nproc.exe"
    "printenv.exe" "tsort.exe" "unexpand.exe" "sleep.exe" "tail.exe"
    "basenc.exe" "join.exe" "arch.exe" "mktemp.exe" "wc.exe"
    "dirname.exe" "expr.exe" "paste.exe" "sum.exe" "cp.exe"
    "expand.exe" "tee.exe" "touch.exe" "pr.exe" "ls.exe"
)

LEAKED_BINARIES=()
if [[ -d "$CARGO_BIN" ]]; then
    for binary in "${EXPECTED_BINARIES[@]}"; do
        if [[ -f "$CARGO_BIN/$binary" ]]; then
            LEAKED_BINARIES+=("$CARGO_BIN/$binary")
        fi
    done
fi

if [[ ${#LEAKED_BINARIES[@]} -eq 0 ]]; then
    write_success "NO leaked binaries found in ~/.cargo/bin/ (PASS)"
else
    write_error "Found ${#LEAKED_BINARIES[@]} LEAKED binaries in ~/.cargo/bin/ (FAIL)"
    echo ""
    write_warning "Leaked binaries:"
    for binary in "${LEAKED_BINARIES[@]}"; do
        echo "  - $binary"
    done
    echo ""

    if [[ "$FIX" == "true" ]]; then
        write_info "Removing leaked binaries..."
        for binary in "${LEAKED_BINARIES[@]}"; do
            /usr/bin/rm -f "$binary"
            write_success "Removed: $binary"
        done
        write_success "All leaked binaries removed"
    else
        write_warning "Run with --fix flag to automatically remove leaked binaries"
    fi

    exit 1
fi

# Check 7: Verify sccache is being used
write_info "Checking sccache usage"
if command -v sccache &>/dev/null; then
    if sccache --show-stats &>/dev/null; then
        write_success "sccache is available and working"
        if [[ "$VERBOSE" == "true" ]]; then
            echo ""
            write_info "sccache statistics:"
            sccache --show-stats
            echo ""
        fi
    else
        write_warning "sccache is installed but not working properly"
    fi
else
    write_warning "sccache is not available"
fi

# Summary
echo ""
write_info "======================================"
write_info "Verification Summary"
write_info "======================================"
write_success "Workspace root: $WORKSPACE_ROOT"
write_success "Target directory: $TARGET_DIR"
write_info "Release binaries: $RELEASE_COUNT"
write_info "Debug binaries: $DEBUG_COUNT"

if [[ ${#LEAKED_BINARIES[@]} -eq 0 ]]; then
    write_success "NO LEAKED BINARIES - All binaries are in correct location"
    echo ""
    write_success "BUILD LOCATION VERIFICATION PASSED ✓"
    exit 0
else
    write_error "LEAKED BINARIES DETECTED - Build location verification FAILED"
    exit 1
fi
