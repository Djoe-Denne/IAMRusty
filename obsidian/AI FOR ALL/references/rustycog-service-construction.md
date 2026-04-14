---
title: >-
  RustyCog Service Construction Guides
category: references
tags: [reference, rustycog, architecture, visibility/internal]
sources:
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - Manifesto/src/main.rs
  - Manifesto/configuration/src/lib.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/command/factory.rs
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
summary: >-
  Manifesto-authored RustyCog build guides checked against current loader, routing, logging, command, and runtime behavior, preserving guide-vs-code drift as explicit conflicts.
provenance:
  extracted: 0.73
  inferred: 0.07
  ambiguous: 0.20
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T20:08:52.0803248Z
---

# RustyCog Service Construction Guides

These guides use `[[projects/manifesto/manifesto]]` as the reference implementation for building on top of `[[projects/rustycog/rustycog]]`. Together they are still the best current map for `[[skills/building-rustycog-services]]`, but they now need to be read alongside the live Manifesto runtime rather than as a perfect description of it.

## Key Ideas

- Service construction still follows the fixed order described by the guides: typed config, logging, DB pool, repositories, domain services, command registry, app state, routes, then tests.
- The composition root still lives outside domain and HTTP layers, and the current code confirms that setup is where DB, command, permission, and transport choices are wired together.
- `RouteBuilder` keeps the HTTP shell consistent by attaching tracing, panic handling, correlation ID propagation, health checks, auth modes, and permission middleware around handlers.
- Permission-protected routes still require explicit model files plus a `PermissionsFetcher`, which matches the guide's emphasis on keeping authorization configuration visible in the setup layer.
- The guides show how config, command, DB, HTTP, permission, and testing crates fit together as one stack, and the current code still reflects that shape.
- Manifesto's implementation guide is explicit that some documented knobs are currently inert: `service.component_service.timeout_seconds` is defined but setup hardcodes `30`, and `[command.retry]` is documented in TOML without a `command` field in `ManifestoConfig`. Conflict to resolve. ^[ambiguous]
- The guides present `setup_logging(&config)` and layered config usage as the canonical path, while `Manifesto/src/main.rs` initializes tracing directly and the live loader does not auto-merge `config/default.toml`. Conflict to resolve. ^[ambiguous]
- Some guide-era ergonomics, especially around automatic error mapping/macros and a fully unified “reference service” story, are not visible in the checked-in tree even though the higher-level docs still reference them. ^[ambiguous]

## Open Questions

- The guides are Manifesto-centric, so the degree to which every other service follows them exactly is not fully cataloged.
- The current docs clarify loader behavior well, but there is still no single cross-service compatibility matrix for crate adoption.
- Which logging, loader, and retry story should be treated as canonical for RustyCog services: the guide recommendation or the current Manifesto implementation? Conflict to resolve. ^[ambiguous]
- The checked-in code does not show where the README-promised macros live today, so the full intended ergonomics are still somewhat unclear. ^[ambiguous]

## Sources

- [[concepts/shared-rust-microservice-sdk]] — Shared crate ecosystem captured by these guides
- [[projects/rustycog/references/rustycog-crate-catalog]] — Code-backed inventory of the crates the guides compose
- [[projects/iamrusty/concepts/hexagonal-architecture]] — Service-boundary pattern the guides enforce
- [[skills/building-rustycog-services]] — Practical workflow derived from this material