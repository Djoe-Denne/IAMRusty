# ADR-0603 : La première tranche livrable de la vague 4 est locale (couches A+B) : IaC dans `deploy/` + `cloud/opentofu/`, preuve Apparatus↔Lazaret encodée en manifests, sans apply GKE ni 0602

- Statut : Proposed
- Réalité : Partial
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0601](0601-cluster-trust-namespaces-standalones.md), [0004](0004-apparatus-capability-gateway.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md), [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)

`Accepted` ratifie une cible. `Réalité` **Partial** : arbres `deploy/` + `cloud/opentofu/` livrés (M1–M3 locaux) ; pas GKE, pas Flux live, pas 0602. Cette ADR **ne SuperSède pas** [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0601](0601-cluster-trust-namespaces-standalones.md), ni [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) (Accepted / **Implemented** A-DEC 2026-09-22). Elle enregistre un **écart de séquence** : livrer A+B local **avant** Accept GKE / couche C / [0602](0602-observabilite-portable-otlp-lgtm.md).

## Contexte

[0600](0600-cloud-portable-opentofu-k8s-gitops.md) et [0601](0601-cluster-trust-namespaces-standalones.md) figent la cible portable (compose / kind / remote k8s ; namespaces ; 4+1 ; NetworkPolicy). Le contrat [platform-cloud-v1-implementation-contract.md](../platform-cloud-v1-implementation-contract.md) dit « ne pas coder tant que Accept » — or la demande produit est d’**implémenter maintenant** l’émulation locale pour **voir** comment Apparatus et Lazaret communiquent, sans cloud réel ni credentials.

Confusion nominale à corriger : « Factory » / host UI = **P5/P6**, **pas livré**, pas de dossier `Factory/`. **Lazaret** = frontière de capacités P3 ([0004](0004-apparatus-capability-gateway.md) / [0007](0007-apparatus-p3-capability-boundary-after-accept.md), Implemented) : I/O untrusted uniquement via gateway (`/lazaret/invoke`, `apparatus_contracts::INVOKE_PATH`). Operator P4 = `apparatus-operator` hors Manifesto ([0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)). Preuves moteur Kind existantes (`apparatus_m5_invoke_isolated`, `apparatus_m6_e2e_chain`, cluster `apparatus-p4-it`) **ne sont pas** la preuve plateforme de cette tranche.

[0602](0602-observabilite-portable-otlp-lgtm.md) (OTLP / LGTM) est **hors scope** de cette ADR et de la tranche locale.

## Décision

1. **Séquence locale-first.** Première tranche Vague 4 livrable = couches **A** (Compose existant, non re-décidé) + **B** (kind + manifests) de 0600. **Pas** d’apply GKE. **Pas** de stack `cloud/opentofu/live/.../gcp/` applicable. **Pas** 0602 / `deploy/obs/` / ns `aiforall-obs`.
2. **Emplacement IaC** (canon 0600) : manifests `deploy/` ; bootstrap OpenTofu `cloud/opentofu/`. **Interdit** `infra/` racine. **Interdit** charts Helm first-party dans les crates Rust. OpenTofu (pas Terraform BSL). Kind = CLI + config + `just` (pas adapter OpenTofu `kind`).
3. **Helm** : rester **Kustomize first-party** pour la topologie AIForAll (ns, Deployments, NetworkPolicy, Services). Helm = **uniquement** tiers via HelmRelease / values documentés (ex. cert-manager ou Envoy Gateway). Pas « un chart Helm par service » : un chart first-party masquerait la carte de confiance 0601 et ferait du moteur Helm le propriétaire des frontières — rejeté pour V1.
4. **Clarification Lazaret ≠ Factory.** Manifests et README parlent de **Lazaret** (gateway) et **apparatus-operator** (P4). Jamais de stub « Factory ».
5. **Trois milestones valuables** (critères démontrables) :

### M1 — Carte lisible + validation sans cluster

Valeur : un non-expert voit **qui parle à qui** et vérifie que les YAML sont sains, **sans** Docker/kind.

- `deploy/apps/base` : **7** namespaces (`aiforall-platform`, `aiforall-gateway`, `aiforall-apparatus`, `aiforall-plugins`, `aiforall-data`, `aiforall-secrets`, `aiforall-gitops`). **Pas** `aiforall-obs` (8ᵉ ns = 0602).
- Workloads **squelette** image placeholder (`pause` ou équivalent) — **pas** de build images Rust : **4+1 complets** (iam, hive, manifesto, telegraph dans `aiforall-platform` + lazaret dans `aiforall-gateway`) + stubs operator (`aiforall-apparatus`) et plugin (`aiforall-plugins`).
- NetworkPolicy **default-deny** + allow **plugins → Lazaret invoke** (port HTTP du Service lazaret) seulement pour ce chemin.
- `deploy/p4/` Kustomize **séparé** (minimal ; jamais fusion Manifesto).
- OpenTofu : module **contrat d’outputs** `cloud/opentofu/modules/cluster/` (`outputs.tf` + README) ; `tofu validate` **sans** credentials. **Pas** de `live/.../gcp/` appliquable.
- Critère : **une** commande documentée — `just deploy-m1` (ou `deploy/verify-m1` PowerShell-friendly) = `kubectl kustomize` / `kustomize build` des overlays kind + p4. README `deploy/README.md`.

### M2 — Kind applique la carte et prouve le chemin invoke (émulé)

Valeur : le cluster local **montre** le chemin plugins → gateway.

- Overlay `deploy/apps/overlays/kind` ; config kind **distincte** de la fixture IT P4 (`apparatus-p4-it`) — nom de cluster **`aiforall-local`**.
- Conteneurs capables d’HTTP minimal côté stub (nginx unprivileged / netcat) **sans** secrets si `pause` ne parle pas HTTP.
- Critère : `just deploy-m2` / `deploy/verify-m2` : kind create (si présent) + apply + `kubectl get` NetworkPolicy/Services **et** probe HTTP stub plugin → Service lazaret (ou Job de preuve). Si kind/Docker absent : **skip clair** ; M1 reste la preuve de session. **Ne pas** faire de `cargo test` M5/M6 la preuve unique de M2.

### M3 — Helm tiers local + pont pédagogique vers la preuve P4

Valeur : un tiers installable localement (lint) + tableau de mapping ns/SA 0601 ↔ IT P4.

- Un HelmRelease **ou** values documentés pour **un** tiers (Envoy Gateway **ou** cert-manager) en overlay kind ; `helm lint` si helm installé ; pas d’install distante ; pas d’exigence Flux controllers live.
- README : tableau (~10 lignes) mappant `m5_invoke_reaches_isolated_plugin_pod` / `m6_e2e_0002_0008_chain` → ns/SA 0601.
- Hors M3 : Flux live, GKE, Cosign CI, 0602, Factory, images Rust de prod.

## Conséquences

- Scaffolding A+B **autorisé** sans Accept GKE/0602 ; couche C et 0602 restent interdits dans cette tranche.
- Contrat d’exécution : [platform-local-v1-implementation-contract.md](../platform-local-v1-implementation-contract.md). Le contrat cloud V1 reste valide pour C/0602/GKE et pointe la tranche locale.
- 0600 reste le **canon long terme** ; 0603 n’est qu’un **ordre de livraison** local.
- Écarts vs souhait « Helm first-party / Terraform / dossier `infra/` / k3d » : **refusés** au profit du canon 0600 (enregistrés ici, pas SuperSédés).

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Attendre Accept 0600+GKE avant tout fichier `deploy/` | Bloque la preuve locale Apparatus↔Lazaret demandée maintenant |
| SuperSéder 0600 pour Helm first-party / `infra/` / Terraform BSL / k3d | Contredit le canon Vague 4 ; 0603 = séquence, pas remplacement |
| Relivrer M5/M6 cargo comme preuve plateforme | Preuve **moteur** P4 ≠ preuve **manifests** plateforme |
| Inclure 0602 / `aiforall-obs` dans M1 | Hors demande ; 8ᵉ ns reporté |
| Stub Factory / host UI | P5/P6 non livrés ; confusion nominale |
| Adapter OpenTofu `kind` ou `live/gcp` avec credentials | 0600 : kind via just+kind ; pas de remote cette tranche |

## Non décidé ici

- Accept formel 0600/0601/0602/0603 (humain / PR).
- Couche C GKE, backends state, Flux controllers live.
- Contenu de [0602](0602-observabilite-portable-otlp-lgtm.md).
- Images digest de prod, Cosign CI, Kyverno staging/prod.
- Host UI / Factory (P5/P6).

## Références

- Canon : `docs/adr/0600-*.md`, `0601-*.md` ; moteur P4 : `0008` (+ closeout) ; gateway : `0004`, `0007`
- Contrat local : `docs/platform-local-v1-implementation-contract.md`
- Contrat cloud (C/0602) : `docs/platform-cloud-v1-implementation-contract.md`
- Code ancré : `Lazaret/application/src/invoke.rs` ; `apparatus_contracts::INVOKE_PATH` ; IT `apparatus-operator/tests/apparatus_m5_invoke_isolated.rs`, `apparatus_m6_e2e_chain.rs` ; fixture kind `apparatus-operator/tests/fixtures/kind/cluster.yaml`
- Compose : `docker-compose.yml` (`lazaret-service` :8084) ; écart `just up` / 0500 cité, non « réparé »
- Wiki pointeur : `obsidian/AI FOR ALL/projects/aiforall/decisions/0603-tranche-locale.md`
- Preuve d’implémentation : `just deploy-m1` (kustomize kind+p4) ; `just deploy-m2` (kind `aiforall-local`, Job `invoke-probe` GET `/lazaret/invoke` 200) ; `just deploy-m3` (`helm lint` shim Envoy Gateway). `tofu validate` non exécuté (binaire absent).
