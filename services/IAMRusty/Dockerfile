# Get build image argument
ARG BUILD_IMAGE=local/build-artifacts:latest
ARG RUNTIME_IMAGE=debian:bookworm-slim

# Copy artifacts from build image
FROM ${BUILD_IMAGE} as build-source

# Final stage - minimal runtime image
FROM ${RUNTIME_IMAGE} AS runtime
ARG RUNTIME_OPENSSL_PACKAGE=libssl-dev
ARG SOURCE_REVISION=unattested
ARG SDK_REVISION=unattested
LABEL org.opencontainers.image.revision=$SOURCE_REVISION aiforall.dev.sdk-revision=$SDK_REVISION

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ${RUNTIME_OPENSSL_PACKAGE} \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binaries from the builder stage
COPY --from=build-source /app/target/release/iam-service /app/iam-service
COPY --from=build-source /app/target/release/iammigration /app/migration

# Copy entrypoint script
COPY services/IAMRusty/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Create directories for application data
RUN mkdir -p /app/keys /app/config /app/certs

# Run as non-root user
RUN (getent group 1000 >/dev/null || groupadd --gid 1000 appuser) \
    && (getent passwd 1000 >/dev/null || useradd --uid 1000 --gid 1000 --no-create-home appuser) \
    && chown -R 1000:1000 /app
USER 1000:1000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/iam/health || exit 1

# Set the entrypoint to run migrations first, then the service
ENTRYPOINT ["/app/docker-entrypoint.sh"]
