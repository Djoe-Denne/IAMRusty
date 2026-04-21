---
title: Event To Tuple Mapping
category: reference
tags: [reference, sentinel-sync, authorization, events]
summary: Table of every domain event consumed by sentinel-sync and the OpenFGA tuple writes/deletes it produces. One row per event variant, grouped by producing service.
updated: 2026-04-20
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
| `ProjectCreated`          | `project:{id}#organization@organization:{owner_id}` (when `owner_type == "organization"`) + `project:{id}#owner@user:{created_by}` | —                                                           |
| `ProjectUpdated` / `ProjectPublished` / `ProjectArchived` | (no tuple change)                                                                                                | —                                                             |
| `ProjectDeleted`          | —                                                                                                                                | all tuples on `project:{id}`                                  |
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

## Related

- [[projects/sentinel-sync/references/sentinel-sync-worker]]
- [[projects/sentinel-sync/references/openfga-model]]
