# Stage 1: Build
FROM rust:latest as builder

WORKDIR /app

# --- THE CURE: Install OpenSSL Dev ---
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p orchestrator/src
RUN echo "fn main() {}" > orchestrator/src/main.rs

COPY Cargo.toml .
COPY Cargo.lock .
COPY orchestrator/Cargo.toml orchestrator/

RUN cargo build --release --manifest-path orchestrator/Cargo.toml
RUN rm orchestrator/src/main.rs

COPY orchestrator/src orchestrator/src
COPY orchestrator/migrations orchestrator/migrations

RUN touch orchestrator/src/main.rs
RUN cargo build --release --manifest-path orchestrator/Cargo.toml

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# --- THE CURE: Install OpenSSL Runtime ---
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/orchestrator /app/orchestrator
COPY --from=builder /app/orchestrator/migrations /app/migrations

EXPOSE 8080
CMD ["./orchestrator"]