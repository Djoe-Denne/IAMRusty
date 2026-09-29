# Opt-in mesh profile (ADR-0308). Copies release binary from Dockerfile.build artifacts.
# Does not alter IAM/Hive/Telegraph/Manifesto/Lazaret images.
ARG BUILD_IMAGE=local/build-artifacts:latest

FROM ${BUILD_IMAGE} AS build-source

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build-source /app/target/release/ext-authz /app/ext-authz

RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

ENV PORT=8090
EXPOSE 8090

ENTRYPOINT ["/app/ext-authz"]
