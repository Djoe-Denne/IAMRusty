# Déploiement local (ADR-0603)

Une commande pour un non-expert infra : **`just deploy-m1`**

Cela valide les manifests (Kustomize) **sans** cluster. Lazaret = gateway P3, **pas** Factory (P5/P6, hors scope).

Canon : [ADR-0603](../docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md) · contrat : [platform-local-v1-implementation-contract.md](../docs/platform-local-v1-implementation-contract.md)

## Milestones

| Milestone | Commande | Ce que ça **prouve** |
|---|---|---|
| **M1** | `just deploy-m1` | 7 namespaces, 4+1 stubs, NP plugins→Lazaret:8080, `deploy/p4` autonome. YAML sains. |
| **M2** | `just deploy-m2` | kind `aiforall-local` applique la carte + Job `invoke-probe` GET `/lazaret/invoke` → 200. |
| **M3** | `just deploy-m3` | Shim Helm **tiers** Envoy Gateway lintable + tableau ci-dessous. |
| **J2** | `just deploy-j2` | Image locale `aiforall-apparatus-controller:j2` (`kind load`, pas de registry) + pod operator **Ready** (binaire `apparatus-controller`, plus `pause`). |
| **J3** | `just deploy-j3` | Overlay **démo** `deploy/apps/overlays/kind-demo-monolith/` : image `aiforall-oodhive-monolith:j3` + Job POST `/lazaret/invoke` → **401** JSON. **Pas** le canon 0601 (4+1). **Pas** une réécriture de `overlays/kind` (M2 nginx). |

M2 : si `kind`, `docker` ou `kubectl` manque → **skip exit 0** (`M2 skip : outil manquant ; M1 reste la preuve`). Sinon create-if-absent, apply, wait Job. **Le cluster n'est pas détruit.** Pour le supprimer : `kind delete cluster --name aiforall-local`.

J2 : `docker build` + `kind load docker-image … --name aiforall-local` + `kubectl apply -k deploy/p4`. Overlay `images:` tag **non-`latest`**, `imagePullPolicy: Never`. Watch CR dans ns `apparatus-system` (inchangé côté Rust). **Ne pas retarget** `apparatus-p4-it`. Si kind/docker/kubectl manque → **STOP** (exit 1).

J3 : `docker build` + `kind load` tag **`j3`** (≠ `latest`) + `kubectl apply -k deploy/apps/overlays/kind-demo-monolith` (contexte `kind-aiforall-local`). Readiness = **GET /health**. Infra DB/FGA = Compose hôte (`just up-infra`) via **`host.docker.internal`** (Docker Desktop Windows) ; pas de Postgres in-kind. Si kind/docker/kubectl ou Postgres :5432 manque → **STOP** (exit 1). Re-appliquer `just deploy-m2` restaure le stub nginx (fichiers `overlays/kind` inchangés).


**CNI :** `aiforall-local` = **Calico v3.29.7** (`disableDefaultCNI` + `podSubnet 192.168.0.0/16` dans `deploy/kind/cluster.yaml` ; install + wait Ready dans `deploy-j2.ps1`). kindnet **interdit** (recreate si détecté). Les NP de `deploy/apps/base/networkpolicies.yaml` sont **enforced** sur ce cluster. La fixture IT `apparatus-operator/tests/fixtures/kind/` (cluster `apparatus-p4-it`) reste **inchangée** et séparée.

## Hors scope

0602 / `deploy/obs/` / `aiforall-obs` · GKE / `cloud/opentofu/live/.../gcp/` · Factory/host UI · Istio / Argo / k3d · charts Helm first-party des slices.

Compose `just up` / `just health` inchangés (couche A).

## Mapping IT P4 → ns/SA 0601

| Preuve moteur | Manifeste plateforme |
|---|---|
| `forward_invoke` / `INVOKE_PATH` (`/invoke`) | Service `lazaret` ns `aiforall-gateway` port **8080**, chemin HTTP `/lazaret/invoke`, SA `gateway` |
| Plugin pod isolé | ns `aiforall-plugins` (pas de SA privilégié) |
| Operator schedule | ns `aiforall-apparatus`, SA `controller` (+ `admit-sign`) |
| Adm-A `VALID` | **≠** Kyverno / Flux (n'émettent pas VALID) |
| Cluster IT `apparatus-p4-it` | **≠** `aiforall-local` (ne pas retarget) |
| `apparatus-operator/tests/apparatus_m5_invoke_isolated.rs` | Pont pédagogique M5 → Service Lazaret + plugin isolé |
| `apparatus-operator/tests/apparatus_m6_e2e_chain.rs` | Pont pédagogique M6 → chaîne admit/schedule/invoke |
| Fixture `apparatus-operator/tests/fixtures/kind/` | Intouchable ; Calico + nom `apparatus-p4-it` |
