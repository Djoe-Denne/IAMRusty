---
title: ADR-0007 — frontière P3 Lazaret (Accepted)
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
  ADR-0007 Accepted 2026-09-13. Réalité Partial (T1–T9). P3-close livré
  (`kv_purge` ← `component_removed`). T9 chemin `/lazaret/invoke`.
  0006 G et E restent. Pas Implemented.
created: 2026-09-13T17:06:00Z
updated: 2026-09-17T17:40:00Z
provenance:
  extracted: 0.82
  inferred: 0.13
  ambiguous: 0.05
---

# ADR-0007 — frontière P3 Lazaret

Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Hub service : [[projects/lazaret/lazaret]]. Hub ADR : [[projects/manifesto/decisions/index]]. Plan : [[projects/manifesto/references/apparatus-implementation-plan]].

## Statut : Accepted (2026-09-13)

Accept explicite utilisateur (« accepté »). Réalité **Partial** : T1–T9 prouvés (identité hybride, grants live, consent write / révocation close-at-commit, KV Postgres+Redis, secrets-by-ref + adaptateur Vault, proxy nommé, invoke HTTP Lazaret, `kv_purge` sur `component_removed`, chemin public `POST /lazaret/invoke`). Holes (mTLS complete, OpenBao produit, APP-05, 0006 G/E) listés dans le canon ADR — **pas** Implemented. P3-close : [[projects/manifesto/references/apparatus-implementation-plan#P3-close — isolation KV en fin de vie]]. P4+ uniquement sur décision explicite.

Revue 16 sept. (`4f3bb8d7`) : GET snapshot = JWT de service (pas FGA) ; enroll anonyme + consult + CSR forcé. ^[inferred] `Lazaret/README.md` encore « T2 only » — périmé. ^[ambiguous]

## Related

- [[projects/lazaret/concepts/workload-identity]]
- [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]
- [[projects/lazaret/skills/running-apparatus-p3-tests]]
- [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]]
- [[journal/2026-09-17]]
