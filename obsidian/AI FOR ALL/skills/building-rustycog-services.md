---
title: >-
  Building RustyCog Services
category: skills
tags: [rustycog, scaffolding, services, visibility/internal]
sources:
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-logger/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
summary: >-
  Stepwise workflow for scaffolding a RustyCog-based service with typed config, logging, DB wiring, command execution, RouteBuilder routes, permissions, and tests.
provenance:
  extracted: 0.88
  inferred: 0.08
  ambiguous: 0.04
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Building RustyCog Services

Use this page when starting a new service that should look like `[[projects/manifesto/manifesto]]` and build on `[[projects/rustycog/rustycog]]`.

## Workflow

- Start with one vertical slice across `domain`, `application`, `infra`, `http`, `setup`, `configuration`, and `tests` rather than scaffolding everything at once.
- Decide whether the service should depend on specific `rustycog-*` crates or the `rustycog-meta` umbrella package, then keep that choice aligned with the workspace manifest.
- Define typed config first using the `[[concepts/structured-service-configuration]]` pattern, then initialize logging once at startup with `setup_logging`.
- Create one `DbConnectionPool`, split read and write repositories correctly, and wire concrete dependencies inside the setup composition root.
- Register commands through the `[[concepts/command-registry-and-retry-policies]]` approach, then wrap the registry in `GenericCommandService` so handlers stay behind one execution surface.
- Create `AppState` with the command service and a `UserIdExtractor`, then use `RouteBuilder` so tracing, panic handling, correlation IDs, and the `/health` endpoint stay standardized.
- For protected routes, set `permissions_dir`, `resource`, and `with_permission_fetcher` before calling `with_permission`, and choose between `authenticated()` and `might_be_authenticated()` intentionally.
- Finish the slice with integration tests that exercise auth, permissions, validation, and the happy path, then add Kafka or LocalStack-backed checks only when transport behavior is part of the contract.

## Common Pitfalls

- Letting `command_type()` strings drift away from registration keys.
- Mixing `AuthUser` and `OptionalAuthUser` with the wrong route mode.
- Assuming `config/default.toml` is always merged automatically.
- Declaring retry or logging config without verifying the current runtime path actually consumes it.
- Forgetting that the permission middleware only extracts UUID-shaped path segments as resource IDs.
- Expecting README-level macros or example projects that are not present in the current tree. ^[ambiguous]

## Sources

- [[references/rustycog-service-construction]] — Combined source summary for this workflow
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the crates this workflow wires together
- [[concepts/shared-rust-microservice-sdk]] — Broader platform motivation for the approach
- [[concepts/hexagonal-architecture]] — Architectural contract the workflow preserves