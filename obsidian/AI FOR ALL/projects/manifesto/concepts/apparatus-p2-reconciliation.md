---
title: >-
  Apparatus P2 — réconciliation in-process (Partial)
category: concepts
tags: [components, architecture, testing, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/apparatus-p2-implementation-prompt.md
  - .serena/memories/architecture/apparatus-p2-t3-persist.md
  - .serena/memories/architecture/apparatus-p2-t4-t7-runtime.md
  - Manifesto/infra/src/apparatus_runtime/tick.rs
  - Manifesto/infra/src/apparatus_outbox.rs
  - Manifesto/tests/apparatus_p2_t5_tick.rs
summary: >-
  Contrôleur in-process Manifesto (ticker + scan DB). T1–T7 livrés en Partial :
  pas de bump update/remove, pas de writer retry, /ready hors ticker.
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-09-12T13:20:00Z
updated: 2026-09-13T10:25:00Z
---

# Apparatus P2 — réconciliation in-process

Pourquoi P2 : réconcilier desired/observed **sans** K8s, gateway, broker dédié ni HTTP 202. Le contrôleur est un ticker in-process de Manifesto (standalone et monolithe). Canon : [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]].

ADR-0006 **Accepted** (2026-09-12). Réalité **Partial**. HEAD `7455ee5` (working tree propre le 13 sept.).

Le prompt `docs/apparatus-p2-implementation-prompt.md` dit encore « P2 n’est pas implémenté » — **périmé** vis-à-vis du code. ^[ambiguous]

## Ce qui est livré (T1–T7)

| Tranche | Rôle | Preuve (docs, Docker non relancé ici) |
|---|---|---|
| T1 | Isolation events : managed sans `binding_id` ignoré ; legacy inchangé | `t1_events` 4 + `t1_lookup` 1 |
| T2 | 9 colonnes + `apparatus_cleanup_jobs` sans FK, down réversible | t2 **12** |
| T3 | Create managed gen=1 + `next_retry_at` ; delete → job ; même txn outbox | t3 **5** |
| T4 | Reprise, pas 2e bind, fencing 0-row | t4 **4** |
| T5 | Claim unique, steal, `is_live`, poison isolé | t5 **5** (poison 2026-09-13) |
| T6 | Ports `ApparatusRuntime` + double in-process | t6 **3** (unit) |
| T7 | Cleanup idempotent + gate P3+ ; T7 P1 retargeté | t7 cleanup **2** + gate **5** ; P1 T7 **3** |

Clippy OK cité dans le jalon. Commandes : [[projects/aiforall/skills/running-apparatus-p2-tests]].

## T1 — isolation events (ADR-0001)

- `source=managed` + `component_status_changed` **sans** `binding_id` → ignoré.
- `source=legacy` / ligne absente → `project_id + component_type`.
- Lookup SQL `SqlApparatusBindingSourceLookup`. Le processeur ignore **tout** managed (même si `event.binding_id` est présent) — plus strict que « sans génération ». ^[extracted]

## Runtime

- Ticker : `start_apparatus_runtime` dans `Application::new_with_maybe_event_publisher`.
- Apply : claim CAS (`lease_epoch`) → `bind` si digest présent → `write_observed`.
- Cleanup : `teardown` puis CAS `completed_at`.
- `/ready` : `ReadinessProbe` DB + publisher + consumer — **pas** le ticker.

## Related

- [[projects/manifesto/concepts/apparatus-p1-persistence]]
- [[projects/manifesto/concepts/apparatus-platform]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0001-apparatus-binding]]
- [[journal/2026-09-13]]
