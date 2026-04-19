---
title: RustyCog Permission
category: references
tags: [reference, rustycog, permissions, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/casbin.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: rustycog-permission defines permission/resource primitives and the Casbin-backed engine used with service-owned permission fetchers.
provenance:
  extracted: 0.87
  inferred: 0.06
  ambiguous: 0.07
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Permission

`rustycog-permission` is the authorization core behind permission-guarded routes in `[[projects/rustycog/references/rustycog-http]]`.

## Key Ideas

- The crate defines core permission primitives: `Permission` (`read`, `write`, `admin`, `owner`) and typed `ResourceId`.
- `PermissionEngine` is the service-agnostic trait used by middleware and route guards.
- `PermissionsFetcher` is the service extension point that resolves effective permissions for a user and resource scope.
- `CasbinPermissionEngine` builds a per-request enforcer, injects derived policies from fetcher results, and then evaluates required permission checks.
- Hierarchical permission expansion (`owner -> admin -> write -> read`) is handled in engine policy injection logic.

## Linked Entities

- [[entities/permissions-fetcher]]
- [[entities/resource-id]]

## Open Questions

- Enforcer construction is currently per request, so long-term performance strategy (caching versus strict per-request freshness) is not documented. ^[ambiguous]
- Resource-ID interpretation is intentionally delegated to services, so cross-service consistency still depends on local fetcher implementation. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[concepts/resource-scoped-permission-fetchers]]
- [[projects/rustycog/rustycog]]
