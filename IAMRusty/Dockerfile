FROM rust:1.87-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src

# Copy shared dependencies
COPY iam-events iam-events/

# Copy rustycog workspace
COPY rustycog/Cargo.toml rustycog/
COPY rustycog/rustycog-core rustycog/rustycog-core/
COPY rustycog/rustycog-events rustycog/rustycog-events/
COPY rustycog/rustycog-http rustycog/rustycog-http/
COPY rustycog/rustycog-command rustycog/rustycog-command/
COPY rustycog/rustycog-config rustycog/rustycog-config/
COPY rustycog/rustycog-db rustycog/rustycog-db/
COPY rustycog/rustycog-server rustycog/rustycog-server/
COPY rustycog/rustycog-testing rustycog/rustycog-testing/

# Copy IAMRusty source files only (exclude target directory)
COPY IAMRusty/Cargo.toml IAMRusty/Cargo.lock IAMRusty/
COPY IAMRusty/domain IAMRusty/domain/
COPY IAMRusty/application IAMRusty/application/
COPY IAMRusty/infra IAMRusty/infra/
COPY IAMRusty/http IAMRusty/http/
COPY IAMRusty/migration IAMRusty/migration/
COPY IAMRusty/setup IAMRusty/setup/
COPY IAMRusty/configuration IAMRusty/configuration/
COPY IAMRusty/src IAMRusty/src/

# Set working directory to IAMRusty
WORKDIR /usr/src/IAMRusty

# Build the application and migration tool
RUN cargo build --release --features test-mode
RUN cd migration && cargo build --release

# Final stage - minimal runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binaries from the builder stage
COPY --from=builder /usr/src/IAMRusty/target/release/iam-service /app/iam-service
COPY --from=builder /usr/src/IAMRusty/target/release/migration /app/migration

# Copy entrypoint script
COPY IAMRusty/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Create directories for application data
RUN mkdir -p /app/keys /app/config /app/certs

# Run as non-root user
RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

# Set the entrypoint to run migrations first, then the service
ENTRYPOINT ["/app/docker-entrypoint.sh"] 