---
title: >-
  Manifesto AuthZ transport and monotonic ledger
category: concepts
tags: [sentinel-sync, events, authorization, visibility/internal]
sources:
  - sentinel-sync/src/handler.rs
  - sentinel-sync/src/idempotency.rs
  - sentinel-sync/src/translator/manifesto.rs
  - manifesto-events/src/authz.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Sentinel-sync canonicalizes {event_type,data}. Undecodable Manifesto events fail the ledger. v1 incomplete destructives are no-ops. Revisions must be greater than last.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# Manifesto AuthZ transport and monotonic ledger

[[projects/sentinel-sync/sentinel-sync]] is a library crate plus binary. The worker still consumes the bus; tests import `sentinel_sync` directly.

## Envelope

The canonical payload is `{ "event_type": "...", "data": { ... } }`. A flat payload is rebuilt with `canonical_event_envelope`. Missing `event_type` is an error.

If the type looks like Manifesto (`project_`, `member_`, `permission_`, `component_`) and **no translator decodes it**, the handler **fails** the ledger entry (retryable). It must not `complete` as a silent skip. Events for other services may still no-op when unclaimed.

## v1 vs v2 tuples

Incomplete **destructive** v1 payloads (revoke/replace/delete without the exact tuple lists) produce an **empty delta**. That is a deliberate stale-allow no-op, not a guessed sweep. v2 events carry exact writes/deletes (`exact_user_tuple` in `manifesto-events`).

`ProjectPublished` stays a tuple no-op. Create-as-public and `ProjectVisibilityChanged` still own `user:*`.

## Monotonic revision

Manifesto AuthZ events carry a `revision`. The ledger accepts `revision > last` (gaps OK) and rejects `<= last`. `PostgresEventLedger` persists last revision; in-memory remains for tests. The handler uses begin / complete / fail so a failed OpenFGA write is retryable.

## Related

- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[projects/sentinel-sync/references/sentinel-sync-worker]]
- [[projects/manifesto/references/manifesto-event-model]]
- [[projects/manifesto/concepts/membership-restore-and-cas]]
