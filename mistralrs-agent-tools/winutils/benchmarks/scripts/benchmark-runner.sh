#!/bin/bash
# WinUtils Benchmark Runner Script for CI/CD

set -euo pipefail

# Configuration
OUTPUT_DIR="${OUTPUT_DIR:-benchmarks/reports}"
CONFIG_FILE="${CONFIG_FILE:-benchmarks/config/default.toml}"
MEMORY_PROFILE="${MEMORY_PROFILE:-false}"
COMPARE_NATIVE="${COMPARE_NATIVE:-false}"
BASELINE="${BASELINE:-false}"
FILTER="${FILTER:-}"
CI="${CI:-false}"
REPORT_FORMAT="${REPORT_FORMAT:-html}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_status() {
    echo -e "${BLUE}🔧 $1${NC}"
}

# Main execution
main() {
    log_status "Starting WinUtils Benchmark Suite"

    # Check prerequisites
    check_prerequisites

    # Validate environment
    validate_environment

    # Build benchmark runner if needed
    build_benchmark_runner

    # Run benchmarks
    run_benchmarks

    # Generate reports
    generate_reports

    # Check for regressions in CI
    if [[ "${CI}" == "true" && "${BASELINE}" == "true" ]]; then
        check_regressions
    fi

    log_success "Benchmark suite completed successfully!"
}

check_prerequisites() {
    log_status "Checking prerequisites..."

    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust toolchain."
        exit 1
    fi

    if ! command -v git &> /dev/null; then
        log_warning "Git not found. Version information may be unavailable."
    fi

    local winutils_dir="T:/projects/coreutils/winutils"
    if [[ ! -d "$winutils_dir" ]]; then
        log_error "WinUtils directory not found at $winutils_dir"
        exit 1
    fi

    log_success "Prerequisites check completed"
}

validate_environment() {
    log_status "Validating benchmark environment..."

    cd "T:/projects/coreutils/winutils"

    if [[ -f "target/release/benchmark-runner.exe" ]]; then
        ./target/release/benchmark-runner.exe validate || {
            log_warning "Environment validation failed, but continuing..."
        }
    else
        log_info "Benchmark runner not found, will build it"
    fi
}

build_benchmark_runner() {
    local winutils_dir="T:/projects/coreutils/winutils"
    local benchmark_binary="$winutils_dir/target/release/benchmark-runner.exe"

    if [[ ! -f "$benchmark_binary" ]]; then
        log_status "Building benchmark runner..."

        cd "$winutils_dir/benchmarks"
        export CARGO_TARGET_DIR="$winutils_dir/target"

        if ! cargo build --release --bin benchmark-runner; then
            log_error "Failed to build benchmark runner"
            exit 1
        fi

        log_success "Benchmark runner built successfully"
    else
        log_info "Benchmark runner already available"
    fi
}

run_benchmarks() {
    log_status "Running benchmarks..."

    local winutils_dir="T:/projects/coreutils/winutils"
    local benchmark_binary="$winutils_dir/target/release/benchmark-runner.exe"

    cd "$winutils_dir"

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Build command arguments
    local args=(
        "run"
        "--config" "$CONFIG_FILE"
        "--output" "$OUTPUT_DIR"
    )

    if [[ "$MEMORY_PROFILE" == "true" ]]; then
        args+=("--memory-profile")
        log_info "Memory profiling enabled"
    fi

    if [[ "$COMPARE_NATIVE" == "true" ]]; then
        args+=("--compare-native")
        log_info "Native utility comparison enabled"
    fi

    if [[ "$BASELINE" == "true" ]]; then
        args+=("--baseline")
        log_info "Baseline mode enabled"
    fi

    if [[ -n "$FILTER" ]]; then
        args+=("--filter" "$FILTER")
        log_info "Filter applied: $FILTER"
    fi

    # Set CI environment
    if [[ "$CI" == "true" ]]; then
        export CI=true
        export RUST_BACKTRACE=1
        log_info "CI mode enabled"
    fi

    log_info "Command: $benchmark_binary ${args[*]}"

    local start_time=$(date +%s)

    if ! "$benchmark_binary" "${args[@]}"; then
        log_error "Benchmark execution failed"
        exit 1
    fi

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    local duration_min=$((duration / 60))

    log_success "Benchmarks completed in ${duration_min} minutes"
}

generate_reports() {
    log_status "Generating reports..."

    local winutils_dir="T:/projects/coreutils/winutils"
    local benchmark_binary="$winutils_dir/target/release/benchmark-runner.exe"

    cd "$winutils_dir"

    # Generate HTML report (default)
    "$benchmark_binary" report --input "$OUTPUT_DIR" --format html

    # Generate additional formats for CI
    if [[ "$CI" == "true" ]]; then
        "$benchmark_binary" report --input "$OUTPUT_DIR" --format json
        "$benchmark_binary" report --input "$OUTPUT_DIR" --format markdown
    fi

    log_success "Reports generated successfully"

    # List generated reports
    log_info "Generated reports:"
    find "$OUTPUT_DIR" -name "*.html" -o -name "*.json" -o -name "*.md" | while read -r file; do
        echo "  📄 $(basename "$file"): $file"
    done
}

check_regressions() {
    log_status "Checking for performance regressions..."

    local winutils_dir="T:/projects/coreutils/winutils"
    local benchmark_binary="$winutils_dir/target/release/benchmark-runner.exe"
    local baseline_file="$OUTPUT_DIR/baseline.json"
    local results_file="$OUTPUT_DIR/results.json"

    cd "$winutils_dir"

    if [[ -f "$baseline_file" ]]; then
        if ! "$benchmark_binary" compare --baseline "$baseline_file" --current "$results_file" --threshold 5.0; then
            log_error "Performance regression detected!"
            exit 1
        else
            log_success "No performance regressions detected"
        fi
    else
        log_info "No baseline file found, saving current results as baseline"
        cp "$results_file" "$baseline_file"
    fi
}

# Run main function
main "$@"
