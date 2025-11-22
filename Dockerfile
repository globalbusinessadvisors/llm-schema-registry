# ============================================================================
# Multi-Stage Dockerfile for LLM Schema Registry
# Optimized for production deployment with minimal image size
# ============================================================================

# ============================================================================
# Stage 1: Build Environment
# ============================================================================
FROM rust:1.82-bookworm AS builder

# Install system dependencies and protoc
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Verify protoc installation
RUN protoc --version

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/

# Copy proto files for gRPC compilation
COPY proto/ ./proto/

# Build dependencies first (cached layer)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo fetch

# Build all binaries in release mode with optimizations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --workspace && \
    cp target/release/schema-registry-server /tmp/schema-registry-server && \
    cp target/release/schema-cli /tmp/schema-cli

# Verify binaries were built
RUN ls -lh /tmp/schema-registry-server /tmp/schema-cli

# ============================================================================
# Stage 2: Runtime Image - Server
# ============================================================================
FROM debian:bookworm-slim AS runtime-server

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r schema-registry && \
    useradd -r -g schema-registry -u 1000 schema-registry && \
    mkdir -p /app /data /config && \
    chown -R schema-registry:schema-registry /app /data /config

# Set working directory
WORKDIR /app

# Copy server binary from builder
COPY --from=builder /tmp/schema-registry-server /app/schema-registry-server

# Copy configuration template
COPY --chown=schema-registry:schema-registry .env.example /config/config.example

# Switch to non-root user
USER schema-registry

# Expose ports
# 8080: HTTP REST API
# 9090: gRPC API
# 9091: Metrics endpoint
EXPOSE 8080 9090 9091

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/schema-registry-server", "health"] || exit 1

# Set environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1

# Run the server
ENTRYPOINT ["/app/schema-registry-server"]
CMD ["serve"]

# ============================================================================
# Stage 3: Runtime Image - CLI
# ============================================================================
FROM debian:bookworm-slim AS runtime-cli

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r schema-registry && \
    useradd -r -g schema-registry -u 1000 schema-registry && \
    mkdir -p /app && \
    chown -R schema-registry:schema-registry /app

WORKDIR /app

# Copy CLI binary from builder
COPY --from=builder /tmp/schema-cli /app/schema-cli

# Switch to non-root user
USER schema-registry

# Set environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1

# Run the CLI
ENTRYPOINT ["/app/schema-cli"]
CMD ["--help"]

# ============================================================================
# Default target is runtime-server
# ============================================================================
FROM runtime-server
