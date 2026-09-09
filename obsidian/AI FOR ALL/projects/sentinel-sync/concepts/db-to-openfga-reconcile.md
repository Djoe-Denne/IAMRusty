---
title: >-
  DB to OpenFGA reconcile without store reset
category: concepts
tags: [openfga, sentinel-sync, manifesto, visibility/internal]
sources:
  - sentinel-sync/src/reconcile.rs
  - sentinel-sync/src/fga_client.rs
  - Manifesto/infra/src/adapters/org_scope.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Reconcile writes the exact desired−existing delta for project and component tuples only. OpenFGA 1.5 cannot Read with an empty object id; the client reads the store then filters.
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# DB to OpenFGA reconcile without store reset

`reconcile_manifesto` loads live Manifesto rows (owners, parents, memberships, grants, visibility), builds the desired tuple set, reads current OpenFGA tuples, and applies `desired − existing` writes plus `existing − desired` deletes. It does **not** delete the store or wipe other types.

## Scope

- Compared types: **`project` and `component` only**. Organization / notification tuples stay untouched.
- Writes and deletes are chunked. Both use the idempotent Write API.

## OpenFGA 1.5 Read limitation

`tuple_key.object = "project:"` (empty id) is **rejected**. `OpenFgaWriteClient::read_manifesto_tuples` therefore Reads the **whole store** (page size only) and filters to Manifesto types. Do not “fix” this by sending a type-only object key.

Manifesto `org_scope` uses the SDK `OpenFgaPermissionChecker::read_tuples` + `RelationshipTuple`. Those APIs must exist on the pinned rustycog `main` or CI will not compile. See [[projects/aiforall/concepts/rustycog-git-submodule]].

## Related

- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]]
- [[projects/sentinel-sync/references/openfga-model]]
- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]]
