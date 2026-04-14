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
summary: Code-backed map of Manifesto's route surface, command entrypoints, and permission behaviors for project, component, and member flows.
provenance:
  extracted: 0.84
  inferred: 0.08
  ambiguous: 0.08
created: 2026-04-14T20:25:00Z
updated: 2026-04-14T20:25:00Z
---

# Manifesto API and Permission Flows

This page focuses on the live application behavior behind `[[projects/manifesto/manifesto]]`: which routes exist, which command/use-case flows back them, and how permissions are enforced once requests cross the HTTP boundary.

## Key Ideas

- `http/src/lib.rs` splits the HTTP surface into three resource scopes in `RouteBuilder`: `project`, `component`, and `member`, each with its own `PermissionsFetcher`.
- Project list/get/detail routes are written as optionally authenticated reads, while create/update/delete/publish/archive routes are authenticated and permission-guarded.
- Component routes are nested under project routes and use `Permission::Read` or `Permission::Admin` on top of component-specific fetcher logic.
- Member routes include both membership CRUD and permission-grant/revoke endpoints, including a generic resource form and a resource-specific form with `resource_id`.
- `ManifestoCommandRegistryFactory` is the command entrypoint behind the handlers, grouping project, component, and member operations into one registry consumed through `GenericCommandService`.
- `ProjectUseCaseImpl` bootstraps the owner member and grants owner permissions for `project`, `component`, and `member` during creation, so access control begins at the first successful project write.
- `ComponentUseCaseImpl` keeps permissions and component lifecycle synchronized by creating or deleting instance resources alongside add/remove operations.
- `MemberUseCaseImpl` enforces a strong rule during grants: a requester cannot grant a permission they do not already hold, and component-instance UUID resources can be granted through either exact or generic component authority.

## Open Questions

- The route layer models optional authentication on some reads, but service-level permission semantics still depend on fetcher and middleware behavior outside the Manifesto crate itself. ^[ambiguous]
- Member permission endpoints are rich enough to express generic and instance-specific resources, but the operator-facing API story for component-specific grants is still implicit in code rather than spelled out in dedicated docs. ^[ambiguous]

## Sources

- [[projects/manifesto/manifesto]] - Project-service overview.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Creation, ownership bootstrap, and publish/archive lifecycle.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission behavior.
- [[concepts/resource-scoped-permission-fetchers]] - Shared RouteBuilder plus PermissionsFetcher pattern used here.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted after these flows complete.
