---
title: >-
  ADR-0006 — réconciliation P2 in-process (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/apparatus-p2-implementation-prompt.md
  - .serena/memories/architecture/apparatus-p2-adr-0006-accepted.md
  - Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs
  - Manifesto/infra/src/apparatus_runtime/tick.rs
  - Manifesto/setup/src/app.rs
summary: >-
  P2 Accepted : contrôleur in-process Manifesto. Réalité Partial : T1–T7
  existent ; pas de bump update/remove, pas de writer retry, /ready hors ticker.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-12T13:14:00Z
updated: 2026-09-13T10:25:00Z
---

# ADR-0006 — réconciliation P2 in-process

Canon : `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`. Hub : [[projects/manifesto/decisions/index]]. Concept : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

## Statut : Accepted (2026-09-12)

Accept explicite utilisateur. Checklist A–M figée dans le canon (génération CAS, lease 30s, fencing génération+epoch, schéma SQL nommé, pas de 202, pas de K8s, pas de nouvel event, ticker in-process, cleanup `apparatus_cleanup_jobs`).

`Accepted` ratifie la **cible**. `Réalité : Partial` se mesure aux tests T1–T7, pas à cette ratification.

## Réalité vs checklist A–M (HEAD `7455ee5`)

Working tree propre au 13 sept. 2026 : P1+P2 sont **dans** ce commit, pas un delta non commité. ^[extracted]

| § | Cible | Code actuel |
|---|---|---|
| A Génération | CAS `desired_generation + 1` sur add/**update/remove** managed | **Partial** — create pose `desired_generation=1` + `next_retry_at=NOW()`. Update/remove **ne bumpent pas**. Aucun `desired_generation = desired_generation + 1`. |
| B Lease | 30s, steal, pas de heartbeat, tick 2s | **Oui** — `APPARATUS_LEASE_TTL` / `APPARATUS_TICK_INTERVAL` ; owner `manifesto-{pid}`. |
| C Fencing | WHERE génération + `lease_epoch` + owner + expiry | **Oui** — `write_observed` ; log `apparatus fencing refused observed write`. |
| D Schéma / retry | 9 colonnes + jobs ; writer backoff | **Partial** — colonnes et index présents ; **aucun writer** `retry_count` / `last_error_code`. Échec `bind` = re-scan. |
| E HTTP 202 | NON | **Oui** (absent) — 5 routes `/components`. |
| F K8s | NON | **Oui** (absent) — gate T7. |
| G Ports | Trait sync, double in-process, pas d'`invoke` | **Oui** — loop prod = `bind` + `teardown` (pas `configure`/`unbind` dans le ticker). |
| H Events | Pas de nouveau type ; outbox ownership | **Oui** — T1 isolation + outbox existant. |
| I `/ready` | Ticker vivant + DB **branché** | **Partial** — ticker démarre ; probe = DB + publisher + consumer queue, **pas** `is_live()`. |
| J Outbox | Desired + cleanup + outbox même txn | **Oui**. |
| K Cleanup | Job UoW delete, teardown + CAS | **Oui**. |
| L Gate T7 | Allowlist P2 ; P3+ interdit | **Oui**. |
| M Consentement | Hors P2 | **Oui** (absent). |

## Écarts de réalité P2 (cible inchangée)

Le bloc canon « Écarts de réalité P2 » reste vrai :

- **§A** — génération figée à 1 après create ; reconfiguration non réconciliable dans ce jalon.
- **§D** — colonnes retry présentes, pas d’écrivain ni backoff.
- **§I** — `/ready` non branché sur le ticker.

Mineurs **présents** dans `7455ee5` : index `CREATE INDEX IF NOT EXISTS`, table cleanup `.if_not_exists()`, isolation poison (`apply_due_once` continue, test `t5_poison_bind_does_not_abort_pass`), log fencing. Les `ADD COLUMN` des 9 colonnes **n’ont pas** `IF NOT EXISTS` (re-`up` casse). ^[extracted]

Écart extra : create managed **ne pose pas `digest`** ; `apply_one` no-op si digest NULL. Les tests T4/T5 injectent le digest en SQL. ^[inferred]

## Related

- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0001-apparatus-binding]]
- [[projects/aiforall/skills/running-apparatus-p2-tests]]
