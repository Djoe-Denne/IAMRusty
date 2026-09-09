---
title: >-
  Membership restore, unique owner, and CAS revisions
category: concepts
tags: [membership, transactions, manifesto, visibility/internal]
sources:
  - Manifesto/infra/src/transaction.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/migration/src/m20260909_000011_member_owner_and_soft_delete.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/3c82c7d9-a42d-448f-8573-90ed739ba6b6/3c82c7d9-a42d-448f-8573-90ed739ba6b6.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Restore during grace reuses the same member id, revokes old grants, and applies only the requested grant. Mutations CAS-lock revisions and emit outbox in the same transaction.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# Membership restore, unique owner, and CAS revisions

`join` and admin add-member share `persist_restore_or_insert`. Ownership transfer and project/member updates go through the Manifesto unit of work.

## Restore during grace

- If the user was soft-deleted and the grace window is still open, restore **the same member UUID**.
- Revoke **all** previous grants, then apply **only** the grant requested by this call. Self-join requests `project` / `read` — a previous `write` does not come back.
- After grace, insert a **new** row. The old soft-deleted row stays for audit.

## Owner uniqueness

At most one owner member per project. Transfer updates the owner member and emits events in the same UoW (`transfer_ownership_with_events`). Personal project limits still apply on transfer.

## CAS revisions

- Optimistic lock on `revision`.
- **Delete** locks the **current** revision.
- **Update** / **transfer** increment in memory then lock `revision - 1` (the value still in the database).
- Outbox rows for AuthZ events are written in the **same** transaction as the row change. Failures roll back both. This replaces the older “best-effort log and continue” story for those mutation paths.

## Related

- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]
- [[projects/manifesto/references/manifesto-event-model]]
- [[projects/rustycog/references/rustycog-outbox]]
- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]]
