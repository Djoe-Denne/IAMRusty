---
title: >-
  ADR-0007 — frontière P3 Lazaret (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/adr/0007-closeout.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/services/lazaret.md
  - Lazaret/README.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/4f3bb8d7-c6d7-4238-a320-445fe479de6b/4f3bb8d7-c6d7-4238-a320-445fe479de6b.jsonl
summary: >-
  ADR-0007 Accepted 2026-09-13. Réalité Implemented (A-DEC 2026-09-20,
  T1–T14b). Holes hors-jalon : APP-05 encore ouvert, 0006 G/E, pas K8s,
  pas de 2e protocole.
created: 2026-09-13T17:06:00Z
updated: 2026-09-20T15:55:00Z
provenance:
  extracted: 0.84
  inferred: 0.12
  ambiguous: 0.04
---

# ADR-0007 — frontière P3 Lazaret

Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Hub service : [[projects/lazaret/lazaret]]. Hub ADR : [[projects/manifesto/decisions/index]]. Plan : [[projects/manifesto/references/apparatus-implementation-plan]]. Mesh : [[projects/aiforall/concepts/https-platform-mesh]].

## Statut : Accepted (2026-09-13)

Accept explicite utilisateur (« accepté »). Réalité **Implemented** (A-DEC 2026-09-20) — T1–T10 + **T11b** + **T12** + **T13** + **T14b**.

Holes **fermés** : mTLS `/session` (T11b `d605377`) ; OpenBao produit (T12 `4e645d5`) ; persistance CA + TLS Lazaret compose (T13 `311e0ab`) ; mTLS Hive–IAM–Telegraph comme HTTPS compose + CA client optionnelle, pas rustls required (T14b `a27ea5b`).

Holes **hors-jalon** (ne bloquent plus Implemented) : APP-05 **reste ouvert** ; 0006 G et E (pas d’`invoke` sur `ApparatusRuntime` ; 5 routes `/components` gelées) ; **pas K8s-as-P3** ; pas de second protocole.

Inventaire : [[projects/manifesto/references/0007-closeout]]. `D-APP01` est tranché par [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (Accepted / **Implemented**, Kind V1) — ce n’est **pas** lever `D-K8S`. ^[inferred]

T14b : preuve mesh = dual-bind in-process (`https_mesh_optional_mtls`) ; les ports compose `8443/8444/8445` ne sont **pas** un gate automatisé (pas de nouveau test compose).

P3-close : [[projects/manifesto/references/apparatus-implementation-plan#P3-close — isolation KV en fin de vie]].

Revue 16 sept. (`4f3bb8d7`) : GET snapshot = JWT de service (pas FGA) ; enroll anonyme + consult + CSR forcé.

## Related

- [[projects/lazaret/concepts/workload-identity]]
- [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]
- [[projects/lazaret/skills/running-apparatus-p3-tests]]
- [[projects/aiforall/concepts/https-platform-mesh]]
- [[entities/paravretius]]
- [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]]
- [[projects/manifesto/references/0007-closeout]]
- [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]
- [[journal/2026-09-20]]
- [[journal/2026-09-18]]
