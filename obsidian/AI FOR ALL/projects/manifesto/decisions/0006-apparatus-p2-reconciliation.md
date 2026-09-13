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
summary: >-
  P2 Accepted 2026-09-12 : contrôleur in-process Manifesto. Colonnes et ports
  figés dans le canon. Réalité Partial.
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
created: 2026-09-12T13:14:00Z
updated: 2026-09-13T09:55:00Z
---

# ADR-0006 — réconciliation P2 in-process

Canon : `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`. Hub : [[projects/manifesto/decisions/index]].

## Statut : Accepted (2026-09-12)

Accept explicite utilisateur. Checklist A–M figée dans le canon (génération CAS, lease 30s, fencing génération+epoch, schéma SQL nommé, pas de 202, pas de K8s, pas de nouvel event, ticker in-process, cleanup `apparatus_cleanup_jobs`).

Réalité : Partial. T1 isolation events livrée. T2–T7 = preuves de tests, pas cette page.

## Écarts de réalité P2

Cible Accepted inchangée. Ce jalon : pas de bump à l’update/remove (§A) ; colonnes retry présentes sans writer ni backoff (§D) ; `/ready` non branché (§I). Détail dans le canon.

## Related

- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0001-apparatus-binding]]
