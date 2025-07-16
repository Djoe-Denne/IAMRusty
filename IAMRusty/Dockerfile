# Get build image argument
ARG BUILD_IMAGE=local/build-artifacts:latest

# Copy artifacts from build image
FROM ${BUILD_IMAGE} as build-source

# Final stage - minimal runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binaries from the builder stage
COPY --from=build-source /app/target/release/iam-service /app/iam-service
COPY --from=build-source /app/target/release/iammigration /app/migration

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