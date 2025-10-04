# Multi-stage Dockerfile for mistral.rs server
# Optimized for: CUDA support, minimal runtime size, production readiness
# Base image: nvidia/cuda for GPU acceleration

# ============================================================================
# Stage 1: Builder - Compile Rust binary with CUDA support
# ============================================================================
FROM nvidia/cuda:12.9.0-devel-ubuntu24.04 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.86.0

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y \
    --default-toolchain ${RUST_VERSION} \
    --profile minimal \
    --no-modify-path

# Set working directory
WORKDIR /build

# Copy workspace configuration first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY mistralrs-core/Cargo.toml ./mistralrs-core/
COPY mistralrs-server/Cargo.toml ./mistralrs-server/
COPY mistralrs-server-core/Cargo.toml ./mistralrs-server-core/
COPY mistralrs/Cargo.toml ./mistralrs/
COPY mistralrs-vision/Cargo.toml ./mistralrs-vision/
COPY mistralrs-quant/Cargo.toml ./mistralrs-quant/
COPY mistralrs-paged-attn/Cargo.toml ./mistralrs-paged-attn/
COPY mistralrs-audio/Cargo.toml ./mistralrs-audio/
COPY mistralrs-mcp/Cargo.toml ./mistralrs-mcp/
COPY mistralrs-bench/Cargo.toml ./mistralrs-bench/
COPY mistralrs-web-chat/Cargo.toml ./mistralrs-web-chat/

# Create dummy source files to cache dependencies
RUN mkdir -p \
    mistralrs-core/src \
    mistralrs-server/src \
    mistralrs-server-core/src \
    mistralrs/src \
    mistralrs-vision/src \
    mistralrs-quant/src \
    mistralrs-paged-attn/src \
    mistralrs-audio/src \
    mistralrs-mcp/src \
    mistralrs-bench/src \
    mistralrs-web-chat/src && \
    echo "fn main() {}" > mistralrs-server/src/main.rs && \
    echo "fn main() {}" > mistralrs-bench/src/main.rs && \
    touch mistralrs-core/src/lib.rs && \
    touch mistralrs-server-core/src/lib.rs && \
    touch mistralrs/src/lib.rs && \
    touch mistralrs-vision/src/lib.rs && \
    touch mistralrs-quant/src/lib.rs && \
    touch mistralrs-paged-attn/src/lib.rs && \
    touch mistralrs-audio/src/lib.rs && \
    touch mistralrs-mcp/src/lib.rs && \
    touch mistralrs-web-chat/src/lib.rs

# Build dependencies only (will be cached)
RUN cargo build --release \
    --package mistralrs-server \
    --features "cuda,flash-attn,cudnn,mkl" && \
    rm -rf target/release/.fingerprint/mistralrs* && \
    rm -rf target/release/deps/mistralrs*

# Copy actual source code
COPY . .

# Build final binary with all features
RUN cargo build --release \
    --package mistralrs-server \
    --features "cuda,flash-attn,cudnn,mkl" && \
    strip target/release/mistralrs-server

# ============================================================================
# Stage 2: Runtime - Minimal image with CUDA runtime
# ============================================================================
FROM nvidia/cuda:12.9.0-runtime-ubuntu24.04 AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libgomp1 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash mistralrs && \
    mkdir -p /app /data /models /config /logs && \
    chown -R mistralrs:mistralrs /app /data /models /config /logs

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/mistralrs-server /app/mistralrs-server

# Copy chat templates (needed for inference)
COPY --chown=mistralrs:mistralrs chat_templates /app/chat_templates

# Switch to non-root user
USER mistralrs

# Environment variables with sensible defaults
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    MISTRALRS_PORT=8080 \
    MISTRALRS_HOST=0.0.0.0 \
    MISTRALRS_MAX_SEQS=256 \
    MISTRALRS_WORKERS=2

# Expose HTTP server port
EXPOSE 8080

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${MISTRALRS_PORT}/health || exit 1

# Volume mount points
VOLUME ["/models", "/config", "/logs", "/data"]

# Default command (can be overridden)
ENTRYPOINT ["/app/mistralrs-server"]
CMD ["--help"]
