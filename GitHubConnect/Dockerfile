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

# Copy the binary from the builder stage (no migrations)
COPY --from=build-source /app/target/release/github-connect-service /app/github-connect-service

COPY GitHubConnect/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

COPY GitHubConnect/config /app/config

# Create directories for application data
RUN mkdir -p /app/certs

# Run as non-root user
RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

EXPOSE 8080 8443

ENTRYPOINT ["/app/docker-entrypoint.sh"]
