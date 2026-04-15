---
title: PermissionsFetcher
category: entities
tags: [rustycog, permissions, authorization, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/casbin.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: PermissionsFetcher is the service extension point that resolves effective permissions for a user/resource scope before policy enforcement.
provenance:
  extracted: 0.89
  inferred: 0.05
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# PermissionsFetcher

`PermissionsFetcher` is the service-owned permission resolver contract used by RustyCog authorization middleware.

## Key Ideas

- Route middleware extracts user and `ResourceId` scope, then delegates effective-permission lookup to the fetcher.
- Fetchers allow each service to encode domain-specific interpretation of resource IDs and role data.
- `CasbinPermissionEngine` consumes fetched permissions, expands hierarchical actions, and evaluates final allow/deny decisions.
- This split keeps RustyCog generic while allowing business authorization logic to stay in each service domain.

## Open Questions

- Resource-ID semantics remain intentionally flexible, which improves reuse but can cause cross-service inconsistency without explicit conventions. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-http]]
- [[concepts/resource-scoped-permission-fetchers]]
