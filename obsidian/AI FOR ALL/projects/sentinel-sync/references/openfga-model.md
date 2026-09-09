---
title: OpenFGA Model
category: reference
tags: [reference, authorization, openfga, sentinel-sync, zanzibar]
summary: >-
  Unified Zanzibar model: organization, project (including component_editor/viewer), component, notification. Replaces per-service Casbin files.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
updated: 2026-09-09T16:45:00Z
---

# OpenFGA Model

Source of truth: [openfga/model.fga](../../../../openfga/model.fga).

## Types

| Type           | Relations                                                | Notes                                         |
|----------------|----------------------------------------------------------|-----------------------------------------------|
| `user`         | —                                                        | Terminal subject type.                        |
| `organization` | `owner`, `admin`, `member`, `viewer` + verb relations    | `admin` inherits from `owner`; `member` from `admin`; `viewer` from `member`. |
| `project`      | `organization`, `owner`, `admin`, `member`, `viewer`, `component_editor`, `component_viewer` + verbs | `viewer` allows `[user, user:*, organization#member]`. Generic component grants do **not** follow `member`. |
| `component`    | `project`, `editor`, `viewer` + verbs | `editor` = user or `admin from project` or `component_editor from project`. `viewer` = user or editor or `component_viewer from project`. |
| `notification` | `recipient` + verb relations                             | Notifications are user-scoped; only the recipient can read/write/admin. |

## Verb mapping

Every type exposes the same four verb relations so the checker middleware can call the same relation name regardless of object type. The `Permission` enum in `rustycog-permission` maps as follows:

| `Permission` | Relation name |
|--------------|---------------|
| `Read`       | `read`        |
| `Write`      | `write`       |
| `Admin`      | `administer`  |
| `Owner`      | `own`         |

## Replaces

| Deleted Casbin file                                         | New OpenFGA equivalent                                   |
|-------------------------------------------------------------|----------------------------------------------------------|
| `Hive/resources/permissions/organization.conf`              | `organization` type                                      |
| `Hive/resources/permissions/member.conf`                    | `organization#member` tuple                              |
| `Hive/resources/permissions/external_link.conf`             | `organization#administer` check                          |
| `Manifesto/resources/permissions/project.conf`              | `project` type                                           |
| `Manifesto/resources/permissions/member.conf`               | `project#member` tuple                                   |
| `Manifesto/resources/permissions/component.conf`            | `component` type with `component#project` parent link    |
| `Telegraph/resources/permissions/notification.conf`         | `notification#recipient` tuple                           |
| `IAMRusty/resources/permissions/provider.conf` / `user.conf` | Not migrated (IAM handles its own internals)             |

## Running locally

OpenFGA is part of the root [`docker-compose.yml`](../../../../../docker-compose.yml) — it shares the same `postgres` instance and `aiforall-network` as every other service.

```bash
# Whole stack
docker compose up -d

# Or just the OpenFGA dependencies (Postgres + database creation + OpenFGA migrate + OpenFGA itself)
docker compose up -d postgres create-databases openfga-migrate openfga
```

Endpoints (host): HTTP `http://localhost:8090`, gRPC `localhost:8091`, Playground `http://localhost:3000`. Inside the network use `http://openfga:8080`.

Upload the model via the Playground or the `fga` CLI. The resulting `store_id` and `authorization_model_id` must be set on every client service via `OPENFGA__STORE_ID` and `OPENFGA__AUTHORIZATION_MODEL_ID` env vars (read by `OpenFgaPermissionChecker` in `rustycog-permission`).

## Related

- [[concepts/zanzibar-relation-tuples]]
- [[concepts/openfga-as-authorization-engine]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
