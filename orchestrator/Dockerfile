# Stage 1: Build
FROM rust:latest as builder

WORKDIR /app

# Create a dummy project to cache dependencies
RUN mkdir -p orchestrator/src
RUN echo "fn main() {}" > orchestrator/src/main.rs

COPY Cargo.toml .
COPY Cargo.lock .
COPY orchestrator/Cargo.toml orchestrator/

# Build dependencies
RUN cargo build --release --manifest-path orchestrator/Cargo.toml

# Remove dummy file
RUN rm orchestrator/src/main.rs

# Copy actual source code
COPY orchestrator/src orchestrator/src
# Copy migrations if they exist
COPY orchestrator/migrations orchestrator/migrations

# Build the actual application
# Use 'touch' to force cargo to rebuild the main file
RUN touch orchestrator/src/main.rs
RUN cargo build --release --manifest-path orchestrator/Cargo.toml

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (OpenSSL, ca-certificates)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/orchestrator /app/orchestrator
COPY --from=builder /app/orchestrator/migrations /app/migrations

# Expose the port
EXPOSE 8080

# Run the binary
CMD ["./orchestrator"]