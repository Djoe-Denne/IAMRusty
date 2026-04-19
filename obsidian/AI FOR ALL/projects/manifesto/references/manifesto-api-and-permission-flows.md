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
summary: >-
  Manifesto-specific API and authorization behavior on top of RustyCog's shared HTTP and
  permission layers, including optional-auth public reads and project-list visibility filtering.
provenance:
  extracted: 0.89
  inferred: 0.07
  ambiguous: 0.04
created: 2026-04-14T20:25:00Z
updated: 2026-04-19T18:00:00Z
---

# Manifesto API and Permission Flows

This page assumes the shared `[[projects/rustycog/references/rustycog-http]]` and `[[concepts/resource-scoped-permission-fetchers]]` patterns are already familiar. It keeps the route, command, and permission details that are specific to `[[projects/manifesto/manifesto]]`.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-http]]` explains `RouteBuilder`, authentication modes, command-context extraction, and permission middleware.
- `[[concepts/resource-scoped-permission-fetchers]]` explains the shared pattern of pairing route resources with dedicated fetchers.
- `[[projects/rustycog/references/rustycog-command]]` covers the shared command execution runtime that the handlers delegate into.

## Service-Specific Differences

- `http/src/lib.rs` splits the HTTP surface into three Manifesto-owned resource scopes in `RouteBuilder`: `project`, `component`, and `member`, each with its own `PermissionsFetcher`.
- Project list/get/detail routes are optionally authenticated reads. They still run through permission evaluation; anonymous callers are represented explicitly instead of being rejected before ACL checks.
- Public projects grant anonymous `Read` through `ProjectPermissionFetcher`. Non-public project reads require an active member with matching permissions.
- Component reads follow the same pattern through `ComponentPermissionFetcher`: anonymous callers can read public-project components, but private component reads require access.
- `GET /api/projects` threads optional caller identity through command, use-case, service, and repository layers so anonymous callers see only public projects and authenticated callers see public projects plus projects they can actually access.
- `ComponentUseCaseImpl` keeps permissions and component lifecycle synchronized by creating or deleting instance resources alongside add/remove operations.
- `MemberUseCaseImpl` enforces a strong grant rule: a requester cannot grant a permission they do not already hold, and component-instance UUID resources can be granted through either exact or generic component authority.
- Specific permission endpoints preserve the path resource type instead of collapsing UUID-shaped resources into an implicit component-only case.
- `ProjectDetailResponse` and `ComponentResponse` still leave `endpoint` and `access_token` as `None`, so the API currently exposes component attachment metadata rather than a provisioning handoff.

## Open Questions

- Should Manifesto eventually surface a richer operator-facing story for component provisioning and component-scoped tokens?
- If more public-read surfaces are added later, should they continue to use the same optional-auth plus explicit-anonymous pattern used here?

## Sources

- [[projects/manifesto/manifesto]] - Project-service overview.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Creation, ownership bootstrap, and publish/archive lifecycle.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission behavior.
- [[concepts/resource-scoped-permission-fetchers]] - Shared RouteBuilder plus PermissionsFetcher pattern used here.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted after these flows complete.
