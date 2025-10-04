#!/bin/bash
# Health check script for mistral.rs server
# Tests: HTTP endpoint, MCP connectivity, model loading, resource usage

set -euo pipefail

# Configuration
HOST="${MISTRALRS_HOST:-localhost}"
PORT="${MISTRALRS_PORT:-8080}"
TIMEOUT=10
VERBOSE="${VERBOSE:-0}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Exit codes (only those actually used retained to satisfy shellcheck)
EXIT_HTTP_FAILED=1
EXIT_HEALTH_FAILED=2
EXIT_RESOURCE_FAILED=4

# ============================================================================
# Helper Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# ============================================================================
# Health Checks
# ============================================================================

check_http_connectivity() {
    log_info "Checking HTTP connectivity..."

    if curl -f -s --max-time "$TIMEOUT" "http://${HOST}:${PORT}/health" > /dev/null 2>&1; then
        log_success "HTTP server is reachable"
        return 0
    else
        log_error "HTTP server is not reachable"
        return 1
    fi
}

check_health_endpoint() {
    log_info "Checking /health endpoint..."

    local response
    response=$(curl -s --max-time "$TIMEOUT" "http://${HOST}:${PORT}/health" 2>&1) || {
        log_error "Health endpoint failed"
        return 1
    }

    if [ "$VERBOSE" = "1" ]; then
        echo "Response: $response"
    fi

    # Check if response contains "ok" or is valid JSON
    if echo "$response" | grep -q -E '(ok|healthy|running|"status":|"state":)'; then
        log_success "Health endpoint responding"
        return 0
    else
        log_warning "Health endpoint returned unexpected response"
        return 1
    fi
}

check_api_version() {
    log_info "Checking API version..."

    local response
    response=$(curl -s --max-time "$TIMEOUT" "http://${HOST}:${PORT}/v1/models" 2>&1) || {
        log_warning "Could not check API version (endpoint may not exist)"
        return 0  # Not critical
    }

    if [ "$VERBOSE" = "1" ]; then
        echo "Models response: $response"
    fi

    log_success "API endpoint accessible"
    return 0
}

check_mcp_connectivity() {
    log_info "Checking MCP server connectivity..."

    # MCP servers use stdio, so we check process existence instead
    if pgrep -f "mcp.*server" > /dev/null 2>&1; then
        log_success "MCP servers detected"
        return 0
    else
        log_warning "No MCP servers detected (may not be configured)"
        return 0  # Not critical if MCP not used
    fi
}

check_resource_usage() {
    log_info "Checking resource usage..."

    # Check if mistralrs-server is running
    local pid
    pid=$(pgrep -f "mistralrs-server" | head -1)

    if [ -z "$pid" ]; then
        log_warning "mistralrs-server process not found"
        return 0  # May be running in container
    fi

    # Get memory usage (MB)
    local mem_mb
    if command -v ps > /dev/null 2>&1; then
        mem_mb=$(ps -p "$pid" -o rss= 2>/dev/null | awk '{print int($1/1024)}')
        log_success "Memory usage: ${mem_mb}MB"
    fi

    # Check CUDA GPU usage if nvidia-smi available
    if command -v nvidia-smi > /dev/null 2>&1; then
        local gpu_mem
        gpu_mem=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits | head -1)
        log_success "GPU memory usage: ${gpu_mem}MB"
    fi

    return 0
}

check_model_loaded() {
    log_info "Checking if model is loaded..."

    # Try a simple completion request
    local response
    response=$(curl -s --max-time "$TIMEOUT" -X POST "http://${HOST}:${PORT}/v1/completions" \
        -H "Content-Type: application/json" \
        -d '{"prompt": "test", "max_tokens": 1}' 2>&1) || {
        log_warning "Could not verify model loading (may need authentication)"
        return 0  # Not critical for basic health check
    }

    if [ "$VERBOSE" = "1" ]; then
        echo "Completion response: $response"
    fi

    # Check if response is valid JSON (indicates model is loaded)
    if echo "$response" | python3 -m json.tool > /dev/null 2>&1; then
        log_success "Model appears to be loaded"
        return 0
    else
        log_warning "Model loading status unclear"
        return 0
    fi
}

check_disk_space() {
    log_info "Checking disk space..."

    # Check available disk space in /models if it exists
    local models_dir="${MODELS_DIR:-/models}"
    if [ -d "$models_dir" ]; then
        local available
        available=$(df -h "$models_dir" | awk 'NR==2 {print $4}')
        log_success "Available disk space in $models_dir: $available"
    fi

    return 0
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    echo ""
    log_info "mistral.rs Health Check"
    log_info "Target: http://${HOST}:${PORT}"
    echo ""

    local exit_code=0

    # Run checks
    check_http_connectivity || exit_code=$EXIT_HTTP_FAILED
    check_health_endpoint || exit_code=$EXIT_HEALTH_FAILED
    check_api_version
    check_mcp_connectivity
    check_model_loaded
    check_resource_usage || exit_code=$EXIT_RESOURCE_FAILED
    check_disk_space

    echo ""
    if [ $exit_code -eq 0 ]; then
        log_success "All critical health checks passed"
    else
        log_error "Some health checks failed"
    fi
    echo ""

    exit $exit_code
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  -h, --help     Show this help message"
        echo "  --verbose      Enable verbose output"
        echo ""
        echo "Environment variables:"
        echo "  MISTRALRS_HOST  Server host (default: localhost)"
        echo "  MISTRALRS_PORT  Server port (default: 8080)"
        echo "  TIMEOUT         Request timeout in seconds (default: 10)"
        echo ""
        exit 0
        ;;
    --verbose|-v)
        VERBOSE=1
        ;;
esac

main
