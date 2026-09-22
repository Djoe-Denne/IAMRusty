---
title: >-
  Compagnon de clôture ADR-0007 (A-DEC)
category: references
tags: [architecture, apparatus, visibility/internal]
sources:
  - docs/adr/0007-closeout.md
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/adr/0004-apparatus-capability-gateway.md
summary: >-
  Inventaire A-DEC 2026-09-20 : 0004/0007 Implemented autorisé sans
  fermer APP-05, sans lever G/E, sans K8s-as-P3, sans second protocole.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-20T15:55:00Z
updated: 2026-09-20T15:55:00Z
---

# Compagnon de clôture ADR-0007 (A-DEC)

Méthode, **pas** une ADR. Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Hub : [[projects/manifesto/decisions/index]]. Journal : [[journal/2026-09-20]].

## Verdict

Accord humain exact : « Implemented autorisé sans fermer APP-05, sans lever G/E, sans K8s, sans second protocole. » Les quatre holes sont **hors-jalon**, pas des preuves manquantes (analogie 0006 Implemented avec G/E).

0004 et 0007 : **Accepted / Implemented** (A-DEC 2026-09-20). Preuve P3 = T1–T14b ; ne pas relivrer.

## Hors-jalon (laisser ouvert)

- `APP-05` : options **A/B non choisies** dans 0004 — [[projects/manifesto/decisions/0004-apparatus-gateway]]
- 0006 G et E : pas d’`invoke` sur `ApparatusRuntime` Manifesto ; `/components` = 5
- **K8s-as-P3** interdit (`D-K8S`). Le moteur P4 = [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (Accepted / **Partial**) — ce n’est **pas** lever `D-K8S`. ^[inferred]
- T14b : preuve mesh = dual-bind in-process ; compose `8443/8444/8445` **non** gated (pas de test compose dédié).
- Pas de second protocole / `trusted_skip_gateway`

`D-APP01` du closeout (« avant P4 ») est **tranché** par ADR-0008. Le fichier closeout lui-même ne cite pas 0008 (photo P3). ^[ambiguous]

## Related

- [[projects/lazaret/lazaret]]
- [[projects/manifesto/references/0008-app01-reconciliation]]
- [[projects/manifesto/references/apparatus-p4-implementation-prompt]]
