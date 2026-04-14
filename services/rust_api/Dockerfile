# Stage 1: Rust builder
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy Cargo files
COPY services/rust_api/Cargo.toml ./
RUN test -f services/rust_api/Cargo.lock && cp services/rust_api/Cargo.lock ./Cargo.lock || true

# Create dummy main.rs for dependency caching
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# Build dependencies for caching
RUN cargo build --release 2>/dev/null || true

# Copy actual source
COPY services/rust_api/src ./src

# Touch main.rs to rebuild
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM rust:slim-bookworm

WORKDIR /app

# Install PostgreSQL client for database migrations
RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/rust_api /app/

# Copy entrypoint and config
COPY services/rust_api/entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

# Copy database migrations from workspace root
COPY infra/db/migrations /app/migrations
RUN chmod -R 755 /app/migrations

# Copy static files (dashboard HTML, CSS, JS)
COPY services/rust_api/static /app/static
RUN chmod -R 755 /app/static

# Non-root user
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
