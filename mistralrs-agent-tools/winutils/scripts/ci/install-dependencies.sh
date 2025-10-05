#!/bin/bash
# Install Dependencies Script for WinUtils CI/CD
# Installs required tools and dependencies for building and testing WinUtils

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case $ARCH in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    log_info "Detected platform: $OS-$ARCH"
}

# Install Rust toolchain
install_rust() {
    log_info "Installing Rust toolchain..."

    if command -v rustc >/dev/null 2>&1; then
        log_info "Rust already installed: $(rustc --version)"
        return 0
    fi

    # Install rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source ~/.cargo/env

    # Install additional targets
    rustup target add x86_64-unknown-linux-musl
    if [ "$OS" = "linux" ]; then
        rustup target add x86_64-pc-windows-gnu
    fi

    # Install useful cargo tools
    cargo install cargo-audit cargo-deny cargo-tarpaulin cargo-watch

    log_success "Rust toolchain installed successfully"
}

# Install system dependencies for Linux
install_linux_deps() {
    log_info "Installing Linux system dependencies..."

    # Detect Linux distribution
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO=$ID
    else
        log_error "Cannot detect Linux distribution"
        exit 1
    fi

    case $DISTRO in
        ubuntu|debian)
            sudo apt-get update
            sudo apt-get install -y \
                build-essential \
                pkg-config \
                libssl-dev \
                musl-tools \
                musl-dev \
                gcc-mingw-w64-x86-64 \
                git \
                curl \
                wget \
                jq \
                bc \
                zip \
                unzip \
                make \
                cmake \
                clang \
                llvm \
                hyperfine \
                ripgrep \
                fd-find
            ;;
        fedora|centos|rhel)
            sudo dnf install -y \
                gcc \
                gcc-c++ \
                pkg-config \
                openssl-devel \
                musl-gcc \
                mingw64-gcc \
                git \
                curl \
                wget \
                jq \
                bc \
                zip \
                unzip \
                make \
                cmake \
                clang \
                llvm \
                hyperfine \
                ripgrep \
                fd-find
            ;;
        arch)
            sudo pacman -S --noconfirm \
                base-devel \
                pkg-config \
                openssl \
                musl \
                mingw-w64-gcc \
                git \
                curl \
                wget \
                jq \
                bc \
                zip \
                unzip \
                make \
                cmake \
                clang \
                llvm \
                hyperfine \
                ripgrep \
                fd-find
            ;;
        *)
            log_warning "Unsupported Linux distribution: $DISTRO"
            log_info "Please install dependencies manually"
            ;;
    esac

    log_success "Linux dependencies installed successfully"
}

# Install system dependencies for macOS
install_macos_deps() {
    log_info "Installing macOS system dependencies..."

    # Check if Homebrew is installed
    if ! command -v brew >/dev/null 2>&1; then
        log_info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi

    # Install dependencies
    brew update
    brew install \
        pkg-config \
        openssl \
        git \
        curl \
        wget \
        jq \
        bc \
        zip \
        unzip \
        make \
        cmake \
        llvm \
        hyperfine \
        ripgrep \
        fd

    log_success "macOS dependencies installed successfully"
}

# Install development tools
install_dev_tools() {
    log_info "Installing development tools..."

    # Install GitHub CLI
    if ! command -v gh >/dev/null 2>&1; then
        case $OS in
            linux)
                if command -v apt-get >/dev/null 2>&1; then
                    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
                    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
                    sudo apt-get update
                    sudo apt-get install gh
                elif command -v dnf >/dev/null 2>&1; then
                    sudo dnf install 'dnf-command(config-manager)'
                    sudo dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo
                    sudo dnf install gh
                fi
                ;;
            darwin)
                brew install gh
                ;;
        esac
    fi

    # Install Docker if not present
    if ! command -v docker >/dev/null 2>&1; then
        log_info "Docker not found. Please install Docker manually for container builds."
    fi

    # Install additional benchmarking tools
    if ! command -v hyperfine >/dev/null 2>&1; then
        case $OS in
            linux)
                # Try to install from package manager or build from source
                if command -v apt-get >/dev/null 2>&1; then
                    sudo apt-get install hyperfine || {
                        log_warning "hyperfine not available in package manager, skipping"
                    }
                fi
                ;;
            darwin)
                brew install hyperfine
                ;;
        esac
    fi

    log_success "Development tools installed successfully"
}

# Install benchmarking tools
install_benchmark_tools() {
    log_info "Installing benchmarking and profiling tools..."

    # Install criterion benchmark framework (handled by Cargo.toml)
    # Install additional profiling tools
    case $OS in
        linux)
            if command -v apt-get >/dev/null 2>&1; then
                sudo apt-get install -y \
                    valgrind \
                    perf-tools-unstable \
                    linux-tools-common \
                    linux-tools-generic \
                    time \
                    strace \
                    htop \
                    iotop \
                    sysstat
            fi
            ;;
        darwin)
            brew install \
                gnu-time \
                htop \
                dtruss
            ;;
    esac

    log_success "Benchmarking tools installed successfully"
}

# Install cross-compilation tools
install_cross_compilation() {
    log_info "Setting up cross-compilation environment..."

    case $OS in
        linux)
            # Already installed mingw64-gcc and musl-tools above

            # Set up cross-compilation environment variables
            cat >> ~/.bashrc << 'EOF'

# WinUtils cross-compilation environment
export CC_x86_64_unknown_linux_musl=musl-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
EOF
            ;;
        darwin)
            # Install cross-compilation tools for macOS
            # For Windows cross-compilation on macOS, we need additional setup
            log_info "Cross-compilation setup for macOS requires additional manual configuration"
            ;;
    esac

    log_success "Cross-compilation tools configured"
}

# Verify installation
verify_installation() {
    log_info "Verifying installation..."

    local errors=0

    # Check Rust
    if ! command -v rustc >/dev/null 2>&1; then
        log_error "Rust not found"
        ((errors++))
    else
        log_success "Rust: $(rustc --version)"
    fi

    # Check Cargo
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo not found"
        ((errors++))
    else
        log_success "Cargo: $(cargo --version)"
    fi

    # Check build tools
    for tool in gcc make cmake; do
        if ! command -v $tool >/dev/null 2>&1; then
            log_warning "$tool not found"
        else
            log_success "$tool: $($tool --version | head -1)"
        fi
    done

    # Check development tools
    for tool in git jq; do
        if ! command -v $tool >/dev/null 2>&1; then
            log_error "$tool not found"
            ((errors++))
        else
            log_success "$tool: $($tool --version)"
        fi
    done

    if [ $errors -eq 0 ]; then
        log_success "All required tools installed successfully!"
        return 0
    else
        log_error "$errors critical tools missing"
        return 1
    fi
}

# Main installation process
main() {
    log_info "Starting WinUtils dependency installation..."

    detect_platform

    # Install based on platform
    case $OS in
        linux)
            install_linux_deps
            ;;
        darwin)
            install_macos_deps
            ;;
        *)
            log_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac

    install_rust
    install_dev_tools
    install_benchmark_tools
    install_cross_compilation

    verify_installation

    log_success "WinUtils dependencies installed successfully!"
    log_info "You may need to restart your shell or run 'source ~/.bashrc' to use new tools"
}

# Run main function
main "$@"
