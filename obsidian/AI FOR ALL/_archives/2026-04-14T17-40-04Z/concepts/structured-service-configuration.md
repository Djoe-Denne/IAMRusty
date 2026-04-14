---
title: >-
  Structured Service Configuration
category: concepts
tags: [configuration, env, rust, visibility/internal]
sources:
  - IAMRusty/docs/DATABASE_CONFIGURATION.md
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
summary: >-
  Services use typed config loaders, RUN_ENV-selected profiles, nested env overrides, queue enums, and structured DB settings instead of ad hoc string parsing.
provenance:
  extracted: 0.85
  inferred: 0.08
  ambiguous: 0.07
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Structured Service Configuration

AIForAll services increasingly rely on typed configuration objects rather than manual URL parsing or scattered env lookups. The clearest examples come from `[[projects/iamrusty/iamrusty]]` and the Manifesto-based guidance captured in `[[skills/building-rustycog-services]]`.

## Key Ideas

- Each service defines a typed config with a service-specific env prefix, and `rustycog-config` selects `config/development.toml`, `config/test.toml`, or `config/production.toml` from `RUN_ENV`.
- Nested environment overrides use double underscores, which keeps complex settings like database credentials, queue transport, and logging options consistent.
- The config crate stays generic through traits such as `ConfigLoader`, `ConfigCache`, `HasDbConfig`, `HasQueueConfig`, `HasServerConfig`, and `HasLoggingConfig`, so shared crates can depend on capabilities instead of one service's concrete config type.
- Database config has moved toward structured fields (`host`, `port`, `db`, nested creds) while still allowing replica URLs for flexibility.
- `QueueConfig` turns transport choice into data: services can select Kafka, SQS, or disabled mode without changing the higher-level event abstractions.
- Random port resolution plus config and port caching make container-based tests more predictable when services set ports to `0`.
- Logging is part of the same typed configuration story through level/filter controls, console/file outputs, and optional Scaleway Loki settings.
- Manifesto's docs note that `config/default.toml` is not automatically merged by the current loader and some declared config sections are not fully consumed at runtime. ^[ambiguous]

## Open Questions

- The wiki does not yet catalog which services prefer cached config loads versus always-fresh loads.
- Queue prefixes and environment-variable naming are still service-specific, so the repo does not yet present one uniform cross-service env contract. ^[ambiguous]
- Dynamic config reload appears as future-looking guidance rather than a currently documented platform standard. ^[ambiguous]

## Sources

- [[references/iamrusty-runtime-and-security]] — Runtime config and DB details
- [[references/rustycog-service-construction]] — Loader and composition-root guidance
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the config and logging crates
- [[concepts/shared-rust-microservice-sdk]] — Broader stack this configuration model supports
