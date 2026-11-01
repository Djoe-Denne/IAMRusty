# Get build image argument
ARG BUILD_IMAGE=local/build-artifacts:latest

# Copy artifacts from build image
FROM ${BUILD_IMAGE} as build-source
ARG SOURCE_REVISION=unattested
ARG SDK_REVISION=unattested
RUN if [ "$SOURCE_REVISION" != unattested ]; then \
    test "$(cat /app/target/release/delivery-profile)" = openbao \
    && test "$(cat /app/target/release/source-revision)" = "$SOURCE_REVISION" \
    && test "$(cat /app/target/release/sdk-revision)" = "$SDK_REVISION"; fi

# Final stage - minimal runtime image
FROM debian:bookworm-slim
ARG SOURCE_REVISION=unattested
ARG SDK_REVISION=unattested
LABEL org.opencontainers.image.revision=$SOURCE_REVISION aiforall.dev.sdk-revision=$SDK_REVISION

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage (no migrations)
COPY --from=build-source /app/target/release/github-connect-service /app/github-connect-service

COPY services/GitHubConnect/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

COPY services/GitHubConnect/config /app/config

# Create directories for application data
RUN mkdir -p /app/certs

# Run as non-root user
RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

EXPOSE 8080 8443

ENTRYPOINT ["/app/docker-entrypoint.sh"]
