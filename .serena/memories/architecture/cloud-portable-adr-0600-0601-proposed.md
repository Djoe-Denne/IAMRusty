# Cloud portable 0600 — GKE + Flux clos, OTLP ouvert (2026-09-20)

Q1 premier adapter V1 = **GKE** (`gcp`). Motif: corpus OpenTofu/Terraform borné (google_container_cluster + node_pool), sans mega-module. EKS trop de surface AWS. Kapsule corpus tiers mince; ScalewayConfig = app, pas cluster. k3s install facile mais OpenTofu snowflake.
Portable: outputs identiques; zéro annotation GCP dans deploy/apps/base.

Q2 GitOps = **Flux**. Argo CD rejeté V1.

Q3 fermée → 0602 (sans SuperSéder 0600). Plan A = câble rustycog (tracing + OTLP, apps sans collector). Plan B = LGTM/Tempo derrière collector/Alloy, pas dans Rust. Ensemble crates V1 = OTel 0.32 + tracing-opentelemetry 0.33.
Dépôt aujourd'hui: tracing stdout + option scaleway-loki; zéro SDK OTel. Digest dédié: `mem:architecture/cloud-portable-adr-0602-proposed`.
