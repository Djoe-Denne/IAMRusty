---
title: Sentinel Sync
category: project
tags: [project, authorization, openfga, sentinel-sync, events]
summary: >-
  OpenFGA Zanzibar store plus sentinel-sync worker: {event_type,data} envelope,
  monotonic Manifesto ledger, v1 destructive no-ops, and DB→tuple reconcile without store reset.
provenance:
  extracted: 0.82
  inferred: 0.14
  ambiguous: 0.04
updated: 2026-09-09T16:45:00Z
---

# Sentinel Sync

The `sentinel-sync` project replaces per-service Casbin authorization with a single Zanzibar-shaped store backed by OpenFGA. Client services no longer own `.conf` files or `PermissionsFetcher` implementations — they only call `Check` through [[entities/permission-checker]].

## Pieces

- **OpenFGA server** — the only authorization engine. Deployed as its own process with its own Postgres store. Model lives at [openfga/model.fga](../../../openfga/model.fga).
- **sentinel-sync worker** — crate `sentinel-sync` (`src/lib.rs` + bin). Consumes events, canonicalizes `{event_type,data}`, translates into OpenFGA Write/Delete, and records `event_id` with begin/complete/fail. Manifesto AuthZ uses a monotonic revision (`revision > last`). Incomplete v1 destructives are no-ops. See [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]].
- **Reconcile** — exact DB→OpenFGA delta for `project`/`component` only, no store reset. OpenFGA 1.5 cannot Read with an empty object id. See [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]].
- **rustycog-permission** — shrunk to a `PermissionChecker` trait plus an `OpenFgaPermissionChecker` client. All Casbin code removed.
- **rustycog-http** middleware — injects `Arc<dyn PermissionChecker>` through `AppState` and exposes `with_permission_on(permission, object_type)` as the only authz builder method.

## Knowledge areas

- References: [[projects/sentinel-sync/references/openfga-model]], [[projects/sentinel-sync/references/sentinel-sync-worker]], [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- Concepts: [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]], [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]], [[concepts/centralized-authorization-service]], [[concepts/openfga-as-authorization-engine]], [[concepts/zanzibar-relation-tuples]]
- Skills: [[projects/sentinel-sync/skills/extending-sentinel-sync-with-new-events]]

## Why centralize

Per-service Casbin engines meant that cross-service decisions (e.g. "this user is an org admin in Hive, so they implicitly admin every Manifesto project under that org") required each service to pull state from the others synchronously. OpenFGA's relation graph lets us express those inheritances declaratively (`admin from organization` on `project`) and answer all decisions with one `Check` RPC.
