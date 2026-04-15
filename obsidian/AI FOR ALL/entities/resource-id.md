---
title: ResourceId
category: entities
tags: [rustycog, permissions, identifiers, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: ResourceId is the typed UUID wrapper RustyCog uses to pass route-scoped resources into permission fetchers and engines.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# ResourceId

`ResourceId` is a thin UUID wrapper that standardizes resource identity for permission checks.

## Key Ideas

- Permission middleware extracts UUID path segments and converts them into ordered `ResourceId` values.
- The type prevents ad hoc stringly-typed IDs in permission APIs.
- `PermissionsFetcher` implementations decide how to interpret one or multiple `ResourceId` values for their domain.
- The same wrapper is used across HTTP middleware and Casbin permission engine flows.

## Open Questions

- Cross-service conventions for multi-resource route layouts are still not fully standardized. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-http]]
- [[entities/permissions-fetcher]]
