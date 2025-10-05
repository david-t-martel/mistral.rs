#!/usr/bin/env bash
# ============================================================================
# Pre-Commit Hook - Comprehensive Quality Gate for WinUtils
# ============================================================================
#
# This hook runs automatically before every git commit to ensure code quality,
# functionality, and maintainability standards.
#
# Installation: make install-hooks
# Bypass (emergency only): git commit --no-verify
#
# ============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
MIN_COVERAGE=70
COVERAGE_TARGET=85
HOOK_START_TIME=$(date +%s)

# Helper functions
print_header() {
    echo -e "\n${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${CYAN}  $1${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_step() {
    echo -e "${BOLD}${GREEN}▶${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

elapsed_time() {
    local end_time=$(date +%s)
    local elapsed=$((end_time - HOOK_START_TIME))
    echo "${elapsed}s"
}

# Main header
clear
print_header "PRE-COMMIT QUALITY GATE - WinUtils Project"
print_info "Starting comprehensive validation pipeline..."
print_info "Minimum coverage: ${MIN_COVERAGE}% | Target: ${COVERAGE_TARGET}%"
echo ""

# ============================================================================
# Phase 1: Code Formatting
# ============================================================================
print_header "Phase 1: Code Formatting (rustfmt)"
print_step "Running cargo fmt --check..."

if ! make fmt-check >/dev/null 2>&1; then
    print_error "Code formatting check failed!"
    print_info "Running auto-format with 'cargo fmt'..."

    if make fmt; then
        print_success "Code auto-formatted successfully"
        print_warning "Formatted files have been staged. Please review changes."

        # Stage formatted files
        git add -u
    else
        print_error "Auto-formatting failed. Manual intervention required."
        exit 1
    fi
else
    print_success "Code formatting check passed"
fi

# ============================================================================
# Phase 2: Linting (Clippy)
# ============================================================================
print_header "Phase 2: Linting (clippy)"
print_step "Running cargo clippy with strict warnings..."

if ! make clippy 2>&1 | tee /tmp/clippy-output.log; then
    print_error "Clippy linting failed!"
    print_info "Review the output above for specific issues."

    # Show summary of issues
    if [ -f /tmp/clippy-output.log ]; then
        WARNING_COUNT=$(grep -c "warning:" /tmp/clippy-output.log || echo "0")
        ERROR_COUNT=$(grep -c "error:" /tmp/clippy-output.log || echo "0")

        print_error "Found ${ERROR_COUNT} errors and ${WARNING_COUNT} warnings"
        print_info "Run 'make clippy' manually to see details"
    fi

    exit 1
else
    print_success "Clippy linting passed (no warnings or errors)"
fi

# ============================================================================
# Phase 3: Functional Tests
# ============================================================================
print_header "Phase 3: Functional Tests"
print_step "Running test suite (make test-functional)..."

TEST_START=$(date +%s)

if ! make test-functional 2>&1 | tee /tmp/test-output.log; then
    print_error "Functional tests failed!"

    # Parse test results
    if [ -f /tmp/test-output.log ]; then
        FAILED_TESTS=$(grep "FAILED" /tmp/test-output.log || echo "")

        if [ -n "$FAILED_TESTS" ]; then
            print_error "Failed tests:"
            echo "$FAILED_TESTS" | head -10
        fi
    fi

    print_info "Fix failing tests before committing"
    exit 1
else
    TEST_END=$(date +%s)
    TEST_DURATION=$((TEST_END - TEST_START))

    print_success "All functional tests passed (${TEST_DURATION}s)"
fi

# ============================================================================
# Phase 4: Code Coverage
# ============================================================================
print_header "Phase 4: Code Coverage Analysis"
print_step "Running coverage collection (cargo llvm-cov)..."

COVERAGE_START=$(date +%s)

# Run coverage with warning-only threshold
if make coverage-check 2>&1 | tee /tmp/coverage-output.log; then
    # Extract coverage percentage
    if [ -f /tmp/coverage-output.log ]; then
        COVERAGE=$(grep -oP 'lines\.\.\.\.\.\. \K[0-9.]+(?=%)' /tmp/coverage-output.log | head -1 || echo "0")

        if [ -z "$COVERAGE" ]; then
            COVERAGE="0"
        fi

        COVERAGE_END=$(date +%s)
        COVERAGE_DURATION=$((COVERAGE_END - COVERAGE_START))

        # Compare coverage
        if (( $(echo "$COVERAGE < $MIN_COVERAGE" | bc -l) )); then
            print_warning "Coverage is ${COVERAGE}% (below minimum ${MIN_COVERAGE}%)"
            print_warning "Please add tests to improve coverage"
            print_info "Coverage check is warning-only and won't block commit"
        elif (( $(echo "$COVERAGE < $COVERAGE_TARGET" | bc -l) )); then
            print_success "Coverage is ${COVERAGE}% (above minimum, below target ${COVERAGE_TARGET}%)"
            print_info "Consider adding more tests to reach ${COVERAGE_TARGET}% target"
        else
            print_success "Coverage is ${COVERAGE}% (meets target ${COVERAGE_TARGET}%!) 🎉"
        fi

        print_info "HTML report: target/coverage/html/index.html"
        print_info "Duration: ${COVERAGE_DURATION}s"
    else
        print_warning "Could not parse coverage results"
    fi
else
    print_warning "Coverage collection had issues but continuing..."
    print_info "Review coverage manually with 'make coverage-html'"
fi

# ============================================================================
# Phase 5: Utility Validation
# ============================================================================
print_header "Phase 5: Utility Validation"
print_step "Validating all 77 utilities (make validate-all-77)..."

VALIDATE_START=$(date +%s)

if ! make validate-all-77 2>&1 | tee /tmp/validate-output.log; then
    print_error "Utility validation failed!"

    # Check which utilities failed
    if [ -f /tmp/validate-output.log ]; then
        FAILED_UTILS=$(grep -E "(FAILED|ERROR)" /tmp/validate-output.log || echo "")

        if [ -n "$FAILED_UTILS" ]; then
            print_error "Failed utilities:"
            echo "$FAILED_UTILS" | head -10
        fi
    fi

    print_info "All utilities must be functional before commit"
    exit 1
else
    VALIDATE_END=$(date +%s)
    VALIDATE_DURATION=$((VALIDATE_END - VALIDATE_START))

    print_success "All 77 utilities validated successfully (${VALIDATE_DURATION}s)"
fi

# ============================================================================
# Phase 6: Security Audit (Optional - Fast Check)
# ============================================================================
print_header "Phase 6: Security Audit (Quick Check)"
print_step "Running cargo audit (dependency vulnerabilities)..."

if command -v cargo-audit &> /dev/null; then
    if ! cargo audit 2>&1 | tee /tmp/audit-output.log; then
        print_warning "Security vulnerabilities detected!"

        # Count vulnerabilities
        if [ -f /tmp/audit-output.log ]; then
            VULN_COUNT=$(grep -c "Vulnerability" /tmp/audit-output.log || echo "0")

            if [ "$VULN_COUNT" -gt 0 ]; then
                print_warning "Found ${VULN_COUNT} vulnerabilities"
                print_info "Review with 'cargo audit' and update dependencies"
                print_info "This is a warning only and won't block commit"
            fi
        fi
    else
        print_success "No known security vulnerabilities"
    fi
else
    print_info "cargo-audit not installed (install with: cargo install cargo-audit)"
    print_info "Skipping security check"
fi

# ============================================================================
# Final Summary
# ============================================================================
print_header "PRE-COMMIT VALIDATION COMPLETE"

TOTAL_TIME=$(elapsed_time)

echo ""
print_success "All required checks passed!"
print_info "Total validation time: ${TOTAL_TIME}"
echo ""

# Show summary table
echo -e "${BOLD}Summary:${NC}"
echo -e "  ${GREEN}✓${NC} Code formatting     : PASSED"
echo -e "  ${GREEN}✓${NC} Clippy linting      : PASSED"
echo -e "  ${GREEN}✓${NC} Functional tests    : PASSED"
echo -e "  ${YELLOW}⚠${NC} Coverage analysis   : COMPLETED (${COVERAGE:-Unknown}%)"
echo -e "  ${GREEN}✓${NC} Utility validation  : PASSED (77/77)"
echo ""

print_info "Proceeding with commit..."
echo ""

# Cleanup temporary files
rm -f /tmp/clippy-output.log /tmp/test-output.log /tmp/coverage-output.log /tmp/validate-output.log /tmp/audit-output.log

exit 0
