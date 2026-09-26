---
title: "ADR-0603 — tranche locale deploy/kind Apparatus↔Lazaret (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: partial
sources:
  - docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md
  - docs/platform-local-v1-implementation-contract.md
summary: >-
  Canon : docs/adr/0603. Première tranche Vague 4 = locale (A+B) :
  deploy/ + cloud/opentofu/ ; preuve Apparatus↔Lazaret en manifests ;
  pas GKE, pas 0602. Lazaret ≠ Factory. Ne SuperSède pas 0600/0601/0008.
created: 2026-09-25T10:51:00Z
updated: 2026-09-25T10:51:00Z
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
---

# ADR-0603 — tranche locale deploy / kind

Canon : `docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md`. Hub : [[projects/aiforall/decisions/index]]. Contrat : `docs/platform-local-v1-implementation-contract.md`. Cadre : [[projects/aiforall/decisions/0600-cloud-portable]], [[projects/aiforall/decisions/0601-cluster-topology]]. Obs (hors tranche) : [[projects/aiforall/decisions/0602-observabilite-portable]]. P4 : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]].

## Statut : Proposed (2026-09-25)

Réalité **Partial** : `just deploy-m1` / `deploy-m2` / `deploy-m3`. Scaffolding A+B autorisé sans Accept GKE. Ne SuperSède **pas** 0600, 0601, ni 0008. Lazaret = gateway P3 ; Factory = P5/P6 non livré.
