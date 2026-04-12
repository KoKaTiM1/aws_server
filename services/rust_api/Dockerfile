# Stage 1: Rust builder
FROM rust:1.75-slim-bullseye as builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency caching
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# Build dependencies for caching
RUN cargo build --release

# Copy actual source
COPY src ./src

# Touch main.rs to rebuild
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/rust_api /app/

# Copy entrypoint and config
COPY entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

# Non-root user
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
