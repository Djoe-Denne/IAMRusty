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
updated: 2026-04-15T22:10:00Z
---

# RouteBuilder

`RouteBuilder` is the service-shell builder used in `[[projects/rustycog/references/rustycog-http]]`.

## Key Ideas

- `RouteBuilder` is the fluent HTTP composition boundary used by RustyCog services.
- It combines route registration, auth mode selection, permission guards, and shared middleware layering.
- It standardizes operational HTTP concerns (tracing, panic handling, correlation IDs, health endpoint wiring).
- Permission flow relies on `PermissionsFetcher` + `ResourceId` contracts rather than hardcoded route ACLs.

## Sources

- [[projects/rustycog/references/rustycog-http]]
- [[entities/permissions-fetcher]]
- [[entities/resource-id]]
