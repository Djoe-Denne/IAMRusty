# Get build image argument
ARG BUILD_IMAGE=local/build-artifacts:latest

# Copy artifacts from build image
FROM ${BUILD_IMAGE} as build-source

# Final stage - minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user and directory
RUN adduser --disabled-password --gecos "" appuser
WORKDIR /app

# Copy the binary from builder stage
COPY --from=build-source /app/target/release/hive-service /app/hive-service
# hive-migration crate exposes its binary as `migration`; rename it inside
# the container so the entrypoint can call /app/migration like other services.
COPY --from=build-source /app/target/release/migration /app/migration

# Copy entrypoint script
COPY Hive/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Copy configuration files
COPY Hive/config /app/config

# Change ownership
RUN chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Set entrypoint
ENTRYPOINT ["/app/docker-entrypoint.sh"]
