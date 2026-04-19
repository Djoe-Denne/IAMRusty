---
title: Component-Instance Permissions
category: concepts
tags: [permissions, components, projects, visibility/internal]
sources:
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/domain/src/service/permission_service.rs
  - Manifesto/domain/src/service/permission_fetcher_service.rs
  - Manifesto/http/src/lib.rs
summary: >-
  Manifesto grants permissions at both generic component and component-instance levels, creating
  and removing UUID-scoped resources as components change while still allowing anonymous reads on public projects.
provenance:
  extracted: 0.86
  inferred: 0.10
  ambiguous: 0.04
created: 2026-04-14T20:25:00Z
updated: 2026-04-19T18:00:00Z
---

# Component-Instance Permissions

`[[projects/manifesto/manifesto]]` does not stop at a generic `component` permission. It also creates per-instance resources so a specific component inside one project can carry tighter access than the category as a whole.

## Key Ideas

- `add_component()` validates the component type and uniqueness, creates the matching component-instance resource first, and only persists the `ProjectComponent` once that ACL resource exists.
- `remove_component()` treats component deletion and component-instance ACL cleanup as one consistency boundary; if ACL cleanup fails after removal, the flow restores the component and returns an error instead of leaving silent drift behind.
- `ComponentPermissionFetcher` interprets the first resource ID as the project and the second as the optional component instance, then combines generic `component` permissions with instance-specific UUID permissions and returns whichever is stronger.
- Anonymous callers are represented explicitly through `Option<Uuid>` in the shared permission flow, so public project visibility can grant `Read` on component routes without inventing fake users.
- Member permission routes support both generic and specific grants through `/permissions/{resource}` and `/permissions/{resource}/{resource_id}`, so the API surface exposes the two-level model directly.
- `grant_permission()` treats UUID resources specially: the requester can grant the permission if they already hold it either on that exact component instance or on the generic `component` resource.
- `ComponentResponse.endpoint` and `access_token` still remain `None`, so the permission model is ahead of the current provisioning handoff.

## Open Questions

- Should component-instance resources stay as bare UUID strings, or eventually move to a more explicit identifier shape?
- When component-scoped JWTs exist, should their issuance live in Manifesto or in the component-service boundary itself?

## Sources

- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Route and use-case paths that apply these permissions.
- [[concepts/resource-scoped-permission-fetchers]] - Shared authorization pattern that this page specializes.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - Component validation before permissioned component creation.
- [[projects/manifesto/manifesto]] - Service overview and surrounding project model.
