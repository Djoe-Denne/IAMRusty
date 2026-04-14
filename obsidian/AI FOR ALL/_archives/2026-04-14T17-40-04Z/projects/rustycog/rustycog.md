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
summary: >-
  RustyCog is the shared Rust SDK/workspace that standardizes commands, config, HTTP, permissions, events, logging, DB access, and test infrastructure across services.
provenance:
  extracted: 0.82
  inferred: 0.12
  ambiguous: 0.06
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:13:01.1911009Z
---

# RustyCog

RustyCog is the shared platform SDK for AIForAll. It lives as a set of `rustycog-*` crates inside the main workspace, and `rustycog/Cargo.toml` also exposes a `rustycog-meta` package that bundles the stack for consumers that want one umbrella dependency. It is the concrete implementation of `[[concepts/shared-rust-microservice-sdk]]` for services such as `[[projects/iamrusty/iamrusty]]` and `[[projects/manifesto/manifesto]]`.

## Key Ideas

- The root workspace manifest treats most RustyCog crates as first-class members alongside application services, which makes the SDK part of the repo's normal build and dependency graph.
- The stack is intentionally split into focused crates: `rustycog-core` for shared errors, `rustycog-command` for registry-driven command execution, `rustycog-config` for typed config loading, `rustycog-db` for read/write pooling, `rustycog-http` for the Axum service shell, `rustycog-events` for Kafka/SQS/no-op publishing, `rustycog-permission` for authorization primitives, `rustycog-logger` for tracing setup, and `rustycog-testing` for reusable integration fixtures.
- `RouteBuilder` centers HTTP startup around `AppState`, `GenericCommandService`, `UserIdExtractor`, health routes, tracing, auth modes, and permission guards, so service setup stays consistent across projects.
- Queue transport is selected at runtime through `QueueConfig`, and the event layer can instantiate Kafka, SQS, or disabled/no-op publishers and consumers from the same abstraction.
- The test harness reuses global servers plus Kafka and LocalStack-backed queue fixtures so services can verify real infrastructure paths without hand-rolling their own container orchestration.
- The README still advertises `rustycog-macros` and example projects that are not present in this tree, so the public narrative is slightly ahead of the checked-in code surface. ^[ambiguous]

## Open Questions

- The wiki still does not catalog which services depend on which crate subset in production.
- `rustycog-logger` is included in `rustycog-meta`, but it is not listed in the root workspace members, so its packaging story is not fully explicit. ^[ambiguous]
- The current tree shows crate APIs clearly, but it does not define which surfaces should be treated as stable public SDK contracts. ^[ambiguous]

## Sources

- [[references/rustycog-crate-catalog]] — Code-backed map of the crate surfaces
- [[references/platform-building-blocks]] — Shared SDK and event-contract foundation
- [[concepts/shared-rust-microservice-sdk]] — Cross-project abstraction implemented here
- [[skills/building-rustycog-services]] — Practical workflow derived from that guidance
