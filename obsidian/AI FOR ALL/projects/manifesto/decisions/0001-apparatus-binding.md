---
title: >-
  ADR-0001 — binding Apparatus 1:1 propriété Manifesto
category: decisions
tags: [architecture, components, visibility/internal]
sources:
  - docs/adr/0001-apparatus-binding-owned-by-manifesto.md
  - Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs
summary: >-
  Binding = extension 1:1 de ProjectComponent, propriété métier Manifesto, zéro nouveau type FGA. Réalité Partial.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# ADR-0001 — binding Apparatus 1:1 propriété Manifesto

Canon : `docs/adr/0001-apparatus-binding-owned-by-manifesto.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- Manifesto reste propriétaire métier du catalogue, du binding et du consentement — sans être autorité d'admission (enregistrer ≠ `VALID`).
- Identité d'installation = `project_components.id` ; desired state en relation 1:1, sans second UUID public.
- Tuples `component:{id}` et événements `ComponentAdded`/`ComponentRemoved` conservés ; pas de nouveau type FGA en V1.
- Un Apparatus canonique du même type par projet.

## Réalité : Partial

- P1 (2026-09-12) : table `apparatus_bindings` 1:1, migration réversible (T1 8/8), backfill legacy idempotent (T2 5/5), mapping injectif (T3), alias `?binding` même `component_id` (T6). Voir [[projects/manifesto/concepts/apparatus-p1-persistence]].
- Manquants : consentement/génération/lease/fencing (ADR-0006 **Proposed**, pas Accepted). T1 isolation events managed livrée (conséquence ADR-0001, pas une ratification P2).

## Non décidé ici

`APP-02` (publish/install), `APP-07` (transfert org), worker/lease/fencing, intention de nettoyage avant cascade SQL.

## Related

- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
