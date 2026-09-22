---
title: "ADR-0600 — déploiement portable OpenTofu / GKE / Flux (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0600-cloud-portable-opentofu-k8s-gitops.md
  - docs/platform-cloud-v1-implementation-contract.md
  - docs/platform-otlp-grafana-oss-explained.md
summary: >-
  Canon : docs/adr/0600. Trois couches (compose, kind, remote k8s).
  OpenTofu jusqu'à l'existence du cluster ; premier adapter GKE (gcp) ;
  Flux + Kustomize ; même image digest. Pas de lock-in Scaleway.
  Q3 OTLP fermée → 0602 (câble rustycog ≠ LGTM dans Rust). Ne SuperSède pas 0008 ni 0500.
created: 2026-09-20T16:20:00Z
updated: 2026-09-20T17:00:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0600 — cloud portable OpenTofu / GKE / Flux

Canon : `docs/adr/0600-cloud-portable-opentofu-k8s-gitops.md`. Hub : [[projects/aiforall/decisions/index]]. Contrat : `docs/platform-cloud-v1-implementation-contract.md`. Topologie : [[projects/aiforall/decisions/0601-cluster-topology]]. Observabilité : [[projects/aiforall/decisions/0602-observabilite-portable]]. P4 (pas cette ADR) : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]].

## Statut : Proposed (2026-09-20)

Réalité **Unimplemented**. Ne pas présenter comme Accepted. Ne réécrit pas [[projects/aiforall/decisions/0500-plateforme-qualite]] ; **cite** l'écart Compose (openbao / lazaret / mesh absents de la photo 0500). Ne SuperSède **pas** 0008.

## Paquet figé (cible)

1. Couches **A** laptop-compose (DX + IT 0200), **B** kind (mêmes manifests que staging), **C** remote k8s cluster-par-env.
2. OpenTofu ≥ 1.10 (cible 1.12) ; modules + stacks dossiers ; **premier adaptateur V1 = GKE (`gcp`)** ; outputs identiques ; state S3-compatible + lockfile + encryption native. Arrêt V1 = existence du cluster. EKS / Kapsule / k3s **rejetés en V1**.
3. GitOps **Flux** pull (V1 **décidé** ; Argo CD **rejeté V1**, possible plus tard si SuperSède) ; Kustomize first-party `deploy/apps/` (sans annotation GCP/GKE/GCE) ; HelmRelease tiers ; `deploy/p4/` Flux **séparé**.
4. Gateway API + Envoy Gateway. Mesh V1 = T14b (pas Istio). kind canon. CI : publish+Cosign ; zéro `kubectl` GHA.
5. Observabilité : collector OTLP ; backends = adaptateurs. **Q3 fermée** → [[projects/aiforall/decisions/0602-observabilite-portable]] (plan A câble rustycog ; plan B LGTM/Tempo derrière collector/Alloy). Pédagogie : `docs/platform-otlp-grafana-oss-explained.md` (pas une ADR).

Suite : [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0602-observabilite-portable]]. Mesh : [[projects/aiforall/concepts/https-platform-mesh]].
