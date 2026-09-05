---
title: Manifesto API and Permission Flows
category: references
tags: [reference, api, permissions, openfga, visibility/internal]
sources:
  - Manifesto/http/src/lib.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - manifesto-events/src/lib.rs
  - sentinel-sync/src/translator/manifesto.rs
  - openfga/model.fga
summary: >-
  Manifesto-specific API behavior on top of RustyCog's shared HTTP shell, plus the OpenFGA-backed authorization model that replaced the per-resource fetcher pattern.
updated: 2026-09-02T18:00:00Z
---

# Manifesto API and Permission Flows

This page assumes the shared [[projects/rustycog/references/rustycog-http]] and [[concepts/centralized-authorization-service]] patterns are already familiar. It keeps the route, command, and authorization details that are specific to [[projects/manifesto/manifesto]].

## RustyCog Baseline

- [[projects/rustycog/references/rustycog-http]] explains `RouteBuilder`, authentication modes, command-context extraction, and the centralized permission middleware.
- [[concepts/centralized-authorization-service]] explains why every check goes through one shared `Arc<dyn PermissionChecker>` and how tuples reach OpenFGA via [[projects/sentinel-sync/sentinel-sync]].
- [[projects/rustycog/references/rustycog-command]] covers the shared command execution runtime that the handlers delegate into.

## Service-Specific Differences

- [Manifesto/http/src/lib.rs](../../../../../Manifesto/http/src/lib.rs) registers project, component, and member routes against the same shared `permission_checker` on `AppState`. There is no per-resource fetcher anymore.
- Project get/detail and component list/get routes are `.might_be_authenticated()` plus project `Read`. `optional_permission_middleware` resolves anonymous callers as `Subject::wildcard()`. The use-case world-read gate then requires `visibility=public` and status `draft` or `active`. A leftover `viewer@user:*` after private / internal / archive is ignored on those surfaces. sentinel-sync writes that tuple on **create-as-public** and on a real visibility flip (`ProjectVisibilityChanged`). Manifesto does not write FGA. `ProjectPublished` does not write it. Authenticated reads succeed for owner, project member, or organization `Read` on org-owned projects.
- `GET /api/projects` is also optionally authenticated, but its visibility enforcement is SQL: public **and** `draft|active`, OR the caller is an active `project_member`. Org members who only inherit FGA viewer do not appear in the list. See [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].
- There is no `POST .../join`. Adding a member is `Admin` on the project plus an in-use-case check that the requester already holds the granted permission.
- Component routes use `"project"` as the OpenFGA object type today because the deepest UUID in component routes is the project id (`{component_type}` is a string segment). When component routes adopt `{component_id}` UUID parameters, switch the relevant routes to `with_permission_on(_, "component")`.
- Member routes are project-scoped (`with_permission_on(Permission::Admin, "project")` for writes; list/get require `Read`).
- Permission grant/revoke endpoints emit `PermissionGrantedEvent` / `PermissionRevokedEvent`. The Manifesto translator maps the string `resource` to either `project` or `component` and writes/deletes the matching relation tuple — see [[projects/sentinel-sync/references/event-to-tuple-mapping]].
- `ComponentUseCaseImpl` keeps domain state and emitted events synchronized so the OpenFGA tuple graph stays consistent.
- `ProjectDetailResponse` and `ComponentResponse` still leave `endpoint` and `access_token` as `None`, so the API currently exposes component attachment metadata rather than a provisioning handoff.

## Open Questions

- Should Manifesto eventually surface a richer operator-facing story for component provisioning and component-scoped tokens?
- Should component routes adopt UUID `{component_id}` parameters so the middleware can guard against `"component"` directly?

## Sources

- [[projects/manifesto/manifesto]]
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]]
- [[projects/manifesto/concepts/component-instance-permissions]]
- [[concepts/centralized-authorization-service]]
- [[concepts/openfga-as-authorization-engine]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[projects/manifesto/references/manifesto-event-model]]
- [[concepts/anonymous-public-read-via-wildcard-subject]] — wildcard-subject design and remaining visibility-flip / publish-vs-public work.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] — list/GET split, missing join, partnership gap.
