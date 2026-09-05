---
title: Event To Tuple Mapping
category: reference
tags: [reference, sentinel-sync, authorization, events]
summary: Table of every domain event consumed by sentinel-sync and the OpenFGA tuple writes/deletes it produces, including the live public wildcard on create-as-public and ProjectVisibilityChanged.
updated: 2026-09-02T19:45:00Z
---

# Event To Tuple Mapping

This is the source of truth for how domain events translate into OpenFGA relation tuples. Each row reflects one `DomainEvent` variant from the producing service.

## Hive

Source enum: `hive_events::HiveDomainEvent`.

| Event                         | Writes                                                      | Deletes                                      |
|-------------------------------|-------------------------------------------------------------|----------------------------------------------|
| `OrganizationCreated`         | `organization:{id}#owner@user:{owner_user_id}`              | —                                            |
| `OrganizationUpdated`         | (no tuple change)                                           | —                                            |
| `OrganizationDeleted`         | —                                                           | all tuples on `organization:{id}`            |
| `MemberJoined`                | `organization:{id}#member@user:{user_id}` (+ role-scoped tuples when present) | —                            |
| `MemberRolesUpdated`          | tuples matching the new role set                            | tuples implied by the previous role set      |
| `MemberRemoved`               | —                                                           | `organization:{id}#member@user:{user_id}` and role tuples |
| `MemberInvited` / `InvitationCreated` / `InvitationAccepted` / `InvitationExpired` | (no tuple change — membership is granted on `MemberJoined`) | — |
| `ExternalLinkCreated`         | (no tuple change — access is derived from `organization#administer`) | —                                    |
| `SyncJobStarted` / `SyncJobCompleted` | (no tuple change)                                   | —                                            |

## Manifesto

Source enum: `manifesto_events::ManifestoDomainEvent`.

| Event                     | Writes                                                                                                                           | Deletes                                                       |
|---------------------------|----------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------|
| `ProjectCreated`          | `project:{id}#owner@user:{created_by}` + `project:{id}#organization@organization:{owner_id}` when `owner_type == "organization"` + `project:{id}#viewer@user:*` when `visibility == "public"` + `project:{id}#viewer@organization:{owner_id}#member` when org-owned Internal | — |
| `ProjectUpdated`          | (no tuple change — metadata only)                                                                                                | —                                                             |
| `ProjectPublished`        | (no tuple change — lifecycle `active`, not visibility)                                                                           | —                                                             |
| `ProjectArchived`         | —                                                                                                                                | `project:{id}#viewer@user:*` and org Internal userset when org-owned |
| `ProjectVisibilityChanged` | `viewer@user:*` when flipping **to** public; org Internal userset when entering Internal on org-owned                            | `viewer@user:*` when flipping **from** public; org Internal userset when leaving Internal |
| `ProjectDeleted`          | —                                                                                                                                | wildcard `viewer@user:*` plus `owner`/`admin`/`member`/`viewer` tuples for `deleted_by` (not a full object sweep) |
| `ComponentAdded`          | `component:{component_id}#project@project:{project_id}`                                                                          | —                                                             |
| `ComponentRemoved`        | —                                                                                                                                | all tuples on `component:{component_id}`                      |
| `MemberAdded`             | `project:{project_id}#member@user:{user_id}`                                                                                     | —                                                             |
| `MemberRemoved`           | —                                                                                                                                | `project:{project_id}#member@user:{user_id}` and any role tuples |
| `MemberPermissionsUpdated` | tuples matching the new permission list                                                                                          | tuples implied by the previous permission list                |
| `PermissionGranted`       | one tuple per granted resource-relation (map the string `resource` to its `object_type` and the string `permission` to a verb relation) | —                                                      |
| `PermissionRevoked`       | —                                                                                                                                | the matching tuple                                            |

## IAM

Source enum: `iam_events::IamDomainEvent`.

| Event                     | Writes                                                                                  | Deletes |
|---------------------------|-----------------------------------------------------------------------------------------|---------|
| `UserSignedUp`            | (no tuple change — user-type has no base relations)                                     | —       |
| `UserEmailVerified`       | (no tuple change)                                                                        | —       |
| `UserLoggedIn`            | (no tuple change)                                                                        | —       |
| `PasswordResetRequested`  | (no tuple change)                                                                        | —       |

IAM currently contributes no authorization tuples — user identity is referenced directly via `user:{uuid}` without needing a derived relation. The `IamTranslator` scaffold is reserved for future events (e.g. platform-admin roles) that may warrant tuples.

## Telegraph

Telegraph is a consumer of notification events and an emitter of at least one authz-relevant event:

| Event                 | Writes                                      | Deletes                                    |
|-----------------------|---------------------------------------------|--------------------------------------------|
| `NotificationCreated` | `notification:{id}#recipient@user:{user_id}` | —                                         |
| `NotificationDeleted` | —                                           | all tuples on `notification:{id}`         |

The Telegraph translator is added by the `telegraph-translator-cutover` todo.

## Conventions

- Every translator is idempotent: the handler already records `event_id` in the ledger before calling the translator, so re-deliveries produce zero network calls.
- Deletions are expressed as full-object deletes when the aggregate is gone; OpenFGA supports this through repeated Write operations plus periodic clean-up jobs if needed.
- Verb mapping (`Read`/`Write`/`Admin`/`Owner` to `read`/`write`/`administer`/`own`) is the same as in [[projects/rustycog/references/rustycog-permission]].

## Public-read status (2026-09-02)

`Tuple::wildcard_user`, the `ProjectCreated` public arm, and `ProjectVisibilityChanged` **are implemented**. `ProjectPublished` is a tuple no-op. `ProjectArchived` still deletes the wildcard. See [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].

Cleanup invariants remain those in [[concepts/anonymous-public-read-via-wildcard-subject]]. The OpenFGA model already permits `[user, user:*]` on `project.viewer`. Historical private-published wildcards are an ops sweep, not a translator bug.

## Related

- [[projects/sentinel-sync/references/sentinel-sync-worker]]
- [[projects/sentinel-sync/references/openfga-model]]
- [[concepts/anonymous-public-read-via-wildcard-subject]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]
