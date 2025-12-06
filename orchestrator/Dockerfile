# Multi-stage build for Rust orchestrator

# Stage 1: Build (use recent Rust for edition 2024 support)
FROM rust:latest as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests
COPY Cargo.toml ./
COPY orchestrator/Cargo.toml orchestrator/

# Create dummy main to cache dependencies
RUN mkdir -p orchestrator/src && \
    echo "fn main() {}" > orchestrator/src/main.rs

# Build dependencies only
RUN cargo build --release --manifest-path orchestrator/Cargo.toml

# Remove dummy main
RUN rm orchestrator/src/main.rs

# Copy actual source code
COPY orchestrator/src orchestrator/src
COPY orchestrator/migrations orchestrator/migrations

# Build the application (clean to avoid stale dummy build artifacts)
RUN cargo clean && cargo build --release --manifest-path orchestrator/Cargo.toml

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (curl needed for healthcheck)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/orchestrator /app/orchestrator

# Copy migrations
COPY orchestrator/migrations /app/migrations

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["/app/orchestrator"]
