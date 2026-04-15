---
title: >-
  RustyCog Crate Catalog
category: references
tags: [reference, rustycog, sdk, visibility/internal]
sources:
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-core/src/error.rs
  - rustycog/rustycog-command/src/lib.rs
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-db/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-events/src/event.rs
  - rustycog/rustycog-http/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-http/src/jwt_handler.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
  - rustycog/rustycog-testing/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
  - rustycog/rustycog-server/src/health.rs
summary: >-
  Code-backed inventory of the current RustyCog crates, their main responsibilities, and a few packaging gaps between the README and the checked-in tree.
provenance:
  extracted: 0.88
  inferred: 0.06
  ambiguous: 0.06
created: 2026-04-14T17:13:01.1911009Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Crate Catalog

This reference maps the current code surfaces behind `[[projects/rustycog/rustycog]]` so the higher-level guidance in `[[concepts/shared-rust-microservice-sdk]]` and `[[skills/building-rustycog-services]]` can stay grounded in the actual crates.

## Key Ideas

- The root workspace manifest includes most RustyCog crates directly, while `rustycog/Cargo.toml` also exposes a `rustycog-meta` package that groups them behind one umbrella dependency.
- The codebase separates cross-cutting concerns by crate instead of one monolith, which keeps service composition explicit.

## Crate Map

- [[projects/rustycog/references/rustycog-core]] provides `ServiceError` and `DomainError`, including category/status helpers and conversion from domain to service errors.
- [[projects/rustycog/references/rustycog-command]] provides `Command`, `CommandHandler`, `CommandContext`, `RetryPolicy`, `CommandRegistry`, and `GenericCommandService`.
- [[projects/rustycog/references/rustycog-config]] provides typed config structs plus generic loaders, caching hooks, env-prefix handling, and queue/logging abstractions.
- [[projects/rustycog/references/rustycog-db]] provides `DbConnectionPool` with one write connection, optional read replicas, round-robin reads, and fallback to primary.
- [[projects/rustycog/references/rustycog-events]] provides `DomainEvent`, publisher/consumer traits, Kafka/SQS/no-op factories, and transport selection/fallback logic.
- [[projects/rustycog/references/rustycog-http]] provides `AppState`, `RouteBuilder`, auth/optional-auth middleware, permission middleware, request tracing, panic handling, validated JSON, and a health endpoint.
- [[projects/rustycog/references/rustycog-permission]] provides `Permission`, `ResourceId`, `PermissionEngine`, and `PermissionsFetcher` for Casbin-backed authorization.
- [[projects/rustycog/references/rustycog-logger]] provides `setup_logging` with env filters and optional Scaleway Loki support.
- [[projects/rustycog/references/rustycog-testing]] re-exports DB/events/HTTP/wiremock helpers and includes a shared test server plus Kafka and LocalStack SQS fixtures.
- [[projects/rustycog/references/rustycog-server]] currently exposes health-checker primitives.
- [[projects/rustycog/references/rustycog-meta]] provides one umbrella dependency over the crate set.
- The root README still mentions `rustycog-macros` and examples that do not appear in the current tree. ^[ambiguous]
- `rustycog-logger` is part of `rustycog-meta` but not listed as a root workspace member. ^[ambiguous]
- `rustycog-server` naming still implies broader server bootstrap scope than the current health-only API. ^[ambiguous]
- `rustycog-events` multi-queue helper currently builds one underlying publisher instance. ^[ambiguous]
- `rustycog-config` SQS endpoint semantics still mix AWS vocabulary and Scaleway URL construction. ^[ambiguous]

## Open Questions

- Which of these crates are intended as stable public API versus internal implementation detail?
- Where, if anywhere, do the README-promised macros live today? ^[ambiguous]

## Sources

- [[projects/rustycog/rustycog]] — Project-level summary of the SDK
- [[concepts/shared-rust-microservice-sdk]] — Higher-level platform abstraction
- [[concepts/command-registry-and-retry-policies]] — Command runtime details
- [[concepts/structured-service-configuration]] — Config and queue model details
- [[concepts/event-driven-microservice-platform]] — Transport and event model context
- [[entities/index]] — Shared technical entity glossary extracted from these crates