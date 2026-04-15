---
title: RouteBuilder
category: entities
tags: [rustycog, http, routing, visibility/internal]
sources:
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/lib.rs
summary: RouteBuilder is RustyCog's fluent HTTP composition API for route wiring, auth modes, permission guards, and middleware layering.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RouteBuilder

`RouteBuilder` is the service-shell builder used in `[[projects/rustycog/references/rustycog-http]]`.

## Key Ideas

- It provides fluent route registration methods (`get`, `post`, `put`, `delete`, `patch`) plus nested routing support.
- It supports explicit auth modes (`authenticated` and `might_be_authenticated`) per route chain.
- Permission guarding is integrated via `permissions_dir`, `resource`, `with_permission_fetcher`, and `with_permission`.
- Build-time composition applies tracing middleware, panic catching, and correlation header propagation.
- It includes a standard `/health` endpoint hook for service liveness checks.

## Open Questions

- Missing permission model resources currently panic during setup, so failure handling is stricter than many service bootstrap paths expect. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-http]]
- [[entities/permissions-fetcher]]
- [[entities/resource-id]]
