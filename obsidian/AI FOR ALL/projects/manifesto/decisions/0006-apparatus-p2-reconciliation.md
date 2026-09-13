---
title: >-
  ADR-0006 — réconciliation P2 in-process (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/apparatus-p2-implementation-prompt.md
  - .serena/memories/architecture/apparatus-p2-adr-0006-accepted.md
  - Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs
  - Manifesto/infra/src/apparatus_runtime/tick.rs
  - Manifesto/infra/src/apparatus_runtime/cas.rs
  - Manifesto/setup/src/app.rs
summary: >-
  P2 Accepted : contrôleur in-process Manifesto. Réalité Implemented : T1–T7,
  update CAS +1, delete snapshot+cleanup, writer backoff/terminal, `/ready`
  ticker.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-12T13:14:00Z
updated: 2026-09-13T12:00:00Z
---

# ADR-0006 — réconciliation P2 in-process

Canon : `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`. Hub : [[projects/manifesto/decisions/index]]. Concept : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

## Statut : Accepted (2026-09-12)

Accept explicite utilisateur. Checklist A–M figée dans le canon (génération CAS, lease 30s, fencing génération+epoch, schéma SQL nommé, pas de 202, pas de K8s, pas de nouvel event, ticker in-process, cleanup `apparatus_cleanup_jobs`).

`Accepted` ratifie la **cible**. `Réalité : Implemented` (2026-09-13) se mesure aux tests T1–T7 + writer §D, pas à une promotion wiki au-delà du canon.

## Réalité Implemented (2026-09-13)

Aucun écart P2 ouvert. Détail : canon `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`.

- **§A** — create `desired_generation=1` ; update managed CAS +1 ; delete/remove **ne bump pas** (snapshot + cleanup, décision figée).
- **§B–C** — lease 30 s, tick 2 s, fencing `write_observed` / `write_bind_failure`.
- **§D** — writer backoff : `APPARATUS_RETRY_MAX=8`, `min(30s * 2^retry_count, 5 min)`, codes `bind_failed` / `teardown_failed`, terminal exclu du scan (bindings et jobs).
- **§I** — `/ready` check `apparatus_runtime` (`live_flag`).
- Preuves : t1 4+1, t2 12, t3 7, t4 **5**, t5 **10**, t6 3, t7 cleanup **4** + gate 5, P1 T7 3, readiness 17.

Create managed ne pose pas `digest` ; T4/T5 l'injectent en SQL. ^[inferred]

## Related

- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0001-apparatus-binding]]
- [[projects/aiforall/skills/running-apparatus-p2-tests]]
