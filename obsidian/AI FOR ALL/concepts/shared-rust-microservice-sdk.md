---
title: >-
  Shared Rust Microservice SDK
category: concepts
tags: [sdk, rust, platform, visibility/internal]
sources:
  - rustycog/README.md
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-core/src/error.rs
  - rustycog/rustycog-command/src/lib.rs
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-db/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
  - rustycog/rustycog-testing/src/lib.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  RustyCog is the shared SDK stack for platform services, but its umbrella package, workspace membership, and builder-level ergonomics do not all line up perfectly.
provenance:
  extracted: 0.78
  inferred: 0.08
  ambiguous: 0.14
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-15T22:10:00Z
---

# Shared Rust Microservice SDK

`[[projects/rustycog/rustycog]]` is the shared SDK layer for service runtime concerns in AIForAll. This page captures the architectural idea; crate-level details live in `[[projects/rustycog/references/index]]`.

## Key Ideas

- The SDK is split by concern (errors, command runtime, config, DB, events, HTTP shell, permissions, logging, tests) so services compose only needed parts without redefining runtime primitives.
- RustyCog standardizes composition seams, not business logic: services still own domain models, fetchers, handlers, route sets, and policy choices.
- Shared entities (`ServiceError`, `CommandRegistry`, `QueueConfig`, `RouteBuilder`, `PermissionsFetcher`, `DomainEvent`, and others) are documented in `[[entities/index]]` as a common vocabulary for service docs.
- Cross-service consistency comes from repeating the same integration boundaries (config -> command -> HTTP -> permissions/events/testing), not from one monolithic starter template.
- `rustycog-meta` and per-crate dependencies are two consumption modes over the same stack; teams trade onboarding speed vs explicit dependency control. ^[inferred]

## Open Questions

- The wiki still lacks a service-by-service adoption matrix for the crate set.
- Stable vs evolving RustyCog surfaces are not yet marked explicitly for consumers.
- Some packaging and runtime edges remain ambiguous (`rustycog-logger` workspace membership, `rustycog-server` scope, README macro references). ^[ambiguous]

## Sources

- [[projects/rustycog/rustycog]] — SDK hub and ownership boundaries
- [[projects/rustycog/references/index]] — Canonical crate inventory
- [[projects/rustycog/references/index]] — Detailed per-crate behavior
- [[references/rustycog-service-construction]] — Manifesto guide-to-runtime construction analysis
- [[skills/building-rustycog-services]] — Practical service assembly workflow