---
title: >-
  Org-Owned Visibility and Participation Limits
category: concepts
tags: [projects, organizations, visibility, authorization, visibility/internal]
sources:
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/infra/src/repository/project_repository.rs
  - Manifesto/http/src/lib.rs
  - Manifesto/domain/src/value_objects/visibility.rs
  - Manifesto/domain/src/value_objects/member_source.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - Hive/domain/src/entity/mod.rs
  - docs/functional/projet.md
summary: >-
  Internal org-owned read uses organization#member userset; private is
  project_member or org admin; public self-join is live. L-PARTNERSHIP remains
  open. Publish is lifecycle; visibility flips sync user:* via
  ProjectVisibilityChanged.
provenance:
  extracted: 0.82
  inferred: 0.14
  ambiguous: 0.04
created: 2026-09-02T18:00:00Z
updated: 2026-09-05T10:00:00Z
---

# Org-Owned Visibility and Participation Limits

`[[projects/manifesto/manifesto]]` can persist organization-owned projects and a `public` / `private` / `internal` visibility flag. Public self-join is live. Org↔org partnership is not.

## Intended product vs live behavior

A Hive organization owns private and internal projects and may create a **public** project. Participation on that public project has two entry paths:

1. Members of a **partner organization** — **not implemented** (L-PARTNERSHIP).
2. Authenticated **users who are not in any organization** — **implemented** via `POST /api/projects/{id}/join`.

What the runtime actually does:

- Hive owns organizations. Manifesto stores `OwnerType::Organization` plus `owner_id` (a Hive UUID). Creation checks OpenFGA `Write` on `organization:{id}`, not a Hive HTTP call.
- Default visibility is `private`. The creator is the only bootstrapped `ProjectMember`.
- `public` is a **world-read** signal (`viewer@user:*`), not a partner-collaboration mode.
- There is no partnership, alliance, or org-to-org entity in Hive or Manifesto. `ExternalLink` is a GitHub/GitLab provider binding, not a peer organization.

## What works today

- Create a personal or org-owned project. Org-owned requires `owner_id` and OpenFGA write on that organization.
- OpenFGA on `ProjectCreated`: `project:{id}#owner@user:{created_by}`, plus `project:{id}#organization@organization:{owner_id}` when org-owned. If `visibility == "public"`, sentinel-sync also writes `project:{id}#viewer@user:*`. If org-owned Internal, it writes `project:{id}#viewer@organization:{owner_id}#member`.
- Model inheritance (`openfga/model.fga`):
  - `project.viewer = [user, user:*, organization#member] or member` (no `viewer from organization`)
  - `project.admin = [user] or owner or admin from organization`
  - `project.write = member`
- A Hive **member** reads **Internal** org-owned projects via `project:{id}#viewer@organization:{org}#member`. Private org-owned GET/list require a `project_member` row or an **org admin**.
- A Hive **admin** inherits project admin (`admin from organization`) and can add members, publish, archive, and read private org-owned projects.
- `GET /api/projects` lists public live rows, the caller's `project_members`, Internal/public org-owned rows for org viewers, and all live org-owned rows for org admins.

## Limitations

### Partnership (open)

- No org↔org partnership graph exists. QMD and Hive domain have no partner/alliance type. Tracked as **L-PARTNERSHIP**.
- `POST /api/projects/{id}/members` remains Admin-only.
- `MemberSource::OrgCascade` is unused. `Invitation` is written by `POST /api/projects/{id}/join`.
- `external_collaboration_enabled` is stored and returned. No use case reads it.

### Join (done 2026-09-05)

`POST /api/projects/{id}/join` is JWT-only (no project Admin middleware). The project must be `public` and live (`draft|active`). Grants `write` / `project`, `MemberSource::Invitation`, `added_by = caller`. Already a member → 409. No invitation tokens.

### List vs GET (aligned 2026-09-05 except anonymous public)

Authenticated list SQL now matches GET for org-inherited access: Internal for org members, private for org admins. Anonymous list still shows live public SQL rows without `user:*`; anonymous GET `{id}` still needs the wildcard. That split is intentional.

### Internal (done 2026-09-05)

`Visibility::Internal` writes `project:{id}#viewer@organization:{owner_id}#member` (org-owned only). Public stays `user:*` only. Private has no org member userset.

### publish is not public (fixed 2026-09-02)

`publish_project()` only moves lifecycle `draft → active`. It does not change `Visibility`. `ProjectPublished` is now a tuple no-op. `ProjectArchived` still deletes `viewer@user:*` (idempotent cleanup if the project was public).

Historical private-published projects may still have a leftover wildcard until an ops sweep.

### Visibility flip updates OpenFGA (fixed 2026-09-02)

`update_project` still emits `ProjectUpdated` (translator no-op). When visibility **actually** changes, it also emits `ProjectVisibilityChanged` (`old_visibility`, `new_visibility`). A flip that involves `public` requires project `Admin`; `private↔internal` stays `Write`. sentinel-sync writes `viewer@user:*` only on a flip **to** public, and deletes it on a flip **from** public. Entering Internal on an org-owned project writes the org-member userset; leaving Internal deletes it. GET/details/components fail-closed on leftover wildcards while that delete is in flight.

### Internal vs private (done 2026-09-05)

`Visibility::Internal` is org-member read via the userset above. Private is not org-member readable. `external_collaboration_enabled` is still unread and is not partnership.

### Cross-service coupling

Manifesto does not load Hive membership rows. Org access is only the shared OpenFGA graph. If those tuples drift, org-owned project access drifts with them.

## Later work

| ID | Limitation | Status |
|---|---|---|
| L-USER-NO-ORG | Org-less IAM user can join a public project via `POST .../join`. | **Done** 2026-09-05 |
| L-PUBLIC-PARTICIPATE | Public join grants `write` / `project` membership, not world-write. | **Done** 2026-09-05 |
| L-PARTNERSHIP | Hive has no org↔org partnership. Partner members have no distinct path onto a public project. | **Open** — one feeder into join, not a substitute |
| L-JOIN-PRIMITIVE | `POST /api/projects/{id}/join` (not `add_member`). | **Done** 2026-09-05 |
| L-LIST-GET | Authenticated org list/GET aligned; anonymous public list-without-wildcard kept. | **Done** 2026-09-05 |
| L-PUBLISH-NE-PUBLIC | ~~`ProjectPublished` writes `viewer@user:*`~~ **fixed**. Publish stays lifecycle. | Done |
| L-VISIBILITY-FGA | ~~PUT visibility does not sync OpenFGA~~ **fixed** via `ProjectVisibilityChanged`. | Ops sweep helper: `OpenFgaWriteClient::reconcile_wildcards` |
| L-INTERNAL-FLAG | Internal userset is live. `external_collaboration_enabled` stays unread. | **Done** (Internal); flag still dead |

Do not treat L-PARTNERSHIP as the only public-project story. Join already covers org-less users.

## Not the same as component public-read

HTTP component list/get is gated on **project** `Read` plus the same world-read use-case gate as project GET/details. The FGA `component.viewer` relation still derives from `member from project`, not from `viewer`. A leftover project wildcard no longer keeps component HTTP open after private/archive. See [[concepts/anonymous-public-read-via-wildcard-subject]].

## Related

- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]]
- [[projects/manifesto/references/manifesto-api-and-permission-flows]]
- [[projects/manifesto/references/manifesto-event-model]]
- [[concepts/anonymous-public-read-via-wildcard-subject]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[entities/organization]]
- [[entities/user]]
- [[entities/project]]
- [[projects/hive/references/hive-entity-model]]
