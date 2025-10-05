#!/bin/bash
set -euo pipefail

# Unix deployment script for winutils
SCRIPT_VERSION="1.0.0"
TARGET="${1:-local}"
VERSION="${2:-latest}"
ENVIRONMENT="${3:-development}"

log_info() {
    echo "[INFO] $1"
}

log_success() {
    echo "[SUCCESS] $1"
}

log_error() {
    echo "[ERROR] $1" >&2
}

deploy_local() {
    log_info "Starting local deployment..."

    # Build using mandatory Makefile
    log_info "Building winutils using Makefile..."
    make clean
    make release

    # Install locally
    make install

    log_success "Local deployment completed"
}

deploy_docker() {
    log_info "Starting Docker deployment..."

    # Build Docker image
    docker build -t winutils:$VERSION .

    # Tag for registry
    docker tag winutils:$VERSION registry.example.com/winutils:$VERSION

    log_success "Docker deployment completed"
}

deploy_snap() {
    log_info "Starting Snap deployment..."

    # Build snap package
    snapcraft

    log_success "Snap deployment completed"
}

main() {
    log_info "winutils Unix Deployment v$SCRIPT_VERSION"
    log_info "Target: $TARGET, Version: $VERSION, Environment: $ENVIRONMENT"

    case "$TARGET" in
        local)
            deploy_local
            ;;
        docker)
            deploy_docker
            ;;
        snap)
            deploy_snap
            ;;
        *)
            log_error "Unknown target: $TARGET"
            exit 1
            ;;
    esac

    log_success "Deployment completed successfully!"
}

main "$@"
