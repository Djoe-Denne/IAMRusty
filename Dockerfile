FROM rust:1.75-slim as builder

WORKDIR /usr/src/app

# Copy over manifests and dependencies first
COPY Cargo.toml Cargo.lock ./
COPY domain/Cargo.toml domain/
COPY application/Cargo.toml application/
COPY infra/Cargo.toml infra/
COPY http/Cargo.toml http/

# Create dummy source files to build dependencies
RUN mkdir -p domain/src application/src infra/src http/src src \
    && echo "fn main() {}" > src/main.rs \
    && echo "pub fn dummy() {}" > domain/src/lib.rs \
    && echo "pub fn dummy() {}" > application/src/lib.rs \
    && echo "pub fn dummy() {}" > infra/src/lib.rs \
    && echo "pub fn dummy() {}" > http/src/lib.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy source files
RUN rm -rf domain/src application/src infra/src http/src src

# Copy the actual source code
COPY domain/src domain/src/
COPY application/src application/src/
COPY infra/src infra/src/
COPY http/src http/src/
COPY src src/

# Touch the source files to trigger rebuild
RUN find . -name "*.rs" -exec touch {} \;

# Build the application
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/iam-service /app/iam-service

# Create a directory for keys
RUN mkdir -p /app/keys /app/config

# Run as non-root user
RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

# Set the entrypoint
ENTRYPOINT ["/app/iam-service"] 