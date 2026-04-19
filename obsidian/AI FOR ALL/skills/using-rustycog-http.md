---
title: Using RustyCog HTTP
category: skills
tags: [rustycog, http, skills, visibility/internal]
sources:
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/lib.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: Step-by-step guide for assembling Axum services with RouteBuilder, auth modes, permission guards, and shared middleware.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Using RustyCog HTTP

Use this guide when wiring `<!-- [[projects/rustycog/references/rustycog-http]] -->`.

## Workflow

- Build `AppState` with your command service and user-id extractor.
- Compose routes through `RouteBuilder` and choose auth mode per route chain.
- Configure permission-protected routes in this order: `permissions_dir` -> `resource` -> `with_permission_fetcher` -> `with_permission`.
- Keep health endpoint and tracing/correlation middleware in the standard builder path.
- Call `build(server_config)` once after all routes are registered.

## Common Pitfalls

- Applying `with_permission` before setting resource and fetcher context.
- Using optional-auth mode while expecting fully public behavior from permission middleware.
- Letting permission model path mistakes panic at startup instead of validating early.

## Sources

- <!-- [[projects/rustycog/references/rustycog-http]] -->
- <!-- [[entities/route-builder]] -->
- <!-- [[concepts/resource-scoped-permission-fetchers]] -->
