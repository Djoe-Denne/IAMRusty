---
title: >-
  Apparatus P2 — réconciliation in-process (T1 seulement)
category: concepts
tags: [components, architecture, testing, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/apparatus-p2-implementation-prompt.md
  - Manifesto/tests/apparatus_p2_t1_events.rs
  - Manifesto/tests/apparatus_p2_t1_lookup.rs
summary: >-
  P2 ADR-0006 Accepted, Réalité Partial. T1–T7 verts. Pas de gateway / K8s / 202.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-12T13:20:00Z
updated: 2026-09-12T13:20:00Z
---

# Apparatus P2 — réconciliation in-process

ADR-0006 **Accepted** (2026-09-12). Réalité **Partial** : T1–T7 tests verts ; pas de gateway / K8s / HTTP 202. Le wiki bindings/plan reste conception hors colonnes figées dans le canon.

## T1 — isolation events (ADR-0001)

- `source=managed` + `component_status_changed` **sans** `binding_id` (champ événement, serde optionnel) → ignoré (`Ok`, pas de mutation de statut).
- `source=legacy` et ligne absente → chemin `project_id + component_type` inchangé.
- Lookup SQL `SELECT source FROM apparatus_bindings WHERE component_id = $1` (`SqlApparatusBindingSourceLookup`).
- Pas de colonne génération, pas de worker, pas de nouveau type FGA, pas de nouveau type d'event.

Fichiers : `Manifesto/infra/src/apparatus_binding_source.rs`, `Manifesto/infra/src/event/processors/component_processor.rs`, `apparatus-events/src/component.rs` (`binding_id: Option<Uuid>`), wiring `Manifesto/setup/src/app.rs`.

Tests : [[projects/aiforall/skills/running-apparatus-p2-tests]].

## Related

- [[projects/manifesto/concepts/apparatus-p1-persistence]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0001-apparatus-binding]]
