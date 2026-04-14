---
title: >-
  RustyCog Service Construction Guides
category: references
tags: [reference, rustycog, architecture, visibility/internal]
sources:
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
summary: >-
  Source summary for the Manifesto-authored RustyCog build guides, checked against the current command, config, HTTP, permission, and logging crates.
provenance:
  extracted: 0.82
  inferred: 0.08
  ambiguous: 0.10
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:13:01.1911009Z
---

# RustyCog Service Construction Guides

These guides use `[[projects/manifesto/manifesto]]` as the reference implementation for building on top of `[[projects/rustycog/rustycog]]`. Together they are the best current map for `[[skills/building-rustycog-services]]`.

## Key Ideas

- Service construction still follows the fixed order described by the guides: typed config, logging, DB pool, repositories, domain services, command registry, app state, routes, then tests.
- The composition root still lives outside domain and HTTP layers, and the current code confirms that setup is where logging, retry policy, permissions, and transport choices are wired together.
- `RouteBuilder` keeps the HTTP shell consistent by attaching tracing, panic handling, correlation ID propagation, health checks, auth modes, and permission middleware around handlers.
- Permission-protected routes still require explicit model files plus a `PermissionsFetcher`, which matches the guide's emphasis on keeping authorization configuration visible in the setup layer.
- The guides show how config, command, DB, HTTP, permission, and testing crates fit together as one stack, and the current code still reflects that shape.
- Some guide-era ergonomics, especially around automatic error mapping/macros, are not visible in the checked-in crate tree even though the higher-level docs still reference them. ^[ambiguous]

## Open Questions

- The guides are Manifesto-centric, so the degree to which every other service follows them exactly is not fully cataloged.
- The current docs clarify loader behavior well, but there is still no single cross-service compatibility matrix for crate adoption.
- The checked-in code does not show where the README-promised macros live today, so the full intended ergonomics are still somewhat unclear. ^[ambiguous]

## Sources

- [[concepts/shared-rust-microservice-sdk]] — Shared crate ecosystem captured by these guides
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the crates the guides compose
- [[concepts/hexagonal-architecture]] — Service-boundary pattern the guides enforce
- [[skills/building-rustycog-services]] — Practical workflow derived from this material
