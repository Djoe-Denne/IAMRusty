---
title: RustyCog Logger
category: references
tags: [reference, rustycog, logging, visibility/internal]
sources:
  - rustycog/rustycog-logger/src/lib.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/Cargo.toml
  - Cargo.toml
summary: rustycog-logger centralizes tracing subscriber setup with level/filter handling and optional Scaleway Loki wiring behind feature flags.
provenance:
  extracted: 0.86
  inferred: 0.06
  ambiguous: 0.08
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Logger

`rustycog-logger` standardizes tracing initialization for services that implement `HasLoggingConfig` (and `HasScalewayConfig` when Loki support is enabled).

## Key Ideas

- `setup_logging()` maps configured log level/filter into an `EnvFilter` chain and initializes tracing layers.
- Logging setup uses `try_init()` to avoid panics when tests or nested startup paths initialize tracing multiple times.
- Under the `scaleway-loki` feature, the crate can attach Loki output using config-driven endpoint and token settings.
- The trait alias `ServiceLoggerConfig` changes by feature flag so services only need the capabilities required by the active build.

## Linked Entities

- [[entities/queue-config]]
- [[entities/service-error]]

## Open Questions

- `rustycog-logger` is included by `rustycog-meta` but is not listed as a root workspace member in `Cargo.toml`. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[concepts/structured-service-configuration]]
- [[projects/rustycog/rustycog]]
