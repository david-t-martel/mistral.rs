#!/bin/bash
# Quick deployment script for mistral.rs
# Supports: docker, systemd, manual

set -euo pipefail

# Configuration
DEPLOY_METHOD="${DEPLOY_METHOD:-docker}"
ENVIRONMENT="${ENVIRONMENT:-prod}"
MODEL_DIR="${MODEL_DIR:-/var/lib/mistralrs/models}"
CONFIG_DIR="${CONFIG_DIR:-./configs/${ENVIRONMENT}}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[!]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }

show_help() {
    cat << EOF
mistral.rs Deployment Script

Usage: $0 [OPTIONS]

Options:
    -m, --method METHOD      Deployment method: docker, systemd, manual (default: docker)
    -e, --environment ENV    Environment: dev, staging, prod (default: prod)
    --model-dir DIR          Model directory (default: /var/lib/mistralrs/models)
    --build                  Build binary before deployment
    --no-cache               Build Docker image without cache
    -h, --help               Show this help message

Environment Variables:
    DEPLOY_METHOD            Same as --method
    ENVIRONMENT              Same as --environment
    MODEL_DIR                Same as --model-dir

Examples:
    # Docker deployment (default)
    $0

    # Systemd deployment
    $0 --method systemd

    # Build and deploy
    $0 --build

    # Staging environment
    $0 --environment staging

EOF
    exit 0
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    case "$DEPLOY_METHOD" in
        docker)
            command -v docker >/dev/null 2>&1 || { log_error "Docker not found"; exit 1; }
            command -v docker-compose >/dev/null 2>&1 || { log_error "Docker Compose not found"; exit 1; }
            ;;
        systemd)
            command -v systemctl >/dev/null 2>&1 || { log_error "systemd not found"; exit 1; }
            ;;
    esac

    log_success "Prerequisites OK"
}

build_binary() {
    log_info "Building binary with CUDA support..."
    make build-cuda-full || {
        log_error "Build failed"
        exit 1
    }
    log_success "Binary built: target/release/mistralrs-server"
}

deploy_docker() {
    log_info "Deploying with Docker..."

    # Check if docker-compose.yml exists
    if [ ! -f docker-compose.yml ]; then
        log_error "docker-compose.yml not found"
        exit 1
    fi

    # Build image
    log_info "Building Docker image..."
    if [ "${NO_CACHE:-0}" = "1" ]; then
        docker-compose build --no-cache
    else
        docker-compose build
    fi

    # Deploy
    log_info "Starting containers..."
    docker-compose up -d

    # Wait for services to start
    sleep 5

    # Health check
    log_info "Running health checks..."
    if curl -f -s http://localhost:8080/health > /dev/null 2>&1; then
        log_success "Deployment successful!"
        log_info "API: http://localhost:8080"
        log_info "Grafana: http://localhost:3000"
    else
        log_warning "Service may not be fully started yet"
        log_info "Check logs with: docker-compose logs -f mistralrs"
    fi
}

deploy_systemd() {
    log_info "Deploying with systemd..."

    # Check if running as root
    if [ "$EUID" -ne 0 ]; then
        log_error "Please run with sudo for systemd deployment"
        exit 1
    fi

    # Check binary exists
    if [ ! -f target/release/mistralrs-server ]; then
        log_error "Binary not found. Run with --build first."
        exit 1
    fi

    # Create directories
    log_info "Creating directories..."
    mkdir -p /opt/mistralrs /var/lib/mistralrs/{models,cache} /etc/mistralrs /var/log/mistralrs

    # Create user if doesn't exist
    if ! id mistralrs >/dev/null 2>&1; then
        useradd -r -s /bin/false mistralrs
    fi

    # Copy binary
    log_info "Installing binary..."
    cp target/release/mistralrs-server /opt/mistralrs/
    chmod +x /opt/mistralrs/mistralrs-server
    chown mistralrs:mistralrs /opt/mistralrs/mistralrs-server

    # Copy chat templates
    cp -r chat_templates /opt/mistralrs/
    chown -R mistralrs:mistralrs /opt/mistralrs/chat_templates

    # Copy MCP config
    log_info "Installing configuration..."
    cp "${CONFIG_DIR}/mcp-config.json" /etc/mistralrs/mcp-config.json
    chown mistralrs:mistralrs /etc/mistralrs/mcp-config.json

    # Install systemd service
    log_info "Installing systemd service..."
    cp mistralrs-server.service /etc/systemd/system/
    systemctl daemon-reload

    # Set permissions
    chown -R mistralrs:mistralrs /var/lib/mistralrs /var/log/mistralrs

    # Enable and start service
    log_info "Starting service..."
    systemctl enable mistralrs-server
    systemctl restart mistralrs-server

    # Wait and check status
    sleep 3
    if systemctl is-active --quiet mistralrs-server; then
        log_success "Deployment successful!"
        log_info "Check status: sudo systemctl status mistralrs-server"
        log_info "View logs: sudo journalctl -u mistralrs-server -f"
    else
        log_error "Service failed to start"
        log_info "Check logs: sudo journalctl -u mistralrs-server -n 50"
        exit 1
    fi
}

deploy_manual() {
    log_info "Manual deployment instructions..."

    cat << EOF

Manual Deployment Steps:

1. Build binary:
   make build-cuda-full

2. Copy binary to deployment location:
   cp target/release/mistralrs-server /usr/local/bin/

3. Copy chat templates:
   cp -r chat_templates /opt/mistralrs/

4. Configure MCP:
   cp ${CONFIG_DIR}/mcp-config.json /etc/mistralrs/

5. Run server:
   mistralrs-server --port 8080 --mcp-config /etc/mistralrs/mcp-config.json \\
       gguf -m ${MODEL_DIR} -f Qwen2.5-1.5B-Instruct-Q4_K_M.gguf

EOF
}

# Parse arguments
BUILD=0
NO_CACHE=0

while [[ $# -gt 0 ]]; do
    case $1 in
        -m|--method)
            DEPLOY_METHOD="$2"
            shift 2
            ;;
        -e|--environment)
            ENVIRONMENT="$2"
            CONFIG_DIR="./configs/${ENVIRONMENT}"
            shift 2
            ;;
        --model-dir)
            MODEL_DIR="$2"
            shift 2
            ;;
        --build)
            BUILD=1
            shift
            ;;
        --no-cache)
            NO_CACHE=1
            shift
            ;;
        -h|--help)
            show_help
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            ;;
    esac
done

# Main execution
log_info "mistral.rs Deployment"
log_info "Method: $DEPLOY_METHOD"
log_info "Environment: $ENVIRONMENT"
echo ""

check_prerequisites

if [ "$BUILD" = "1" ]; then
    build_binary
fi

case "$DEPLOY_METHOD" in
    docker)
        deploy_docker
        ;;
    systemd)
        deploy_systemd
        ;;
    manual)
        deploy_manual
        ;;
    *)
        log_error "Invalid deployment method: $DEPLOY_METHOD"
        log_info "Valid methods: docker, systemd, manual"
        exit 1
        ;;
esac
