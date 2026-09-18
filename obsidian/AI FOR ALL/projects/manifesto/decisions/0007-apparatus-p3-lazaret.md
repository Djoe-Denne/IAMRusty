---
title: >-
  ADR-0007 — frontière P3 Lazaret (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/services/lazaret.md
  - Lazaret/README.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/4f3bb8d7-c6d7-4238-a320-445fe479de6b/4f3bb8d7-c6d7-4238-a320-445fe479de6b.jsonl
summary: >-
  ADR-0007 Accepted 2026-09-13. Réalité Partial (T1–T10). T8 kv_purge
  f0cf1d2. T10 enrollment Postgres 9e85edd. 0006 G et E restent. Pas
  Implemented.
created: 2026-09-13T17:06:00Z
updated: 2026-09-18T13:45:00Z
provenance:
  extracted: 0.82
  inferred: 0.13
  ambiguous: 0.05
---

# ADR-0007 — frontière P3 Lazaret

Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Hub service : [[projects/lazaret/lazaret]]. Hub ADR : [[projects/manifesto/decisions/index]]. Plan : [[projects/manifesto/references/apparatus-implementation-plan]].

## Statut : Accepted (2026-09-13)

Accept explicite utilisateur (« accepté »). Réalité **Partial** : T1–T10 prouvés (identité hybride, grants live, consent write / révocation close-at-commit, KV Postgres+Redis, secrets-by-ref + adaptateur Vault, proxy nommé, invoke HTTP Lazaret, `kv_purge` sur `component_removed` T8 `f0cf1d2`, chemin public `POST /lazaret/invoke`, enrollment persisté Postgres T10 `9e85edd`). Holes (mTLS complete, OpenBao produit, APP-05, 0006 G/E) listés dans le canon ADR — **pas** Implemented. P3-close : [[projects/manifesto/references/apparatus-implementation-plan#P3-close — isolation KV en fin de vie]]. P4+ uniquement sur décision explicite.

Revue 16 sept. (`4f3bb8d7`) : GET snapshot = JWT de service (pas FGA) ; enroll anonyme + consult + CSR forcé. ^[inferred] `Lazaret/README.md` encore « T2 only » — périmé. ^[ambiguous]

## Related

- [[projects/lazaret/concepts/workload-identity]]
- [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]
- [[projects/lazaret/skills/running-apparatus-p3-tests]]
- [[entities/paravretius]]
- [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]]
- [[journal/2026-09-18]]
