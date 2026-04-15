---
title: RustyCog HTTP
category: references
tags: [reference, rustycog, http, visibility/internal]
sources:
  - rustycog/rustycog-http/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-http/src/jwt_handler.rs
summary: rustycog-http provides the Axum app shell, RouteBuilder composition API, auth middleware, permission middleware, and request tracing helpers.
provenance:
  extracted: 0.89
  inferred: 0.05
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog HTTP

`rustycog-http` is the service-shell crate that wires routes, middleware, auth extraction, permissions, and health checks for `[[projects/rustycog/rustycog]]` services.

## Key Ideas

- `AppState` packages the shared `GenericCommandService` and user-id extraction infrastructure for handlers.
- `RouteBuilder` gives a fluent setup API for routes, auth mode (`authenticated` or `might_be_authenticated`), permission guards, middleware, and `/health`.
- Permission middleware extracts UUID path segments into ordered `ResourceId` values and delegates authorization checks to the configured `PermissionsFetcher`.
- The builder layers tracing, panic catching, and correlation-id propagation to keep HTTP concerns standardized across services.
- Route-level permissions are model-file driven (`permissions_dir` + `resource`) and wired with service-provided fetchers.

## Linked Entities

- [[entities/route-builder]]
- [[entities/permissions-fetcher]]
- [[entities/resource-id]]

## Open Questions

- The crate includes an `optional_permission_middleware`, but its current behavior still rejects requests without user identity, so optional-auth semantics are not fully relaxed. ^[ambiguous]
- Missing permission model paths currently panic at runtime in builder methods instead of producing recoverable startup errors. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[concepts/resource-scoped-permission-fetchers]]
- [[projects/rustycog/rustycog]]
