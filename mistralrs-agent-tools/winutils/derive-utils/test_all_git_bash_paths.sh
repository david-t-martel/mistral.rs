#!/bin/bash

# Git Bash Path Normalization Test Script for derive-utils
# This script comprehensively tests all derive-utils (where, which, tree) with various Git Bash path formats

set -euo pipefail

# Configuration
VERBOSE=false
BENCHMARK_ONLY=false
OUTPUT_DIR="test_results"
TIMEOUT_SECONDS=30

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -b|--benchmark)
            BENCHMARK_ONLY=true
            shift
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -t|--timeout)
            TIMEOUT_SECONDS="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -v, --verbose    Enable verbose output"
            echo "  -b, --benchmark  Run benchmark tests only"
            echo "  -o, --output     Output directory (default: test_results)"
            echo "  -t, --timeout    Timeout in seconds (default: 30)"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Utility functions
log_header() {
    echo -e "\n${PURPLE}$(printf '=%.0s' {1..80})${NC}"
    echo -e "${PURPLE} $1${NC}"
    echo -e "${PURPLE}$(printf '=%.0s' {1..80})${NC}"
}

log_info() {
    echo -e "${CYAN}$1${NC}"
}

log_success() {
    echo -e "${GREEN}$1${NC}"
}

log_warning() {
    echo -e "${YELLOW}$1${NC}"
}

log_error() {
    echo -e "${RED}$1${NC}"
}

log_test_result() {
    local test_name="$1"
    local passed="$2"
    local details="${3:-}"

    if [[ "$passed" == "true" ]]; then
        echo -e "${GREEN}[PASS]${NC} $test_name"
    else
        echo -e "${RED}[FAIL]${NC} $test_name"
    fi

    if [[ -n "$details" && ("$VERBOSE" == "true" || "$passed" == "false") ]]; then
        echo -e "  ${CYAN}$details${NC}"
    fi
}

# Initialize test environment
log_header "Git Bash Path Normalization Tests for derive-utils"
log_info "Initializing test environment..."

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if we're in Git Bash or WSL
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    log_info "Running in Git Bash environment"
    BASE_PATH="/c"
elif [[ -d "/mnt/c" ]]; then
    log_info "Running in WSL environment"
    BASE_PATH="/mnt/c"
else
    log_error "This script should be run in Git Bash or WSL on Windows"
    exit 1
fi

# Define derive-utils binary paths
declare -A BIN_PATHS=(
    ["where"]="$BASE_PATH/projects/coreutils/winutils/derive-utils/where/target/release/where.exe"
    ["which"]="$BASE_PATH/projects/coreutils/winutils/derive-utils/which/target/release/which.exe"
    ["tree"]="$BASE_PATH/projects/coreutils/winutils/derive-utils/tree/target/release/tree.exe"
)

# Check if binaries exist
missing_binaries=()
for tool in "${!BIN_PATHS[@]}"; do
    if [[ ! -f "${BIN_PATHS[$tool]}" ]]; then
        missing_binaries+=("$tool")
    fi
done

if [[ ${#missing_binaries[@]} -gt 0 ]]; then
    log_error "Missing binaries: ${missing_binaries[*]}"
    log_info "Please build the derive-utils first:"
    log_info "  cd /c/projects/coreutils/winutils/derive-utils/"
    log_info "  cargo build --release"
    exit 1
fi

# Create test directory structure
TEST_ROOT="/tmp/derive_utils_git_bash_test_$(date +%Y%m%d_%H%M%S)"
log_info "Creating test directory: $TEST_ROOT"

mkdir -p "$TEST_ROOT"

# Create complex directory structure for testing
test_dirs=(
    "Program Files (x86)/Microsoft/Application"
    "Users/Developer/Documents/Projects"
    "Windows/System32/drivers"
    "temp/build/output"
    "git-repos/project with spaces/src"
    "very/deep/nested/directory/structure/for/testing/long/paths"
    "unicode/café/中文/test"
    "bin"
    "usr/local/bin"
)

for dir in "${test_dirs[@]}"; do
    mkdir -p "$TEST_ROOT/$dir"
done

# Create test executables and files
declare -A test_files=(
    ["bin/testapp.exe"]="Test executable"
    ["bin/python.exe"]="Python executable"
    ["bin/node.exe"]="Node.js executable"
    ["usr/local/bin/gcc.exe"]="GCC compiler"
    ["Program Files (x86)/Microsoft/Application/app.exe"]="Microsoft application"
    ["git-repos/project with spaces/src/main.c"]="int main() { return 0; }"
    ["Users/Developer/Documents/Projects/readme.txt"]="Project documentation"
    ["temp/build/output/result.dll"]="Build output"
    ["unicode/café/中文/test/file.txt"]="Unicode test file"
    ["very/deep/nested/directory/structure/for/testing/long/paths/deep.exe"]="Deep executable"
)

for file in "${!test_files[@]}"; do
    mkdir -p "$(dirname "$TEST_ROOT/$file")"
    echo "${test_files[$file]}" > "$TEST_ROOT/$file"
done

log_success "Test environment created successfully"

# Define path formats to test
declare -A PATH_FORMATS=(
    ["Windows"]="C:${TEST_ROOT#/c}"
    ["Git_Bash"]="$TEST_ROOT"
    ["WSL"]="/mnt/c${TEST_ROOT#/c}"
    ["Mixed"]="C:${TEST_ROOT#/c}"  # We'll modify this to have mixed separators
)

# Fix mixed separators format
PATH_FORMATS["Mixed"]="${PATH_FORMATS["Mixed"]//\/\\//}"
PATH_FORMATS["Mixed"]="${PATH_FORMATS["Mixed"]:0:10}\\${PATH_FORMATS["Mixed"]:10}"

# Test results tracking
declare -A test_results_where=()
declare -A test_results_which=()
declare -A test_results_tree=()

# Test 1: WHERE.EXE Tests
log_header "Testing WHERE.EXE with various path formats"

for format_name in "${!PATH_FORMATS[@]}"; do
    test_path="${PATH_FORMATS[$format_name]}"
    log_info "Testing where.exe with $format_name path format..."

    # Test basic executable search
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[where]}" testapp.exe -R "$test_path" >/tmp/where_result.txt 2>&1; then
        result=$(cat /tmp/where_result.txt)
        if [[ "$result" =~ [A-Z]:\\ && "$result" =~ testapp\.exe ]]; then
            test_results_where["${format_name}_basic"]="PASS"
            log_test_result "where.exe basic search ($format_name)" "true" "$result"
        else
            test_results_where["${format_name}_basic"]="FAIL"
            log_test_result "where.exe basic search ($format_name)" "false" "Invalid output: $result"
        fi
    else
        test_results_where["${format_name}_basic"]="FAIL"
        result=$(cat /tmp/where_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "where.exe basic search ($format_name)" "false" "Command failed: $result"
    fi

    # Test wildcard search
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[where]}" "*.exe" -R "$test_path" >/tmp/where_wild_result.txt 2>&1; then
        result=$(cat /tmp/where_wild_result.txt)
        result_lines=$(echo "$result" | wc -l)
        if [[ "$result" =~ [A-Z]:\\ && $result_lines -gt 1 ]]; then
            test_results_where["${format_name}_wildcard"]="PASS"
            log_test_result "where.exe wildcard search ($format_name)" "true" "Found $result_lines matches"
        else
            test_results_where["${format_name}_wildcard"]="FAIL"
            log_test_result "where.exe wildcard search ($format_name)" "false" "Invalid output or insufficient matches"
        fi
    else
        test_results_where["${format_name}_wildcard"]="FAIL"
        result=$(cat /tmp/where_wild_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "where.exe wildcard search ($format_name)" "false" "Command failed: $result"
    fi
done

# Test 2: WHICH.EXE Tests
log_header "Testing WHICH.EXE with various path formats"

original_path="$PATH"

for format_name in "${!PATH_FORMATS[@]}"; do
    test_path="${PATH_FORMATS[$format_name]}"
    log_info "Testing which.exe with $format_name path format..."

    # Set PATH to include test directory
    export PATH="$test_path:$original_path"

    # Test basic command lookup
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[which]}" testapp.exe >/tmp/which_result.txt 2>&1; then
        result=$(cat /tmp/which_result.txt)
        if [[ "$result" =~ [A-Z]:\\ && "$result" =~ testapp\.exe ]]; then
            test_results_which["${format_name}_basic"]="PASS"
            log_test_result "which.exe basic lookup ($format_name)" "true" "$result"
        else
            test_results_which["${format_name}_basic"]="FAIL"
            log_test_result "which.exe basic lookup ($format_name)" "false" "Invalid output: $result"
        fi
    else
        test_results_which["${format_name}_basic"]="FAIL"
        result=$(cat /tmp/which_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "which.exe basic lookup ($format_name)" "false" "Command failed: $result"
    fi

    # Test --all flag
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[which]}" --all python.exe >/tmp/which_all_result.txt 2>&1; then
        result=$(cat /tmp/which_all_result.txt)
        if [[ "$result" =~ [A-Z]:\\ ]]; then
            test_results_which["${format_name}_all"]="PASS"
            log_test_result "which.exe --all flag ($format_name)" "true" "$result"
        else
            test_results_which["${format_name}_all"]="FAIL"
            log_test_result "which.exe --all flag ($format_name)" "false" "Invalid output: $result"
        fi
    else
        test_results_which["${format_name}_all"]="FAIL"
        result=$(cat /tmp/which_all_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "which.exe --all flag ($format_name)" "false" "Command failed: $result"
    fi

    # Test silent mode
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[which]}" --silent node.exe >/tmp/which_silent_result.txt 2>&1; then
        result=$(cat /tmp/which_silent_result.txt)
        if [[ -z "$result" ]]; then
            test_results_which["${format_name}_silent"]="PASS"
            log_test_result "which.exe silent mode ($format_name)" "true" "Exit code: 0, no output"
        else
            test_results_which["${format_name}_silent"]="FAIL"
            log_test_result "which.exe silent mode ($format_name)" "false" "Unexpected output: $result"
        fi
    else
        test_results_which["${format_name}_silent"]="FAIL"
        log_test_result "which.exe silent mode ($format_name)" "false" "Command failed"
    fi

    # Restore original PATH
    export PATH="$original_path"
done

# Test 3: TREE.EXE Tests
log_header "Testing TREE.EXE with various path formats"

for format_name in "${!PATH_FORMATS[@]}"; do
    test_path="${PATH_FORMATS[$format_name]}"
    log_info "Testing tree.exe with $format_name path format..."

    # Test basic directory tree
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[tree]}" "$test_path" >/tmp/tree_result.txt 2>&1; then
        result=$(cat /tmp/tree_result.txt)
        if [[ "$result" =~ (bin|usr|Program\ Files) ]]; then
            test_results_tree["${format_name}_basic"]="PASS"
            log_test_result "tree.exe basic tree ($format_name)" "true" "Shows directory structure"
        else
            test_results_tree["${format_name}_basic"]="FAIL"
            log_test_result "tree.exe basic tree ($format_name)" "false" "Missing expected directories"
        fi
    else
        test_results_tree["${format_name}_basic"]="FAIL"
        result=$(cat /tmp/tree_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "tree.exe basic tree ($format_name)" "false" "Command failed: $result"
    fi

    # Test with depth limit
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[tree]}" -L 2 "$test_path" >/tmp/tree_depth_result.txt 2>&1; then
        result=$(cat /tmp/tree_depth_result.txt)
        if [[ ! "$result" =~ very.*deep.*nested.*directory.*structure ]]; then
            test_results_tree["${format_name}_depth"]="PASS"
            log_test_result "tree.exe depth limit ($format_name)" "true" "Respects depth limit"
        else
            test_results_tree["${format_name}_depth"]="FAIL"
            log_test_result "tree.exe depth limit ($format_name)" "false" "Depth limit not respected"
        fi
    else
        test_results_tree["${format_name}_depth"]="FAIL"
        result=$(cat /tmp/tree_depth_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "tree.exe depth limit ($format_name)" "false" "Command failed: $result"
    fi

    # Test directories only
    if timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[tree]}" -d "$test_path" >/tmp/tree_dirs_result.txt 2>&1; then
        result=$(cat /tmp/tree_dirs_result.txt)
        if [[ ! "$result" =~ \.(exe|txt|c|dll) ]]; then
            test_results_tree["${format_name}_dirs"]="PASS"
            log_test_result "tree.exe directories only ($format_name)" "true" "Shows only directories"
        else
            test_results_tree["${format_name}_dirs"]="FAIL"
            log_test_result "tree.exe directories only ($format_name)" "false" "Files present in output"
        fi
    else
        test_results_tree["${format_name}_dirs"]="FAIL"
        result=$(cat /tmp/tree_dirs_result.txt 2>/dev/null || echo "Command failed")
        log_test_result "tree.exe directories only ($format_name)" "false" "Command failed: $result"
    fi
done

# Performance benchmarks (if requested)
if [[ "$BENCHMARK_ONLY" == "true" || "$VERBOSE" == "true" ]]; then
    log_header "Performance Benchmarks"

    declare -A benchmark_results=()

    for tool in where which tree; do
        log_info "Benchmarking $tool.exe..."

        for format_name in Windows Git_Bash; do
            test_path="${PATH_FORMATS[$format_name]}"
            times=()

            for i in {1..5}; do
                start_time=$(date +%s%3N)

                case "$tool" in
                    where)
                        timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[where]}" "*.exe" -R "$test_path" >/dev/null 2>&1
                        ;;
                    which)
                        export PATH="$test_path:$original_path"
                        timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[which]}" testapp.exe >/dev/null 2>&1
                        export PATH="$original_path"
                        ;;
                    tree)
                        timeout "$TIMEOUT_SECONDS" "${BIN_PATHS[tree]}" -L 3 "$test_path" >/dev/null 2>&1
                        ;;
                esac

                end_time=$(date +%s%3N)
                elapsed=$((end_time - start_time))
                times+=("$elapsed")
            done

            # Calculate average
            total=0
            for time in "${times[@]}"; do
                total=$((total + time))
            done
            avg_time=$((total / ${#times[@]}))

            benchmark_results["${tool}_${format_name}"]="$avg_time"
            log_info "  $format_name path: ${avg_time}ms average"
        done

        # Calculate overhead
        windows_key="${tool}_Windows"
        git_bash_key="${tool}_Git_Bash"

        if [[ -n "${benchmark_results[$windows_key]:-}" && -n "${benchmark_results[$git_bash_key]:-}" ]]; then
            windows_time="${benchmark_results[$windows_key]}"
            git_bash_time="${benchmark_results[$git_bash_key]}"

            if [[ "$windows_time" -gt 0 ]]; then
                overhead=$(( (git_bash_time * 100 / windows_time) - 100 ))

                if [[ "$overhead" -lt 50 ]]; then
                    log_success "  Git Bash overhead: ${overhead}%"
                elif [[ "$overhead" -lt 100 ]]; then
                    log_warning "  Git Bash overhead: ${overhead}%"
                else
                    log_error "  Git Bash overhead: ${overhead}%"
                fi
            fi
        fi
    done
fi

# Generate summary report
log_header "Test Summary Report"

total_tests=0
passed_tests=0

for tool in where which tree; do
    declare -n results="test_results_$tool"
    log_info ""
    log_info "${tool^^}.EXE Results:"

    for test in "${!results[@]}"; do
        total_tests=$((total_tests + 1))
        result="${results[$test]}"

        if [[ "$result" == "PASS" ]]; then
            passed_tests=$((passed_tests + 1))
            log_success "  ✓ $test"
        else
            log_error "  ✗ $test"
        fi
    done
done

if [[ "$total_tests" -gt 0 ]]; then
    success_rate=$(( (passed_tests * 100) / total_tests ))
else
    success_rate=0
fi

if [[ "$success_rate" -ge 90 ]]; then
    log_success "\nOverall Results: $passed_tests/$total_tests tests passed (${success_rate}%)"
elif [[ "$success_rate" -ge 70 ]]; then
    log_warning "\nOverall Results: $passed_tests/$total_tests tests passed (${success_rate}%)"
else
    log_error "\nOverall Results: $passed_tests/$total_tests tests passed (${success_rate}%)"
fi

# Save detailed results
report_path="$OUTPUT_DIR/git_bash_path_test_report_$(date +%Y%m%d_%H%M%S).json"
cat > "$report_path" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "testEnvironment": {
    "testRoot": "$TEST_ROOT",
    "binPaths": $(printf '%s\n' "${BIN_PATHS[@]}" | jq -R . | jq -s 'to_entries | map({key: ("where", "which", "tree")[.key], value: .value}) | from_entries'),
    "pathFormats": $(printf '%s\n' "${PATH_FORMATS[@]}" | jq -R . | jq -s 'to_entries | map({key: ("Windows", "Git_Bash", "WSL", "Mixed")[.key], value: .value}) | from_entries')
  },
  "results": {
    "where": $(printf '%s\n' "${test_results_where[@]}" | jq -R . | jq -s 'to_entries | map({key: (input_filename), value: .value}) | from_entries'),
    "which": $(printf '%s\n' "${test_results_which[@]}" | jq -R . | jq -s 'to_entries | map({key: (input_filename), value: .value}) | from_entries'),
    "tree": $(printf '%s\n' "${test_results_tree[@]}" | jq -R . | jq -s 'to_entries | map({key: (input_filename), value: .value}) | from_entries')
  },
  "summary": {
    "totalTests": $total_tests,
    "passedTests": $passed_tests,
    "successRate": $success_rate
  }
}
EOF

log_info "\nDetailed report saved to: $report_path"

# Cleanup recommendations
log_header "Cleanup and Recommendations"

if [[ "$success_rate" -lt 100 ]]; then
    log_warning "Some tests failed. Please check:"
    log_info "1. Ensure all derive-utils are built with latest winpath library"
    log_info "2. Verify path normalization functions are working correctly"
    log_info "3. Check Windows path handling in each utility"
else
    log_success "All tests passed! Git Bash path normalization is working correctly."
fi

log_info "\nTo run individual tests:"
log_info "  cargo test --manifest-path /c/projects/coreutils/winutils/derive-utils/where/Cargo.toml git_bash_path"
log_info "  cargo test --manifest-path /c/projects/coreutils/winutils/derive-utils/which/Cargo.toml git_bash_path"
log_info "  cargo test --manifest-path /c/projects/coreutils/winutils/derive-utils/tree/Cargo.toml git_bash_path"

# Cleanup test directory
if [[ -d "$TEST_ROOT" ]]; then
    log_info "\nCleaning up test directory..."
    rm -rf "$TEST_ROOT"
fi

# Cleanup temporary files
rm -f /tmp/where_result.txt /tmp/where_wild_result.txt
rm -f /tmp/which_result.txt /tmp/which_all_result.txt /tmp/which_silent_result.txt
rm -f /tmp/tree_result.txt /tmp/tree_depth_result.txt /tmp/tree_dirs_result.txt

log_success "\nGit Bash path normalization testing completed."
