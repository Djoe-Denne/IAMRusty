---
title: ADR-0007 — frontière P3 Lazaret (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
summary: >-
  ADR-0007 Accepted 2026-09-13. Réalité Partial (T1–T7). BC Lazaret =
  frontière capacités/données, distinct de Manifesto. 0006 G et E restent.
  Holes listés dans le canon ADR ; pas Implemented.
created: 2026-09-13T17:06:00Z
updated: 2026-09-15T16:42:00Z
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.00
---

# ADR-0007 — frontière P3 Lazaret

Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Hub : [[projects/manifesto/decisions/index]]. Plan : [[projects/manifesto/references/apparatus-implementation-plan]].

## Statut : Accepted (2026-09-13)

Accept explicite utilisateur (« accepté »). Réalité **Partial** : T1–T7 prouvés (consent write / révocation close-at-commit, KV Postgres+Redis, secrets-by-ref + adaptateur Vault, proxy nommé, invoke HTTP Lazaret). Holes (mTLS complete, OpenBao produit, kv_purge↔unbind, prefix invoke IT vs prod, APP-05, 0006 G/E) listés dans le canon ADR — **pas** Implemented. La conception wiki du plan P3 reste `^[inferred]`.
