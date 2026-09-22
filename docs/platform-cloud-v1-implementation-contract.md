# Contrat d’implémentation — plateforme cloud portable V1

Canon : [ADR-0600](adr/0600-cloud-portable-opentofu-k8s-gitops.md), [ADR-0601](adr/0601-cluster-trust-namespaces-standalones.md), [ADR-0602](adr/0602-observabilite-portable-otlp-lgtm.md). Statut ADR : **Proposed** / **Unimplemented**. Ne pas coder tant que l’Accept humain n’est pas donné, sauf scaffolding explicitement demandé après Accept.

**Hors scope de ce contrat (tour docs)** : code applicatif des slices, retarget des gates Manifesto, édition des ADR Accepted (0001–0008, 0100–0502, 0407–0410), `cargo test` / Clippy « plateforme » comme preuve cloud. **Ne pas** scaffolder `deploy/obs/` ni le SDK OTel **dans le même work** que l’IaC. Bootstrap rustycog (0602 plan A) = **work séparé**, après Accept.

P4 mécanisme = [ADR-0008](adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (Accepted / Unimplemented). Ce contrat **place** P4 ; il ne le ré-ADRe pas. **Ne pas toucher ADR-0008.**

## Couches

| Id | Nom | Quoi livrer | Ne pas faire |
|---|---|---|---|
| A | laptop-compose | Conserver `docker-compose.yml` + `just` + IT 0200. Observabilité = profil **`obs`** opt-in (0602), **pas** le `up` défaut | Ne pas « corriger » ADR-0500 ; ne pas Kompose ; ne pas ajouter Redis/Kafka/sentinel-sync au Compose défaut ; ne pas faire de `grafana/otel-lgtm` un modèle prod |
| B | laptop-k8s (kind) | Cluster kind + **mêmes** manifests que staging (overlay replicas) | Pas k3d/minikube canon ; pas Tilt/Skaffold V1 |
| C | remote k8s | Un cluster par env (staging puis prod) | Pas monocluster multi-env V1 ; pas monolithe Deployment |

Commun : **même image digest** / service ; `RUN_ENV` + TOML + préfixes HTTP ; dual-bind 8080+8443. Overlay **seulement** : replicas, secrets, SQL managé, annotations LB.

## Répertoires (à créer — absents du dépôt)

```
cloud/opentofu/modules/          # modules HCL réutilisables
cloud/opentofu/modules/cluster/gcp/         # V1 : GKE seulement
cloud/opentofu/live/.../gcp/     # stacks dossiers (pas Terragrunt/Atmos/workspaces)
cloud/opentofu/envs/             # tfvars / backends par env — pas de secrets
deploy/apps/base/                # Kustomize first-party — SANS annotation GCP/GKE/GCE
deploy/apps/overlays/kind/
deploy/apps/overlays/staging/
deploy/apps/overlays/prod/
deploy/p4/                       # operator P4 — Kustomization Flux SÉPARÉE
deploy/obs/                      # collector + LGTM (HelmRelease) — APRÈS Accept, work plateforme 0602
```

Un 2ᵉ adaptateur (`aws` / `scaleway` / `k3s`) = **copie du contrat d’outputs**, pas un `if` dans les apps. Ne pas scaffolder ces dossiers en V1 **maintenant**.

Work observabilité V1 **séparé** (après Accept seulement) — **ne pas** mélanger dans un seul PR :

1. **Plan A — instrumentation rustycog** (pas Grafana) : feature `otel` ; layers dans `setup_logging` (singleton) ; W3C `traceparent` extract/inject via `TraceContextPropagator` ; ensemble crates **OTel 0.32 + tracing-opentelemetry 0.33** ([0602](adr/0602-observabilite-portable-otlp-lgtm.md)) ; `OTEL_*` ; apps **doivent** tourner sans collector (exporter down ≠ panic). Éditer le checkout `rustycog/` ; durable = sibling rustycog + bump gitlink **plus tard**. Tests : `otel` off ; `otel` on sans endpoint ; panne exporter ; round-trip `traceparent`. **Pas** `cargo test --workspace`. **Pas** de crate `opentelemetry*` dans les slices / `domain` / `application`.
2. **Plan B — plateforme obs** : `deploy/obs/`, ns `aiforall-obs`, overlay kind vs staging, profil Compose `obs`. Kind = collector **ou** Alloy + Tempo + Grafana min (fil traces **obligatoire pour voir**). Staging/prod = LGTM complet. Alloy = distribution collector, pas un SDK.

**Interdit APM / cœurs** : façade CloudWatch / Cloud Monitoring / Cockpit ; Datadog V1 ; Prometheus+Grafana sans Tempo (cible **plateforme**) ; collector-only comme cible **plateforme** V1 (≠ apps sans collector) ; Grafana/Tempo **dans** Rust ; Grafana Cloud V1 ; Jaeger seul ; SigNoz cœur ; `scaleway-loki` comme design traces ; pin OTel **0.33** tant que le bridge spans reste sur **0.32**.

**Interdit** : `infra/` à la racine (collision crates `*/infra`). Kubeconfig **jamais** git.

Outputs adaptateur cluster (contrat identique) : `cluster_name`, `kubernetes_host`, `cluster_ca`, `oidc_issuer` (nullable), auth **exec**.

OpenTofu ≥ 1.10 (cible 1.12). Backend `s3` S3-compatible + `use_lockfile=true` + encryption **native**. Arrêt V1 = **existence du cluster** (pas de providers `kubernetes`/`helm` TF V1).

## Namespaces et workloads (0601)

Préfixe `aiforall-*`, **identiques** sur chaque cluster.

| NS | Contenu |
|---|---|
| `aiforall-platform` | Deployments iam, hive, manifesto, telegraph + sentinel-sync (B/C) |
| `aiforall-gateway` | Lazaret seul + SA `gateway` |
| `aiforall-apparatus` | operator, admit/sign, controller, Jobs build |
| `aiforall-plugins` | pods untrusted uniquement |
| `aiforall-data` | 1 Postgres / 6 DB, OpenFGA, Redis KV Lazaret (B/C), queue adapter |
| `aiforall-secrets` | OpenBao (KV ≠ Transit), ESO |
| `aiforall-gitops` | Flux |
| `aiforall-obs` | Collector **ou** Alloy OTLP, Tempo, Grafana, Loki, Prometheus V1 ([0602](adr/0602-observabilite-portable-otlp-lgtm.md) plan B) |

SA k8s (4, conceptuels) : `build`, `admit-sign`, `controller`, `gateway`. Colocation admit+sign **autorisée** jusqu’au split 0008. Plugins **deny** OTLP ; platform+gateway+sentinel **MAY** → collector. UI Grafana via Gateway.

## In-scope V1 vs later

| In V1 | Later | Interdit (pas un later) |
|---|---|---|
| OpenTofu bootstrap cluster + **1 adaptateur GKE (`gcp`)** | 2ᵉ adaptateur (copie du contrat) ; Crossplane / CAPI ; providers k8s/helm TF | Terraform BSL/HCP ; Terragrunt/Atmos/workspaces ; mega-module multi-cloud ; EKS/Kapsule/k3s comme *premier* adapter V1 |
| **Flux** pull ; Kustomize first-party ; HelmRelease tiers (cert-manager, Envoy Gateway, OpenFGA, OpenBao) | Argo CD possible **si SuperSède** 0600 | Argo CD V1 ; Argo+helm-template comme moteur first-party |
| Gateway API + Envoy Gateway (kind+staging) ; bases sans annotation provider | Retarget `gatewayClassName` prod | Istio/Linkerd V1 |
| kind + `just` + registry local | Remote-dev partagé | k3d/minikube canon ; Kompose ; Tilt/Skaffold V1 |
| CI actuelle + publish+Cosign digest `main` ; promotion PR `image@sha256` | | `kubectl` depuis GHA ; secrets git ; KMS cloud Cosign V1 |
| OTLP rustycog (plan A) + collector/Alloy + LGTM/Tempo (plan B, **pas** dans Rust) — [0602](adr/0602-observabilite-portable-otlp-lgtm.md) ; W3C `traceparent` | Mimir HA ; tail sampling ; spans SQL/queues ; Meter HTTP rustycog ; OTel Logs | Datadog/Cockpit/APM natif **cœur** ; Grafana Cloud V1 ; collector-only *cible plateforme* V1 ; Prometheus+Grafana sans Tempo ; Grafana dans le SDK ; pin OTel 0.33 aveugle ; `scaleway-loki` = traces |
| OpenBao SoT ; ESO → Secret DSN/HMAC ; SOPS seulement bootstrap GitOps | | Transit = KV plugin ; ESO dans `aiforall-plugins` |
| NetworkPolicy default-deny ; Kyverno digest/sign staging/prod | Velero ; SPIRE | Kyverno émet `VALID` ; ns-per-tenant ; monolithe=Deployment prod |
| `deploy/p4/` Flux séparé | | Fusion P4 × Manifesto ; tokens k8s sous `Manifesto/*/src` |
| 4+1 Deployments ; tenancy Hive+FGA | Host P5 dans le dual (0404) | 1 Deployment cluster ; nest HTTP operator |

Lock-in : **pas** Scaleway. `ScalewayConfig` / `scaleway-loki` = adaptateurs **app**. Premier adaptateur cluster V1 = **GKE** (`gcp`). GitOps V1 = **Flux**. Observabilité V1 = **tracing métier + OTLP rustycog (plan A) ; LGTM/Tempo derrière collector/Alloy (plan B)** — [ADR-0602](adr/0602-observabilite-portable-otlp-lgtm.md). Pédagogie (pas canon) : [docs/platform-otlp-grafana-oss-explained.md](platform-otlp-grafana-oss-explained.md).

## Mapping ADR-0008 (ne pas modifier 0008)

| 0008 | Ce dépôt cloud |
|---|---|
| Reg-A+D Cosign + Transit + registry portable | CI publish+Cosign ; Transit isolé du KV ; pas KMS cloud V1 |
| Adm-A seule source `VALID` | Kyverno / admission k8s ≠ Adm-A |
| BC-A operator+Jobs ; pas Factory ; pas new-service | `aiforall-apparatus` + `deploy/p4/` |
| Pkg-B enveloppe ≠ CRI | kubelet = `image@sha256` admis seulement |
| Run-A 4 SA ; plugins autre ns | SA + `aiforall-plugins` |
| Kind pas prérequis T2 unitaire | kind = couche B seulement |
| Split admit/signer ouvert | 4 SA conceptuels ; colocation OK |

## Validation future (après scaffolding — pas maintenant)

Preuves **cloud**, pas `cargo` plateforme :

```text
tofu fmt -check -recursive cloud/opentofu
tofu validate                          # dans chaque stack live/ après init
kustomize build deploy/apps/overlays/kind
kustomize build deploy/apps/overlays/staging
kustomize build deploy/p4
kustomize build deploy/obs          # après scaffold plateforme 0602
```

Compléments acceptables plus tard : `kubeconform` / `kyverno apply --dry-run` sur les overlays. Preuve fil traces **plan B** : une requête HTTP multi-services **un** `trace-id` dans Tempo. Preuve **plan A** : tests rustycog feature `otel` (disabled / console / panne exporter / extract-inject W3C) — **pas** `cargo test --workspace` comme preuve cloud. IT rustycog restent couche A (0200), profil Compose défaut. Gates Manifesto P2 T7 / P3 T2 / P4 T1 **non retargetées**.

## Secrets

Aucun secret dans `cloud/` / `deploy/` / git. Exemples = placeholders. OpenBao pour kubeconfig et SoT. Compose `-dev` inchangé.

## Escalade humaine

Accept 0600/0601/0602 ; tout SuperSède d’une Accepted. Split admit/signer → 0008, pas ici. Adapter GKE, Flux V1 et **Q3** (câble rustycog ≠ LGTM-dans-Rust) **déjà tranchés** — ne pas relancer. Produit Helm LGTM vs charts séparés : Non décidé 0602 (contrainte Tempo via collector/Alloy). Bump ensemble OTel 0.33 : quand `tracing-opentelemetry` suit.
