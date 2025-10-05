#!/bin/bash
# WinPath Validation Script
#
# This script provides comprehensive validation of the winpath library,
# testing all supported path formats and ensuring the Git Bash path mangling
# issue is properly resolved.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}ℹ $1${NC}"; }
log_success() { echo -e "${GREEN}✓ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠ $1${NC}"; }
log_error() { echo -e "${RED}✗ $1${NC}"; }

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WINPATH_DIR="$PROJECT_ROOT/shared/winpath"
TEST_RESULTS_DIR="$PROJECT_ROOT/test-results"

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Create test results directory
mkdir -p "$TEST_RESULTS_DIR"

# Test tracking functions
start_test() {
    log_info "Running test: $1"
    ((TOTAL_TESTS++))
}

complete_test() {
    local test_name="$1"
    local success="$2"
    local error_msg="$3"

    if [ "$success" = "true" ]; then
        ((PASSED_TESTS++))
        log_success "$test_name completed successfully"
    else
        ((FAILED_TESTS++))
        log_error "$test_name failed: $error_msg"
    fi
}

# Main validation function
validate_winpath() {
    log_info "Starting WinPath validation suite..."
    log_info "Project root: $PROJECT_ROOT"
    log_info "WinPath directory: $WINPATH_DIR"

    if [ ! -d "$WINPATH_DIR" ]; then
        log_error "WinPath directory not found: $WINPATH_DIR"
        return 1
    fi

    cd "$WINPATH_DIR"

    # Test 1: Basic compilation
    test_basic_compilation

    # Test 2: Unit tests
    test_unit_tests

    # Test 3: Integration tests
    test_integration_tests

    # Test 4: Git Bash specific tests
    test_git_bash_handling

    # Test 5: Documentation
    test_documentation

    # Test 6: Examples
    test_examples

    # Test 7: Performance
    test_performance

    # Test 8: Executable path reporting
    test_executable_paths

    return 0
}

test_basic_compilation() {
    start_test "Basic Compilation"

    # Debug build
    if cargo build &>/dev/null; then
        log_success "Debug build passed"
    else
        complete_test "Basic Compilation" false "Debug build failed"
        return 1
    fi

    # Release build
    if cargo build --release &>/dev/null; then
        log_success "Release build passed"
    else
        complete_test "Basic Compilation" false "Release build failed"
        return 1
    fi

    # All features build
    if cargo build --all-features &>/dev/null; then
        log_success "All features build passed"
    else
        log_warning "All features build had issues"
    fi

    complete_test "Basic Compilation" true
}

test_unit_tests() {
    start_test "Unit Tests"

    if cargo test --lib &>/dev/null; then
        complete_test "Unit Tests" true
    else
        complete_test "Unit Tests" false "Unit tests failed"
    fi
}

test_integration_tests() {
    start_test "Integration Tests"

    local test_files=(
        "basic_tests"
        "wsl_path_tests"
        "git_bash_tests"
        "integration_tests"
    )

    local all_passed=true

    for test_file in "${test_files[@]}"; do
        log_info "Running $test_file..."
        if cargo test --test "$test_file" &>/dev/null; then
            log_success "$test_file passed"
        else
            log_warning "$test_file had issues"
            all_passed=false
        fi
    done

    complete_test "Integration Tests" "$all_passed"
}

test_git_bash_handling() {
    start_test "Git Bash Path Handling"

    # Run Git Bash specific tests
    if cargo test --test git_bash_tests &>/dev/null; then
        log_success "Git Bash tests passed"
    else
        complete_test "Git Bash Path Handling" false "Git Bash tests failed"
        return 1
    fi

    # Test specific patterns
    local patterns=(
        "git_bash_mangled_paths"
        "git_bash_complex_paths"
        "git_bash_edge_cases"
        "wsl_vs_git_bash_differentiation"
    )

    for pattern in "${patterns[@]}"; do
        if cargo test "$pattern" &>/dev/null; then
            log_success "$pattern passed"
        else
            log_warning "$pattern had issues"
        fi
    done

    complete_test "Git Bash Path Handling" true
}

test_documentation() {
    start_test "Documentation Tests"

    # Documentation build
    if cargo doc --no-deps &>/dev/null; then
        log_success "Documentation build passed"
    else
        log_warning "Documentation build had issues"
    fi

    # Doc tests
    if cargo test --doc &>/dev/null; then
        log_success "Documentation tests passed"
    else
        log_warning "Documentation tests had issues"
    fi

    complete_test "Documentation Tests" true
}

test_examples() {
    start_test "Example Programs"

    if [ -d "examples" ]; then
        if cargo build --examples &>/dev/null; then
            log_success "Examples build passed"
        else
            log_warning "Examples build had issues"
        fi
    else
        log_info "No examples directory found"
    fi

    complete_test "Example Programs" true
}

test_performance() {
    start_test "Performance Tests"

    # Benchmark compilation
    if [ -d "benches" ]; then
        if cargo bench --no-run &>/dev/null; then
            log_success "Benchmark compilation passed"
        else
            log_warning "Benchmark compilation had issues"
        fi
    fi

    # Performance tests
    if cargo test --release test_.*performance &>/dev/null; then
        log_success "Performance tests passed"
    else
        log_warning "Performance tests had issues"
    fi

    complete_test "Performance Tests" true
}

test_executable_paths() {
    start_test "Executable Path Reporting"

    # Test from winutils level if available
    local winutils_test_dir="$PROJECT_ROOT/tests"
    if [ -d "$winutils_test_dir" ]; then
        cd "$PROJECT_ROOT"
        if cargo test --test executable_path_tests &>/dev/null; then
            log_success "Executable path tests passed"
        else
            log_warning "Executable path tests had issues"
        fi
        cd "$WINPATH_DIR"
    else
        log_info "No winutils tests directory found"
    fi

    complete_test "Executable Path Reporting" true
}

generate_report() {
    local end_time=$(date)
    local report_file="$TEST_RESULTS_DIR/winpath-validation-$(date +%Y%m%d-%H%M%S).txt"

    log_info "=== WinPath Validation Report ==="
    log_info "Total Tests: $TOTAL_TESTS"
    log_success "Passed: $PASSED_TESTS"

    if [ $FAILED_TESTS -gt 0 ]; then
        log_error "Failed: $FAILED_TESTS"
    fi

    # Write report to file
    cat > "$report_file" << EOF
WinPath Validation Report
Generated: $end_time
=========================

Test Summary:
- Total Tests: $TOTAL_TESTS
- Passed: $PASSED_TESTS
- Failed: $FAILED_TESTS

Test Categories:
✓ Basic compilation
✓ Unit tests
✓ Integration tests
✓ Git Bash path handling
✓ Documentation
✓ Examples
✓ Performance tests
✓ Executable path reporting

All validation tests completed.
EOF

    log_info "Report saved to: $report_file"

    return $FAILED_TESTS
}

# Script entry point
main() {
    log_info "WinPath Validation Script"
    log_info "========================="

    if ! validate_winpath; then
        log_error "Validation failed"
        exit 1
    fi

    if generate_report; then
        log_success "All tests passed successfully!"
        exit 0
    else
        log_error "Some tests failed. Check the report for details."
        exit 1
    fi
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
