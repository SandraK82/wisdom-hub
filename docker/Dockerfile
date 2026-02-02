# Build stage
FROM rust:1.84-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto/ ./proto/

# Create dummy source for dependency caching
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# Build dependencies only
RUN cargo build --release && \
    rm -rf src target/release/wisdom-hub* target/release/.fingerprint/wisdom-hub*

# Copy actual source
COPY src/ ./src/

# Build the actual binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/wisdom-hub /app/wisdom-hub

# Copy default config
COPY config.toml /app/config.toml

# Create data directory
RUN mkdir -p /app/data

# Expose ports
EXPOSE 8080 50051

# Set environment variables
ENV RUST_LOG=info
ENV WISDOM_HUB__DATABASE__DATA_DIR=/app/data

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/wisdom-hub"]
