# Multi-stage Dockerfile for WinUtils
# Supports both Linux utilities and Windows cross-compilation

# Stage 1: Build environment
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Install cross-compilation targets
RUN rustup target add x86_64-unknown-linux-musl \
    && rustup target add x86_64-pc-windows-gnu

# Install cross-compilation tools
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    gcc-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

# Configure cross-compilation
ENV CC_x86_64_unknown_linux_musl=musl-gcc
ENV CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

# Build for Linux (musl for static linking)
RUN cargo build --release --target x86_64-unknown-linux-musl

# Build for Windows (cross-compilation)
RUN cargo build --release --target x86_64-pc-windows-gnu

# Stage 2: Linux runtime image
FROM alpine:3.18 as linux-runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tzdata

# Create non-root user
RUN adduser -D -s /bin/sh winutils

# Copy Linux binaries
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/ls /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/cat /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/cp /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mv /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rm /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/grep /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/sort /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/wc /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/head /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/tail /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/cut /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/tr /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/find /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/which /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/du /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/touch /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mkdir /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rmdir /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pwd /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/echo /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/chmod /usr/local/bin/

# Set permissions
RUN chmod +x /usr/local/bin/*

# Switch to non-root user
USER winutils

# Set working directory
WORKDIR /home/winutils

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ls --version || exit 1

# Default command
CMD ["/bin/sh"]

# Stage 3: Windows binary distribution
FROM scratch as windows-dist

# Copy Windows binaries
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/*.exe /

# Stage 4: Multi-arch distribution
FROM alpine:3.18 as dist

# Create directories for different architectures
RUN mkdir -p /dist/linux-x64 /dist/windows-x64

# Copy Linux binaries
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/* /dist/linux-x64/

# Copy Windows binaries
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/*.exe /dist/windows-x64/

# Create archive scripts
RUN echo '#!/bin/sh' > /create-archives.sh && \
    echo 'cd /dist' >> /create-archives.sh && \
    echo 'tar -czf winutils-linux-x64.tar.gz -C linux-x64 .' >> /create-archives.sh && \
    echo 'cd windows-x64 && zip -r ../winutils-windows-x64.zip . && cd ..' >> /create-archives.sh && \
    chmod +x /create-archives.sh

# Stage 5: Development image with tools
FROM rust:1.75-slim as development

# Install development tools
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    make \
    gdb \
    valgrind \
    strace \
    htop \
    vim \
    ripgrep \
    fd-find \
    && rm -rf /var/lib/apt/lists/*

# Install additional Rust tools
RUN cargo install cargo-watch cargo-audit cargo-deny cargo-tarpaulin

# Set working directory
WORKDIR /workspace

# Copy source code
COPY . .

# Expose port for development server (if needed)
EXPOSE 8080

# Default development command
CMD ["cargo", "build"]

# Stage 6: Benchmark runner
FROM rust:1.75-slim as benchmark

# Install dependencies for benchmarking
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    time \
    hyperfine \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source and benchmarks
COPY . .

# Build benchmark suite
RUN cd benchmarks && cargo build --release

# Set up benchmark environment
ENV RUST_LOG=info
ENV BENCHMARK_MODE=docker

# Default benchmark command
CMD ["./benchmarks/target/release/benchmark-runner", "run", "--compare-native", "--output", "/benchmarks/results"]

# Stage 7: CI runner (minimal image for CI/CD)
FROM alpine:3.18 as ci

# Install minimal dependencies for CI
RUN apk add --no-cache \
    git \
    curl \
    bash

# Copy only the built binaries
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/* /usr/local/bin/

# Set permissions
RUN chmod +x /usr/local/bin/*

# Verification script
RUN echo '#!/bin/bash' > /verify.sh && \
    echo 'echo "Verifying WinUtils installation..."' >> /verify.sh && \
    echo 'for util in ls cat cp mv rm grep sort wc head tail cut tr find which du touch mkdir rmdir pwd echo chmod; do' >> /verify.sh && \
    echo '  if command -v $util >/dev/null 2>&1; then' >> /verify.sh && \
    echo '    echo "✓ $util"' >> /verify.sh && \
    echo '  else' >> /verify.sh && \
    echo '    echo "✗ $util"' >> /verify.sh && \
    echo '    exit 1' >> /verify.sh && \
    echo '  fi' >> /verify.sh && \
    echo 'done' >> /verify.sh && \
    echo 'echo "All utilities verified!"' >> /verify.sh && \
    chmod +x /verify.sh

# Default verification
CMD ["/verify.sh"]
