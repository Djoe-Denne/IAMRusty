---
title: IAMRusty Service
category: references
tags: [reference, iam, architecture, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
  - IAMRusty/setup/src/app.rs
  - IAMRusty/http/src/lib.rs
summary: Code-backed overview of IAMRusty's crate layout, route surface, runtime composition, and shared rustycog dependencies.
provenance:
  extracted: 0.81
  inferred: 0.13
  ambiguous: 0.06
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T17:46:37.6929647Z
---

# IAMRusty Service

These sources define the overall shape of `[[projects/iamrusty/iamrusty]]`: the crate layout, the runtime composition path, and the routes the service exposes once its dependencies are wired.

## Key Ideas

- The service is split across domain, application, infrastructure, HTTP, configuration, setup, and migration crates, with the workspace leaning on shared `rustycog` libraries for commands, HTTP, config, DB, logging, and events.
- `setup/src/app.rs` is the key runtime assembly point, creating database pools, combined repositories, JWT and registration-token services, password adapters, queue-backed event publishing, use cases, and the final `GenericCommandService`.
- The HTTP route table includes public signup, login, verification, resend-verification, registration completion, password reset, OAuth login, callback, token refresh, and JWKS endpoints, plus authenticated profile, provider-token, link, relink, and authenticated reset behavior.
- The runtime builds separate OAuth and token-repository instances for login, provider linking, and internal provider-token operations, which keeps those flows isolated while still sharing domain abstractions.
- Event publishing is part of the service composition, not an afterthought: `create_multi_queue_event_publisher` is wired into auth, registration, and password-reset flows through `IAMErrorMapper`.
- The high-level docs and the current route table do not match perfectly; some documentation still describes `/start`-style endpoints and older callback assumptions that differ from the live `http/src/lib.rs` surface. ^[ambiguous]

## Open Questions

- `migration/` and `iam-events` are part of the wider runtime picture, but they were only indirectly inspected in this ingest batch. ^[ambiguous]
- `README.md` still references a missing `docs/TEST_DATABASE_GUIDE.md`, so the published service docs are incomplete relative to the repo. ^[ambiguous]

## Sources

- [[projects/iamrusty/iamrusty]] - Main project overview.
- [[concepts/hexagonal-architecture]] - Structural pattern behind the crate split.
- [[references/iamrusty-api-and-auth-flows]] - Route-level behavior and auth contracts.
- [[references/iamrusty-runtime-and-security]] - Runtime config, JWT, TLS, and queue context.
