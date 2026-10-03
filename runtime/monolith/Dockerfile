# syntax=docker/dockerfile:1
# Image locale J3 : binaire oodhive-monolith (overlay démo kind, pas canon 0601).
# Contexte de build = racine du workspace Cargo.
# slim-bookworm + apt cmake/clang/nasm : même runtime/deps que Dockerfile.controller.
# rustc 1.94 (pas 1.89 J2) : Cargo.lock aws-sdk/smithy exige rustc >= 1.91.1.
# Runtime debian:bookworm-slim inchangé (glibc aligné J2).
FROM rust:1.94-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    nasm \
    perl \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src
COPY . .

ENV CARGO_TERM_COLOR=never
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,id=oodhive-monolith-target-bookworm,target=/usr/src/target \
    cargo build --release -p oodhive-monolith \
    && cp /usr/src/target/release/oodhive-monolith /tmp/oodhive-monolith

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /app/config \
    && chown 65534:65534 /app /app/config

COPY --from=builder /tmp/oodhive-monolith /usr/local/bin/oodhive-monolith
COPY --from=builder /usr/src/config/development.toml /app/config/development.toml
RUN chmod 0555 /usr/local/bin/oodhive-monolith \
    && chmod 0444 /app/config/development.toml

WORKDIR /app
USER 65534:65534
EXPOSE 8080 8443
ENTRYPOINT ["/usr/local/bin/oodhive-monolith"]
