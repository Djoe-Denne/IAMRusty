---
title: Resource-Scoped Permission Fetchers
category: concepts
tags: [authorization, permissions, rust, visibility/internal]
sources:
  - Manifesto/domain/src/service/permission_fetcher_service.rs
  - Manifesto/http/src/lib.rs
  - Hive/domain/src/service/permission_service.rs
  - Hive/http/src/lib.rs
  - Telegraph/domain/src/service/permission_service.rs
  - Telegraph/http/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: RustyCog services pair RouteBuilder resource guards with domain-backed PermissionsFetcher implementations that translate path resource IDs into effective permissions.
provenance:
  extracted: 0.83
  inferred: 0.07
  ambiguous: 0.10
created: 2026-04-14T20:25:00Z
updated: 2026-04-14T20:25:00Z
---

# Resource-Scoped Permission Fetchers

Across `[[projects/manifesto/manifesto]]`, `[[projects/hive/hive]]`, and `[[projects/telegraph/telegraph]]`, route authorization is not just a static route-level ACL. `rustycog_http::RouteBuilder` provides the shell, but each service supplies domain-backed `PermissionsFetcher` implementations that turn the current route's resource IDs into effective permissions.

## Key Ideas

- The common RouteBuilder pattern is: set `permissions_dir`, declare a `resource(...)`, attach a `with_permission_fetcher(...)`, then apply `with_permission(Permission::...)` on guarded routes.
- `permission_middleware` extracts every UUID-shaped path segment into ordered `ResourceId` values and passes that list to the current fetcher.
- Manifesto specializes the pattern with separate project, member, and component fetchers; its component fetcher can combine generic `component` permissions with instance-specific UUID permissions.
- Hive uses organization-backed fetchers tied to persisted organization members, roles, permissions, and resources, so the route guard resolves through domain data rather than fixed role strings.
- Telegraph uses the same interface more narrowly: its notification fetcher grants `Write` when the requesting user owns one of the notification IDs in scope.
- The shared pattern is therefore "static model file plus dynamic domain lookup," not "Casbin policy file contains the whole authorization story."
- The services do not share one exact interpretation contract for `resource_ids`: some fetchers anchor on the first ID and ignore extras, while Manifesto's component fetcher reads the first two IDs as project and component scope. ^[ambiguous]
- `RouteBuilder` exposes both `authenticated()` and `might_be_authenticated()`, but permission-protected routes still depend on auth context being present when the middleware runs. Conflict to resolve. ^[ambiguous]

## Open Questions

- Should RustyCog standardize a stronger cross-service convention for how many `resource_ids` a fetcher is expected to interpret? ^[inferred]
- Should optional-auth permission routes gain a distinct builder path so public-read patterns are less dependent on service-specific workarounds? ^[ambiguous]

## Sources

- [[projects/manifesto/concepts/component-instance-permissions]] - Manifesto's generic-plus-instance permission model.
- [[projects/hive/concepts/organization-resource-authorization]] - Hive's organization-scoped specialization of the same pattern.
- [[projects/telegraph/telegraph]] - Telegraph's narrower notification-ownership usage of the shared contract.
- [[projects/rustycog/rustycog]] - SDK project where RouteBuilder and permission middleware live.
