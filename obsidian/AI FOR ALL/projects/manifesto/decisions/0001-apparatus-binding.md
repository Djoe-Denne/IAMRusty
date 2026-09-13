---
title: >-
  ADR-0001 — binding Apparatus 1:1 propriété Manifesto
category: decisions
tags: [architecture, components, visibility/internal]
sources:
  - docs/adr/0001-apparatus-binding-owned-by-manifesto.md
  - Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
summary: >-
  Binding 1:1 ProjectComponent, propriété Manifesto, zéro type FGA. Réalité
  Partial : P1 table + P2 ticker ; consentement hors P2.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-12T09:30:00Z
updated: 2026-09-13T10:25:00Z
---

# ADR-0001 — binding Apparatus 1:1 propriété Manifesto

Canon : `docs/adr/0001-apparatus-binding-owned-by-manifesto.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- Manifesto reste propriétaire métier du catalogue, du binding et du consentement — sans être autorité d'admission (enregistrer ≠ `VALID`).
- Identité d'installation = `project_components.id` ; desired state en relation 1:1, sans second UUID public.
- Tuples `component:{id}` et événements `ComponentAdded`/`ComponentRemoved` conservés ; pas de nouveau type FGA en V1.
- Un Apparatus canonique du même type par projet.

## Réalité : Partial

- P1 (2026-09-12, désormais dans HEAD `7455ee5`) : table `apparatus_bindings` 1:1, migration réversible (T1 8/8), backfill legacy idempotent (T2 5/5), mapping injectif (T3), alias `?binding` même `component_id` (T6). Voir [[projects/manifesto/concepts/apparatus-p1-persistence]].
- P2 : ADR-0006 **Accepted** / Réalité Partial. Isolation events T1, ticker in-process, lease/fencing, cleanup. Le canon ADR-0001 dit encore « 0006 Proposed » — **périmé** vis-à-vis de `docs/adr/0006`. ^[ambiguous]
- Manquants vis-à-vis 0001 : consentement ; génération CAS complète (create seulement). Voir [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]].

## Non décidé ici

`APP-02` (publish/install), `APP-07` (transfert org). Worker/lease/fencing : tranchés dans [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]] (Partial).

## Related

- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]]
