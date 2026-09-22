Jalon : Cloud-portable (hors Apparatus P0–P6).
Chemin : `docs/adr/0600-cloud-portable-opentofu-k8s-gitops.md`.
Statut : Proposed. Réalité : Unimplemented.
- Trois couches : A laptop-compose (DX + IT 0200), B kind (mêmes manifests que staging), C remote k8s cluster-par-env.
- OpenTofu ≥ 1.10 (cible 1.12) jusqu'à l'existence du cluster ; **premier adapter V1 = GKE (`gcp`)** ; 2e adapter = copie du contrat (pas un if apps). Pas Terraform BSL/HCP, pas Terragrunt/Atmos/workspaces, pas `infra/`.
- GitOps V1 = **Flux** pull + Kustomize first-party + HelmRelease tiers ; Argo CD **rejeté V1** (possible si SuperSède). Mesh = T14b (pas Istio) ; `deploy/p4/` Flux séparé. `deploy/apps/base` sans annotation GCP/GKE/GCE.
- Q3 OTLP **fermée → 0602** (plan A câble rustycog ; plan B LGTM/Tempo derrière collector/Alloy, pas dans Rust) — sans SuperSéder 0600. CI : publish+Cosign ; zéro kubectl GHA. Écart 0500 cité, pas corrigé. 0008 intacte.
Voir le fichier ADR.