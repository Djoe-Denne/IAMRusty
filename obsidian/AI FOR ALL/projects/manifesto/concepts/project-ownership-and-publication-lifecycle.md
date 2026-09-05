---
title: Project Ownership and Publication Lifecycle
category: concepts
tags: [projects, ownership, lifecycle, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/application/src/usecase/project.rs
summary: Manifesto ties project creation, ownership, visibility defaults, membership bootstrap, and publish/archive transitions into one lifecycle flow — and publish must not be read as public.
provenance:
  extracted: 0.80
  inferred: 0.12
  ambiguous: 0.08
created: 2026-04-14T20:25:00Z
updated: 2026-09-02T18:00:00Z
---

# Project Ownership and Publication Lifecycle

`[[projects/manifesto/manifesto]]` treats project creation as both an ownership decision and a permission bootstrap. A project can be personal or organization-owned, but in either case the creator is inserted into the membership model and the lifecycle then governs whether the project stays draft, becomes active, or is archived.

## Key Ideas

- Personal projects derive `owner_id` directly from the authenticated user, while organization projects require an explicit organization owner ID and an OpenFGA `Write` check on that organization (not a Hive membership-table call).
- New projects default to `Visibility::Private` and `DataClassification::Internal` when the request does not override those fields. Only the creator is inserted as a `ProjectMember`. `MemberSource::OrgCascade` is unused, so other Hive members are not copied onto the project.
- `create_project()` persists the project, creates an owner member, and emits a `ProjectCreated` event. [[projects/sentinel-sync/sentinel-sync]] writes `project:{id}#owner@user:{created_by}` (and `project:{id}#organization@organization:{owner_id}` when org-owned). If `visibility == "public"` it also writes `project:{id}#viewer@user:*`.
- `publish_project()` validates that the project is publishable before transitioning **lifecycle** to `active`. It does not change `Visibility`. `ProjectPublished` is a tuple no-op; `ProjectArchived` still deletes `viewer@user:*`.
- `update_project` emits `ProjectVisibilityChanged` (then `ProjectUpdated`) when visibility actually flips. sentinel-sync writes or deletes `viewer@user:*` from that event.
- Ownership, publication, and archival all emit Manifesto domain events, so lifecycle changes are modeled as integration-relevant state transitions rather than local DB updates only.
- The README documents a broader workflow including `suspended`, while the current HTTP surface centers on publish and archive operations. Conflict to resolve. ^[ambiguous]
- Org-owned public/private participation limits — including the missing partnership/join story — live on [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].

## Open Questions

- When should `suspended` become a first-class operator-facing state in the HTTP and wiki surface? ^[ambiguous]
- Should organization-owned project creation validate Hive membership rows in addition to the OpenFGA write check? ^[inferred]
- `ProjectPublished` no longer writes `viewer@user:*`; visibility flips emit `ProjectVisibilityChanged` (answered 2026-09-02).

## Sources

- [[projects/manifesto/manifesto]] - Service overview for the project-service MVP.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] - Live limits for org-owned visibility, list/GET split, and partner join.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Route and use-case behavior behind creation, publication, and archival.
- [[projects/manifesto/concepts/component-instance-permissions]] - Membership and resource bootstrap that accompanies project creation.
- [[concepts/centralized-authorization-service]] - Shared OpenFGA-backed pattern that the owner bootstrap feeds into.
- [[projects/sentinel-sync/references/event-to-tuple-mapping]] - Manifesto event-to-tuple table.
