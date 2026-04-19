---
title: >-
  RustyCog
category: project
tags: [sdk, rust, platform, visibility/internal]
sources:
  - rustycog/README.md
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  RustyCog is the shared Rust SDK/workspace that standardizes commands, config, HTTP, permissions, events, logging, DB access, and test infrastructure across services.
provenance:
  extracted: 0.77
  inferred: 0.09
  ambiguous: 0.14
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-19T10:59:36Z
---

# RustyCog

RustyCog is the shared Rust framework used to compose service runtime concerns across AIForAll. This page is the orientation hub; crate-level details live in `[[projects/rustycog/references/index]]`.

## Documentation Note

- Treat `rustycog/README.md` as historical context, not the canonical API map; crate-level source-of-truth behavior is maintained in `[[projects/rustycog/references/index]]` and linked per-crate pages.

## Canonical Scope

RustyCog currently has 11 documented crate surfaces:

- [[projects/rustycog/references/rustycog-core]] — shared error contracts (`ServiceError`, `DomainError`)
- [[projects/rustycog/references/rustycog-command]] — command execution runtime and registry
- [[projects/rustycog/references/rustycog-config]] — typed config models and loaders
- [[projects/rustycog/references/rustycog-db]] — DB pool and replica-aware read/write routing
- [[projects/rustycog/references/rustycog-events]] — event envelope plus Kafka/SQS/no-op adapters
- [[projects/rustycog/references/rustycog-http]] — Axum shell, route builder, auth/permission middleware
- [[projects/rustycog/references/rustycog-permission]] — permission primitives and Casbin engine
- [[projects/rustycog/references/rustycog-testing]] — integration-test fixtures and bootstrap helpers
- [[projects/rustycog/references/rustycog-server]] — health-check abstractions
- [[projects/rustycog/references/rustycog-logger]] — tracing/logging initialization helpers
- [[projects/rustycog/references/rustycog-meta]] — umbrella dependency package

## Documentation Ownership

- Per-crate API and behavior details: `[[projects/rustycog/references/index]]`
- Shared SDK vocabulary: `[[entities/index]]`
- Cross-crate architecture patterns: `[[concepts/shared-rust-microservice-sdk]]`
- Service-construction usage flow: `[[skills/building-rustycog-services]]`

## Scope Mismatches To Track

- README mentions `rustycog-macros` and examples that are not visible in the checked-in tree. ^[ambiguous]
- `rustycog-logger` is included in `rustycog-meta` but not listed as a root workspace member. ^[ambiguous]
- `rustycog-server` name suggests broader server bootstrap ownership, but current surface is health-only. ^[ambiguous]
- Multi-queue publishing intent exists in events APIs, but helper behavior is still partly single-publisher underneath. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]] — Inventory and scope boundaries for all crates
- [[references/platform-building-blocks]] — Shared SDK plus event-contract context
- [[concepts/shared-rust-microservice-sdk]] — Cross-project framing for the same stack
- [[skills/building-rustycog-services]] — Service composition workflow using these crates