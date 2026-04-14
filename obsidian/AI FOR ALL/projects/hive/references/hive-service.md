---
title: Hive Service
category: references
tags: [reference, organizations, architecture, visibility/internal]
sources:
  - Hive/Cargo.toml
  - Hive/setup/src/app.rs
  - Hive/http/src/lib.rs
  - Hive/application/src/command/factory.rs
  - Hive/configuration/src/lib.rs
summary: Code-backed overview of Hive's crate layout, runtime wiring, event publishing, and organization-management service boundaries.
provenance:
  extracted: 0.80
  inferred: 0.12
  ambiguous: 0.08
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Hive Service

These sources define the overall shape of `[[projects/hive/hive]]`: the crate layout, runtime composition, route surface, and shared platform dependencies that make Hive an organization-management service rather than just a set of handlers.

## Key Ideas

- `Hive/Cargo.toml` describes Hive as the organization management service for AIForAll and wires together domain, application, infra, HTTP, configuration, setup, migration, `[[projects/hive-events/hive-events]]`, and shared `rustycog` crates.
- `setup/src/app.rs` is the composition root: it creates the DB pool, event publisher, repositories, external provider client, domain services, permission fetchers, use cases, command registry, and final `AppState`.
- The HTTP server is built through `rustycog_http::RouteBuilder`, while use cases publish outbound `HiveDomainEvent` values through a `MultiQueueEventPublisher`, so Hive is both an HTTP service and an event-producing integration point.
- Hive's route table is smaller than its command registry, and the command registry is smaller than the OpenAPI contract, which makes “what exists in code” depend on whether you ask the router, the handlers, or the spec. ^[ambiguous]
- Unlike `[[projects/telegraph/telegraph]]`, Hive is HTTP-first at runtime and does not run a queue consumer loop in parallel.
- Unlike the current `[[projects/iamrusty/iamrusty]]` documentation, Hive's local docs are sparse: there is no service README in the `Hive/` tree, so Cargo metadata, OpenAPI, config, and code are doing most of the documentation work. ^[ambiguous]

## Open Questions

- The service boundaries are clear, but the repo still lacks a concise Hive-local operator guide comparable to the stronger docs around IAMRusty or the Manifesto guides.
- The exact boundary between shipped API and aspirational API is still split across `openspecs.yaml`, handlers, and route registration. ^[ambiguous]

## Sources

- [[projects/hive/hive]] - Main project overview.
- [[projects/hive/references/hive-runtime-and-configuration]] - Typed config, event publisher, and environment behavior.
- [[projects/hive/references/hive-http-api-and-openapi-drift]] - Route-level view of the live HTTP surface.
- [[projects/hive/references/hive-command-execution]] - Registry and event-publishing use case coverage.