# syntax=docker/dockerfile:1
# Image Linux pour Kind et le profil Compose mesh.
# Contexte de build = racine du workspace Cargo.
# Le binaire hôte reste `cargo build` → target/. Cette image ne relance pas
# un second cargo sur l'hôte : la compilation Linux est dans le conteneur,
# comme runtime/monolith/Dockerfile.
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
    --mount=type=cache,id=ext-authz-target-bookworm,target=/usr/src/target \
    cargo build --release -p ext-authz \
    && cp /usr/src/target/release/ext-authz /tmp/ext-authz

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && chown 65534:65534 /app

COPY --from=builder /tmp/ext-authz /app/ext-authz
RUN chmod 755 /app/ext-authz

USER 65534

ENV PORT=8090
EXPOSE 8090

ENTRYPOINT ["/app/ext-authz"]
