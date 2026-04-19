---
title: Manifesto API and Permission Flows
category: references
tags: [reference, api, permissions, visibility/internal]
sources:
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/domain/src/service/permission_fetcher_service.rs
summary: Manifesto-specific API and authorization behavior on top of RustyCog's shared HTTP and permission layers, including route-level differences and current component-detail limits.
provenance:
  extracted: 0.84
  inferred: 0.08
  ambiguous: 0.08
created: 2026-04-14T20:25:00Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Manifesto API and Permission Flows

This page assumes the shared `[[projects/rustycog/references/rustycog-http]]` and `[[concepts/resource-scoped-permission-fetchers]]` patterns are already familiar. It keeps the route, command, and permission details that are specific to `[[projects/manifesto/manifesto]]`.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-http]]` explains `RouteBuilder`, authentication modes, command-context extraction, and permission middleware.
- `[[concepts/resource-scoped-permission-fetchers]]` explains the shared pattern of pairing route resources with dedicated fetchers.
- `[[projects/rustycog/references/rustycog-command]]` covers the shared command execution runtime that the handlers delegate into.

## Service-Specific Differences

- `http/src/lib.rs` splits the HTTP surface into three Manifesto-owned resource scopes in `RouteBuilder`: `project`, `component`, and `member`, each with its own `PermissionsFetcher`.
- Project list/get/detail routes are optionally authenticated reads, while create, update, delete, publish, and archive routes are authenticated and permission-guarded.
- Component routes are nested under project routes and layer `Permission::Read` or `Permission::Admin` on top of component-specific fetcher logic.
- Member routes include both membership CRUD and permission grant/revoke endpoints, including a generic resource form and a resource-specific form with `resource_id`.
- `ManifestoCommandRegistryFactory` is the command entrypoint behind the handlers, grouping project, component, and member operations into one registry consumed through `GenericCommandService`.
- `ProjectUseCaseImpl` bootstraps the owner member and grants owner permissions for `project`, `component`, and `member` during creation, so access control begins at the first successful project write.
- `ComponentUseCaseImpl` keeps permissions and component lifecycle synchronized by creating or deleting instance resources alongside add/remove operations.
- `MemberUseCaseImpl` enforces a strong rule during grants: a requester cannot grant a permission they do not already hold, and component-instance UUID resources can be granted through either exact or generic component authority.
- Project detail reads return component rows, but `ProjectDetailResponse` still leaves `endpoint` and `access_token` as `None`, so the API currently exposes attachment metadata rather than a ready-to-use component runtime handoff. ^[ambiguous]

## Open Questions

- The route layer models optional authentication on some reads, but service-level permission semantics still depend on fetcher and middleware behavior outside the Manifesto crate itself. ^[ambiguous]
- Member permission endpoints are rich enough to express generic and instance-specific resources, but the operator-facing API story for component-specific grants is still implicit in code rather than spelled out in dedicated docs. ^[ambiguous]

## Sources

- [[projects/manifesto/manifesto]] - Project-service overview.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Creation, ownership bootstrap, and publish/archive lifecycle.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission behavior.
- [[concepts/resource-scoped-permission-fetchers]] - Shared RouteBuilder plus PermissionsFetcher pattern used here.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted after these flows complete.
