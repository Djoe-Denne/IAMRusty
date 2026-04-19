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
updated: 2026-04-15T22:10:00Z
---

# PermissionsFetcher

`PermissionsFetcher` is the service-owned permission resolver contract used by RustyCog authorization middleware.

## Key Ideas

- `PermissionsFetcher` is the service-owned adapter that resolves effective permissions for `(user, resource scope)`.
- RustyCog middleware extracts `ResourceId` values from routes, then delegates business-specific authorization lookup to this interface.
- `CasbinPermissionEngine` consumes fetched permissions to enforce final allow/deny decisions.
- This split keeps RustyCog reusable while each service preserves domain-specific authorization semantics.

## Sources

- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-http]]
- [[concepts/resource-scoped-permission-fetchers]]
