---
title: Hexagonal Architecture
category: concepts
tags: [architecture, hexagonal, ddd, visibility/internal]
sources:
  - IAMRusty/docs/ARCHITECTURE.md
  - IAMRusty/setup/src/app.rs
  - IAMRusty/http/src/lib.rs
summary: IAMRusty keeps domain rules, use-case orchestration, adapters, and runtime composition separate so auth behavior stays testable and replaceable.
provenance:
  extracted: 0.79
  inferred: 0.16
  ambiguous: 0.05
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T17:46:37.6929647Z
---

# Hexagonal Architecture

`[[projects/iamrusty/iamrusty]]` uses a hexagonal layout where domain services and ports stay isolated from HTTP, database, provider, and queue adapters. The codebase extends the classic four-layer story with explicit setup and configuration crates that own composition and runtime policy.

## Key Ideas

- The domain crate owns entities, ports, and business services such as OAuth, provider linking, token issuance, and email/password auth.
- The application crate wraps those services in use cases and typed commands, while `http/src/lib.rs` exposes the resulting behavior through validated routes and a shared `RouteBuilder`.
- The infrastructure crate implements repositories, OAuth clients, JWT encoders, password adapters, and event adapters, keeping external concerns out of the domain layer.
- `setup/src/app.rs` is the composition root: it creates database pools, combined repositories, token services, provider clients, the queue-backed event publisher, and the `GenericCommandService`.
- The runtime uses separate OAuth and token-repository instances for login, provider linking, and internal provider-token retrieval, which keeps flows isolated even when they share the same domain abstractions.
- Shared `rustycog` crates provide the HTTP, command, config, logging, database, and event primitives the service is built on.

## Open Questions

- The architecture guide mostly describes four layers, but the current repo also treats configuration and setup as first-class crates with their own runtime responsibilities. ^[ambiguous]
- Some doc examples still show older route names and DTO shapes, so not every example in the architecture docs matches the live HTTP surface exactly. ^[ambiguous]

## Sources

- [[projects/iamrusty/iamrusty]] - Service where the pattern is applied concretely.
- [[references/iamrusty-service]] - Crate map and composition-root wiring.
- [[concepts/structured-service-configuration]] - Runtime config layer that supports the architecture in practice.
- [[concepts/command-registry-and-retry-policies]] - Cross-cutting orchestration built on top of the layered design.
