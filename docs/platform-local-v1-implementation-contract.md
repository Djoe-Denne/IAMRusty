# Contrat d’implémentation — plateforme locale V1 (tranche 0603)

Canon tranche : [ADR-0603](adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md). Cadre long terme : [ADR-0600](adr/0600-cloud-portable-opentofu-k8s-gitops.md), [ADR-0601](adr/0601-cluster-trust-namespaces-standalones.md). Statut 0603 : **Proposed** / **Partial** (preuves : `just deploy-m1` / `deploy-m2` / `deploy-m3`). **L’utilisateur a autorisé l’implémentation locale** (écart vs contrat cloud « wait Accept »).

Contrat cloud (GKE / 0602 / couche C) : [platform-cloud-v1-implementation-contract.md](platform-cloud-v1-implementation-contract.md) — **ne pas l’écraser**. Ce document-ci **ne couvre pas** 0602, `deploy/obs/`, ns `aiforall-obs`, ni apply GKE.

**Clarification** : Lazaret = gateway P3. Factory / host UI = P5/P6, **hors scope**. P4 = `apparatus-operator` ([0008](adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) Implemented).

## Autorisé / interdit

| Autorisé | Interdit |
|---|---|
| Créer `deploy/` + `cloud/opentofu/` (pas `infra/` racine) | Secrets, kubeconfig git, credentials cloud |
| Kustomize first-party + NetworkPolicy 0601 (7 ns) | Charts Helm first-party / charts dans crates Rust |
| HelmRelease **tiers** YAML ou values + `helm lint` local | Terraform BSL ; `live/.../gcp/` appliquable |
| Kind cluster **`aiforall-local`** | Réutiliser / casser `apparatus-p4-it` |
| Placeholders images (`pause` / nginx unprivileged) | Build images Rust de prod ; édition crates applicatives |
| Scripts `deploy/verify-m1` / `m2` + recipes `just` | Fusion `deploy/p4/` × Manifesto ; ns-per-tenant ; Istio ; Argo V1 ; k3d canon |
| Pont doc IT P4 ↔ ns/SA | Relancer cargo M5/M6 comme **seule** preuve M2 ; Kyverno comme Adm-A |

## Répertoires à créer

```
deploy/README.md
deploy/verify-m1          # PowerShell-friendly OU just deploy-m1
deploy/verify-m2
deploy/apps/base/         # 7 Namespace + 4+1 stubs + NP + Services
deploy/apps/overlays/kind/
deploy/p4/                # Kustomize séparé (minimal)
deploy/kind/cluster.yaml  # cluster name aiforall-local
cloud/opentofu/README.md
cloud/opentofu/modules/cluster/   # outputs.tf contrat — tofu validate sans cloud
# PAS : cloud/opentofu/live/.../gcp/  |  deploy/obs/  |  infra/
```

## Milestones (à implémenter)

### M1 — Carte + validate sans cluster (obligatoire session)

**Valeur** : topologie lisible + YAML sains.

Livrer :
- 7 ns (pas obs) ; stubs **4+1** (iam, hive, manifesto, telegraph, lazaret) + operator + plugin.
- NP default-deny + allow plugins→lazaret invoke (port Service).
- `deploy/p4/` minimal séparé.
- Module outputs OpenTofu sous `cloud/opentofu/modules/cluster/` ; README cloud ; **pas** live GKE.
- `deploy/README.md` : une commande unique pour non-expert.

**Acceptation** : `just deploy-m1` (ou équivalent documenté) exit 0 = `kustomize build` / `kubectl kustomize` overlays kind + p4 réussit. Pas de cluster requis.

### M2 — Kind + preuve invoke émulée (obligatoire session si Docker/kind)

**Valeur** : chemin plugins→gateway **démontré** sur cluster local.

Livrer :
- Overlay kind ; `deploy/kind/cluster.yaml` nom `aiforall-local`.
- Stubs HTTP si besoin ; probe ou Job de preuve.
- `just deploy-m2` / `deploy/verify-m2`.

**Acceptation** : apply OK + `kubectl get` NP/Services + probe HTTP plugin→Service lazaret. Si kind/Docker absent : message skip clair ; M1 suffit pour la session.

### M3 — Helm tiers + pont P4 (squelette session)

**Valeur** : un tiers lintable + pédagogie mapping IT P4.

Livrer :
- Un tiers (Envoy Gateway **ou** cert-manager) : HelmRelease YAML **ou** values + `helm lint` si helm présent.
- Tableau README (~10 lignes) : tests M5/M6 ↔ ns/SA 0601.

**Acceptation** : fichier(s) présents + lint OK si outil dispo ; tableau ancré chemins repo. Pas Flux live, pas install distante.

## Critères de validation (dépôt)

- M1 : recipe/script unique documentée.
- M2 : script kind + preuve plateforme (pas cargo seul).
- M3 : helm lint optionnel ; doc mapping.
- `tofu validate` sur module cluster **sans** credentials (si tofu installé).
- **Pas** `cargo test --workspace` ; **pas** retarget gates Manifesto.

## Hors scope (explicit)

0602 / OTLP / LGTM / `deploy/obs/` / `aiforall-obs` ; couche C GKE ; Flux controllers live ; Cosign CI ; Factory/host ; images Rust prod ; édition 0008 / 0600 (sauf pointeur déjà fait) ; commit (orchestrateur / humain).

## Ce que l’implementer DOIT / NE DOIT PAS

**DOIT** : suivre 0600 chemins ; 4+1 stubs M1 ; cluster `aiforall-local` ; README une commande ; clarifier Lazaret≠Factory dans les docs `deploy/`.

**NE DOIT PAS** : secrets ; `infra/` ; Terraform BSL ; live GKE ; 0602 ; fusion p4×Manifesto ; charts first-party ; casser fixture `apparatus-p4-it` ; présenter Factory comme livrée.
