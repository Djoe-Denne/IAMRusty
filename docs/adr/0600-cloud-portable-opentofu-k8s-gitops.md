# ADR-0600 : Le déploiement portable s’appuie sur trois couches (laptop-compose, laptop-k8s kind, remote k8s), OpenTofu jusqu’à l’existence du cluster (premier adaptateur GKE/gcp), et GitOps Flux + Kustomize avec la même image digest

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-20
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. `Réalité` décrit le dépôt. Aujourd’hui : Compose + `just` + IT rustycog ([0200](0200-it-infra-reelle-rustycog-testing.md)) ; scaffolding local `deploy/` + `cloud/opentofu/` (module outputs, pas d’apply GKE) = [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md) Réalité Partial ; **aucun** bootstrap OpenTofu GKE / Flux live. Cette ADR **ne SuperSède pas** [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (Accepted / Implemented) ni [0500](0500-config-typee-et-compose-local.md). Le compagnon `docs/adr/0008-app01-reconciliation.md` reste **méthode**, pas canon.

Plage **0600–0699** = Cloud / IaC / GitOps / topologie cluster (Vague 4 living). **Interdit** d’utiliser 0009 (collision sémantique P4), 0411 (prochain IAM-IdP), 0503 (mélangerait CI rétro 0500). Première tranche livrable **locale** (A+B, sans GKE ni 0602) : [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md).

## Contexte

Le dépôt livre des standalones hexagonaux ([0404](0404-runtime-microservices-et-monolithe.md)) derrière `docker-compose.yml`, des Dockerfiles (`IAMRusty/Dockerfile`, `Hive/Dockerfile`, `Manifesto/Dockerfile`, `Telegraph/Dockerfile`, `Lazaret/Dockerfile`, `Dockerfile.build`, variantes `Dockerfile.solo-build`) et une CI fmt / build / test / coverage / Sonar (`.github/workflows/ci.yml`, [0501](0501-qualite-fmt-clippy-sonar.md)). Config = `rustycog-config` + `RUN_ENV` + préfixes env ([0500](0500-config-typee-et-compose-local.md)). Mesh applicatif T14b = dual-bind 8080+8443, deux CA (wiki `https-platform-mesh`).

ADR-0008 **Accepted** place le moteur P4 **hors Manifesto** (operator + Jobs, Cosign + Transit, enveloppe ≠ image CRI, 4 SA). Elle ne décide **pas** comment un cluster existe, ni GitOps, ni le lock-in cloud. Sans cette ADR, la tentation est : Scaleway comme cœur, Terraform BSL / HCP, `kubectl` depuis GitHub Actions, Kompose depuis Compose, Istio « pour le mesh », un dossier `infra/` qui collisionne les crates `*/infra`.

`ScalewayConfig` / feature rustycog `scaleway-loki` ([0502](0502-rustycog-framework-feature-gated.md)) sont des **adaptateurs applicatifs**, pas une topologie cluster.

Écart **non corrigé** dans 0500 (photographie rétroactive 2026-09-12) : le Compose réel embarque aussi `openbao`, `openbao-seed`, `platform-mesh-certs`, `lazaret-service`. 0500 liste encore exactement les services d’alors. **On ne réécrit pas 0500** ; on cite l’écart.

## Décision

### Trois couches

| Couche | Rôle V1 | Simulation / parité |
|---|---|---|
| **A — laptop-compose** | DX quotidien + IT rustycog (0200) | LocalStack SQS, OpenBao `-dev`, OpenFGA Compose, mesh certs oneshot (`platform-mesh-certs`) |
| **B — laptop-k8s (kind)** | Parité scheduling / SA / namespaces / Gateway / NetworkPolicy / P4 | **Mêmes manifests** que staging ; overlay = replicas (et secrets / SQL / LB **uniquement** overlay) |
| **C — remote k8s** | Staging puis prod | **Cluster-par-env** (pas monocluster multi-env V1) |

Canon inner-loop k8s = **kind** (pas k3d, pas minikube). Inner loop B : `just` + kind + registry local. Pas Tilt / Skaffold V1. Pas Kompose.

Le **monolithe** (`oodhive-monolith`) n’est **pas** le chemin cluster (laptop / démo seulement — détail d’imbrication : [0601](0601-cluster-trust-namespaces-standalones.md)). Remote-dev partagé = **plus tard**.

### Commun à A/B/C vs spécifique overlay

**Commun** : **même image digest** par service ; TOML / `RUN_ENV` ; préfixes HTTP (`SERVICE_PREFIX`, 0404) ; dual-bind 8080+8443 T14b.

**Overlay seulement** : replicas, secrets, SQL managé, annotations load-balancer.

### Control plane IaC — OpenTofu jusqu’à l’existence du cluster

1. **OpenTofu** ≥ 1.10 (cible **1.12**). Pas HashiCorp Terraform V1 (licence BSL, HCP). OpenTofu reste Terraform-compatible : la contrainte « provider TF + K8s » est satisfaite **sans** le binaire HashiCorp.
2. Modules **HCL** + stacks en **dossiers**. Pas Terragrunt, pas Atmos, pas workspaces CLI Terraform/OpenTofu.
3. **Bootstrap cluster = OpenTofu-only.** Crossplane / Cluster API = plus tard.
4. **Un adaptateur** par cloud ; **contrat d’outputs identique** : `cluster_name`, `kubernetes_host`, `cluster_ca`, `oidc_issuer` (nullable), auth **exec**. **V1 livré** = **GKE** (`gcp`). Un 2ᵉ adaptateur (`aws` / `scaleway` / `k3s`) = **copie du contrat**, pas un `if` dans les apps.
5. **State** : backend `s3` OpenTofu S3-compatible + `use_lockfile=true` + **encryption native** OpenTofu (pas de KMS cloud obligatoire). Le bucket **peut** ≠ cloud du cluster.
6. **Arrêt V1 à l’existence du cluster.** Providers Terraform/OpenTofu `kubernetes` / `helm` **hors V1**. Kubeconfig **jamais** git → OpenBao.
7. Arbre : `cloud/opentofu/{modules,live,envs}/`. **Pas** `infra/` (collision crates `*/infra`). Premier overlay cluster : `cloud/opentofu/modules/cluster/gcp/` + `cloud/opentofu/live/.../gcp/`.

#### Premier adaptateur V1 = GKE (`gcp`)

Le critère n’est **pas** le cloud préféré ni le moins cher : **connu + assez de docs pour que les workers implémentent**, sans faire fuiter le provider dans la couche portable.

**GKE** : guide officiel OpenTofu/Google « Using GKE with Terraform » + ressources bornées `google_container_cluster` + `google_container_node_pool` (pièges documentés : `deletion_protection`, `remove_default_node_pool`). Corpus workers = schéma provider + tutoriel Google Learn, **pas** un mega-module.

**EKS rejeté en V1** : densité max (`terraform-aws-modules/eks` v21 sur OpenTofu Registry, Auto Mode, addons, Pod Identity) — trop de surface ; les workers recopient VPC/KMS/IAM dans le premier adapter et font fuiter AWS.

**Kapsule rejeté en V1** : quickstart Scaleway clair (`scaleway_k8s_cluster` + pool) mais corpus tiers mince ; le dépôt a déjà `ScalewayConfig` / `scaleway-loki` (adaptateur **app**, rustycog-config) — risque de fusion mentale cluster ≠ config service.

**k3s VPS rejeté en V1** : docs install k3s excellentes ; chemin OpenTofu = snowflake (Hetzner `identiops/k3s/hcloud`, Proxmox, cloud-init+Ansible) — pas un adapter unique « connu ». `oidc_issuer` souvent nullable (déjà prévu au contrat).

**Portable** : contrat d’outputs **inchangé**. `deploy/apps/base` **sans** annotation GCP/GKE/GCE. GatewayClass cloud = overlay prod MAY plus tard.

**Overlay / adapter** : seulement `cloud/opentofu/modules/cluster/gcp/` + `live/.../gcp/`. Un 2ᵉ adapter = copie du contrat, pas un if dans les apps.

### Runtime GitOps — Flux V1 + Kustomize first-party, Helm tiers

1. First-party : **Kustomize** `deploy/apps/base` + overlays.
2. Tiers : **HelmRelease Flux** (cert-manager, Envoy Gateway, OpenFGA, OpenBao).
3. GitOps V1 : **Flux** pull-based (**décidé**). **Argo CD rejeté pour V1** (pas pour toujours : possible plus tard **si SuperSède**). Interdit V1 : Argo + helm-template comme moteur. Manifests dans **ce** monorepo `deploy/`.
4. **Gateway API** + **Envoy Gateway** (kind + staging). Overlay prod **MAY** retarget `gatewayClassName` plus tard. Bases **sans** annotation provider (pas GCP/GKE/GCE dans `deploy/apps/base`).
5. Mesh V1 : **aucun Istio / Linkerd**. Continuer mTLS applicatif T14b (2 CA : platform-mesh ≠ Lazaret).
6. Operator P4 : `deploy/p4/` — **Kustomization Flux séparée**, **jamais** fusionnée aux charts / overlays Manifesto. P4 **n’est pas** cette ADR ; 0008 reste le canon moteur ; 0601 place les namespaces / SA.

### CI, observabilité, secrets, policy, backups

| Sujet | V1 |
|---|---|
| CI | Jobs actuels (fmt / build / test / coverage / Sonar) **plus** publish + Cosign digest sur `main` (images plateforme). **Zéro `kubectl`** depuis GHA. Promotion = PR GitOps `image@sha256` |
| Observabilité | Collector **OpenTelemetry** OTLP (4317/4318). Q3 → [0602](0602-observabilite-portable-otlp-lgtm.md) : **plan A** câble apps (tracing + bootstrap rustycog, marche sans collector) ; **plan B** premier adaptateur cluster = Grafana OSS LGTM (**Tempo** via collector/Alloy, **pas** dans Rust). Apps → collector seulement. Cockpit / Datadog / APM natif / `scaleway-loki` **interdits comme cœur**. Pédagogie : `docs/platform-otlp-grafana-oss-explained.md` |
| Secrets env | OpenBao = SoT ; ESO → Secret k8s pour DSN / HMAC. Compose `-dev` OK. **Pas** de secrets git. SOPS seulement bootstrap clés GitOps si besoin |
| Policy | NetworkPolicy + Kyverno (image digest / signée, staging/prod). **Adm-A 0008 ≠ Kyverno** : Kyverno n’émet pas `VALID` |
| Backups | Postgres **natif** prod. Velero **plus tard** |
| Cosign TCB | Pas de KMS cloud Cosign V1 (aligné 0008 Reg-A : Transit OpenBao) |

### V1 vs plus tard

| In V1 | Plus tard |
|---|---|
| Couches A + B (kind) + C staging ; prod = même modèle cluster-par-env | Remote-dev partagé ; monocluster multi-env |
| OpenTofu → existence du cluster ; **un** adaptateur **GKE (`gcp`)** ; outputs identiques | 2ᵉ adaptateur (copie du contrat) ; Crossplane / Cluster API ; providers kubernetes/helm TF |
| Flux pull + Kustomize first-party + HelmRelease tiers (**Flux V1 décidé**) | Argo CD possible **si SuperSède** 0600 |
| Gateway API + Envoy Gateway (kind+staging) | Retarget `gatewayClassName` prod si besoin |
| mTLS app T14b, 2 CA | Mesh Istio/Linkerd (rejeté V1, pas « bientôt par défaut ») |
| OTLP + collector + LGTM/Tempo **derrière** collector (0602 plan B) ; bootstrap rustycog (plan A) | Mimir HA ; tail sampling ; spans SQL/queues/plugins ; Meter HTTP rustycog |
| OpenBao SoT + ESO | SOPS généralisé, Transit comme KV plugin (interdit) |
| NetworkPolicy + Kyverno digest/sign | Policy engines supplémentaires ; Adm-B 0008 |
| Backup Postgres natif prod | Velero |
| `deploy/p4/` Flux séparé | Fusion avec Manifesto (interdit) |

## Conséquences

- Premier cluster V1 = **GKE** via `cloud/opentofu/modules/cluster/gcp/` + `live/.../gcp/` seulement. Un nouvel hébergeur = **nouvel adaptateur** au contrat d’outputs, pas un mega-module multi-cloud, pas un if provider dans `deploy/apps/base`.
- `ScalewayConfig` / `scaleway-loki` restent des adaptateurs **app**. Ils ne justifient **pas** un cluster Kapsule V1.
- GitOps V1 = **Flux**. Ne pas relancer Argo CD sauf SuperSède. Ne pas poser helm-template comme moteur first-party.
- Ne pas créer `infra/` à la racine. Ne pas commiter de kubeconfig. Ne pas appeler `kubectl` depuis `.github/workflows/`.
- 0500 **reste** la photo Compose 2026-09-12 (sans openbao / lazaret / mesh). L’écart avec `docker-compose.yml` actuel est **signalé**, pas « réparé » dans 0500.
- 0008 **intacte** : Cosign + Transit, Adm-A worker-only, operator+Jobs hors Manifesto, enveloppe ≠ CRI, 4 SA. Kyverno / Gateway / Flux **n’émettent pas** `VALID`.
- 0407–0410 **intactes**. 0411 reste le prochain libre IAM-IdP.
- IT rustycog (0200) restent sur **couche A**. Kind n’est **pas** un prérequis des tests unitaires P4 T2 (0008 / 0601).
- Travail imposé après Accept : scaffolder `cloud/opentofu/` et `deploy/` ; étendre la CI (publish+Cosign) ; **pas** de `cargo` « plateforme » comme preuve de cette ADR.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Terraform BSL / HCP | Licence ; lock-in HashiCorp ; OpenTofu suffit (compatible providers) |
| Terragrunt / Atmos / workspaces CLI | Stacks = dossiers ; workspaces CLI = piège d’état |
| Crossplane / Cluster API V1 | Surface ; bootstrap = OpenTofu-only |
| Mesh Istio / Linkerd V1 | T14b + 2 CA déjà livrés ; ambient Istio n’est pas le cœur |
| Argo CD V1 | Rejeté pour V1 (pas pour toujours). First-party = Kustomize + Flux. Argo plus tard **seulement** si SuperSède |
| Argo + helm-template comme moteur first-party | Interdit V1 (pas un « later » par défaut) |
| EKS (`aws`) V1 | Module `terraform-aws-modules/eks` trop dense (Auto Mode, addons, Pod Identity) ; fuite AWS dans le premier adapter |
| Kapsule (`scaleway`) V1 | Quickstart clair mais corpus tiers mince ; collision mentale avec `ScalewayConfig` / `scaleway-loki` (adaptateurs **app**) |
| k3s VPS V1 | Install docs excellentes ; OpenTofu = snowflake (Hetzner / Proxmox / cloud-init) — pas un adapter unique « connu » |
| k3d / minikube canon | Canon B = kind |
| Kompose | Compose ≠ source des manifests cluster |
| Monolithe = Deployment prod | 0404 : dual laptop ; cluster = standalones (0601) |
| `kubectl` depuis la CI | Promotion = PR GitOps digest |
| Secrets dans git | OpenBao SoT ; ESO |
| Velero = backup Postgres V1 | Postgres natif prod |
| Mega-module multi-cloud | Un adaptateur / cloud, outputs identiques |
| Fusion `deploy/p4/` × Manifesto | 0008 : P4 hors `Manifesto/*/src` |
| KMS cloud Cosign V1 | 0008 Reg-B rejeté ; Transit |
| OpenBao Transit = KV plugin | 0008 : forge de signatures |
| Datadog / Cockpit / `scaleway-loki` **cœur** | Backends = adaptateurs derrière OTel ; premier adaptateur **cluster** V1 = LGTM+Tempo ([0602](0602-observabilite-portable-otlp-lgtm.md) plan B), pas Loki-cœur ni Grafana-dans-Rust |
| Dossier `infra/` | Collision `*/infra` |

## Non décidé ici

1. Split admit vs signer — **reste dans 0008**, ne pas le redécider.
2. Imbrication namespaces / 4+1 Deployments / SA — déjà écrit : [0601](0601-cluster-trust-namespaces-standalones.md). `aiforall-obs` = [0602](0602-observabilite-portable-otlp-lgtm.md).
3. Produit Helm LGTM vs charts séparés — [0602](0602-observabilite-portable-otlp-lgtm.md) (contrainte Tempo via collector).

Q1 (premier adaptateur), Q2 (Flux vs Argo) et **Q3 (OTLP / observabilité)** sont **clos** (Q3 = 0602 : câble rustycog ≠ backend LGTM, sans SuperSéder cette ADR). Escalade humaine restante : **Accept** 0600/0601/0602 ; tout SuperSède d’une Accepted. **Plus de Q3.**

## Références

- Wiki : `obsidian/AI FOR ALL/projects/aiforall/decisions/0600-cloud-portable.md` ; mesh T14b `obsidian/AI FOR ALL/projects/aiforall/concepts/https-platform-mesh.md` ; P4 `obsidian/AI FOR ALL/projects/manifesto/decisions/0008-apparatus-p4-k8s.md`
- Dépôt : `docker-compose.yml`, `justfile`, `Dockerfile.build`, `*/Dockerfile`, `.github/workflows/ci.yml`, `docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md`, `docs/adr/0404-runtime-microservices-et-monolithe.md`, `docs/adr/0500-config-typee-et-compose-local.md`, `docs/adr/0502-rustycog-framework-feature-gated.md`
- Contrat d’implémentation : `docs/platform-cloud-v1-implementation-contract.md`
- Q3 OTLP (**fermée** → 0602, deux plans câble vs backend) : [0602](0602-observabilite-portable-otlp-lgtm.md) ; pédagogie : `docs/platform-otlp-grafana-oss-explained.md`
- Web : [CNCF OpenTofu](https://www.cncf.io/projects/opentofu/), [OpenTofu manifesto](https://opentofu.org/manifesto/), [backend S3](https://opentofu.org/docs/language/settings/backends/s3/), [state encryption](https://opentofu.org/docs/language/state/encryption/), [Terraform workspaces](https://developer.hashicorp.com/terraform/cli/workspaces), [Crossplane](https://www.crossplane.io/), [Cluster API](https://cluster-api.sigs.k8s.io/user/quick-start), [kind](https://kind.sigs.k8s.io/), [Gateway API](https://gateway-api.sigs.k8s.io/), [Flux bootstrap](https://fluxcd.io/flux/installation/bootstrap/), [OTel collector](https://opentelemetry.io/docs/collector/), [12factor config](https://12factor.net/config), [ESO OpenBao](https://external-secrets.io/latest/provider/openbao/), [Kyverno policy types](https://kyverno.io/docs/policy-types/overview/), [Istio ambient GA](https://istio.io/latest/blog/2024/ambient-reaches-ga/)
- Premier adapter GKE : [Using GKE with Terraform (OpenTofu/Google)](https://search.opentofu.org/provider/opentofu/google/v7.43.0/docs/guides/using_gke_with_terraform), [`google_container_cluster`](https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster)
- Alternatives V1 rejetées : [module EKS OpenTofu Registry](https://search.opentofu.org/module/terraform-aws-modules/eks/aws/latest), [Scaleway Terraform quickstart](https://www.scaleway.com/en/docs/terraform/quickstart/), [k3s installation](https://docs.k3s.io/installation)
- Preuve d’implémentation : `aucune`
