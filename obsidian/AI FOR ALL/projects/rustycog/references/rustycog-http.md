---
title: RustyCog HTTP
category: references
tags: [reference, rustycog, http, visibility/internal]
sources:
  - rustycog/rustycog-http/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/error.rs
  - rustycog/rustycog-http/src/extractors.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-http/src/tracing_middleware.rs
  - rustycog/rustycog-http/src/jwt_handler.rs
summary: rustycog-http provides the Axum service shell, including TLS/HTTP startup branching, RouteBuilder auth/permission wiring, validated JSON extraction, and tracing/error helpers.
provenance:
  extracted: 0.9
  inferred: 0.05
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-19T10:59:36Z
---

# RustyCog HTTP

`rustycog-http` is the service-shell crate that wires routes, middleware, auth extraction, permissions, and health checks for `[[projects/rustycog/rustycog]]` services.

## Key Ideas

- `AppState` packages the shared `GenericCommandService` and user-id extraction infrastructure for handlers.
- `RouteBuilder` gives a fluent setup API for routes, auth mode (`authenticated` or `might_be_authenticated`), permission guards, middleware, and `/health`.
- `RouteBuilder::build()` chooses HTTPS (`axum_server` + rustls cert/key paths + `tls_port`) when `ServerConfig.tls_enabled` is true, otherwise starts plain HTTP on `port`.
- `ValidatedJson<T>` enforces body validation through the `validator` crate and emits uniform `ValidationError` responses for malformed or invalid payloads.
- `GenericHttpError` and `ValidationError` normalize API error envelopes (`error_code`, `message`, `status`) so handlers can return consistent JSON errors.
- Tracing middleware standardizes `x-correlation-id` and `x-request-id` behavior and exposes helper accessors (`get_correlation_id`, `get_request_id`) for downstream logging.
- Permission middleware extracts UUID path segments into ordered `ResourceId` values and delegates authorization checks to the configured `PermissionsFetcher`.

## Linked Entities

- [[entities/route-builder]]
- [[entities/permissions-fetcher]]
- [[entities/resource-id]]

## Open Questions

- The crate includes an `optional_permission_middleware`, but its current behavior still rejects requests without user identity, so optional-auth semantics are not fully relaxed. ^[ambiguous]
- Missing permission model paths currently panic at runtime in builder methods instead of producing recoverable startup errors. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[concepts/resource-scoped-permission-fetchers]]
- [[projects/rustycog/rustycog]]
