# Déploiement local (ADR-0603)

Une commande pour un non-expert infra : **`just deploy-m1`**

Cela valide les manifests (Kustomize) **sans** cluster. Lazaret = gateway P3, **pas** Factory (P5/P6, hors scope).

Canon : [ADR-0603](../../docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md) · contrat : [platform-local-v1-implementation-contract.md](../../docs/platform-local-v1-implementation-contract.md)

## Milestones

| Milestone | Commande | Ce que ça **prouve** |
|---|---|---|
| **M1** | `just deploy-m1` | 7 namespaces, 4+1 stubs, NP plugins→Lazaret:8080, `ops/deploy/p4` autonome. YAML sains. |
| **M2** | `just deploy-m2` | kind `aiforall-local` applique la carte + Job `invoke-probe` GET `/lazaret/invoke` → 200. |
| **M3** | `just deploy-m3` | Shim Helm **tiers** Envoy Gateway lintable + tableau ci-dessous. |
| **J2** | `just deploy-j2` | Image locale `aiforall-apparatus-controller:j2` (`kind load`, pas de registry) + pod operator **Ready** (binaire `apparatus-controller`, plus `pause`). |
| **J3** | `just deploy-j3` | Overlay **démo** `ops/deploy/apps/overlays/kind-demo-monolith/` : image `aiforall-oodhive-monolith:j3` + Job POST `/lazaret/invoke` → **401** JSON. **Pas** le canon 0601 (4+1). **Pas** une réécriture de `overlays/kind` (M2 nginx). |

M2 : si `kind`, `docker` ou `kubectl` manque → **skip exit 0** (`M2 skip : outil manquant ; M1 reste la preuve`). Sinon bootstrap commun create-if-absent, Calico, DNS, apply, wait Job. **Aucun reset, suppression ou redémarrage automatique.** Le parent conserve le bail runtime et assure le stop gracieux des IDs possédés en fin de tests.

J2 : `docker build` + `kind load docker-image … --name aiforall-local` + `kubectl apply -k ops/deploy/p4`. Overlay `images:` tag **non-`latest`**, `imagePullPolicy: Never`. Watch CR dans ns `apparatus-system` (inchangé côté Rust). **Ne pas retarget** `apparatus-p4-it`. Si kind/docker/kubectl manque → **STOP** (exit 1).

J3 : `docker build` + `kind load` tag **`j3`** (≠ `latest`) + `kubectl apply -k ops/deploy/apps/overlays/kind-demo-monolith` (contexte `kind-aiforall-local`). Readiness = **GET /health**. Infra DB/FGA = Compose hôte (`just up-infra`) via **`host.docker.internal`** (Docker Desktop Windows) ; pas de Postgres in-kind. Si kind/docker/kubectl ou Postgres :5432 manque → **STOP** (exit 1). Re-appliquer `just deploy-m2` restaure le stub nginx (fichiers `overlays/kind` inchangés).


**CNI :** `aiforall-local` = **Calico v3.29.7** (`disableDefaultCNI` + `podSubnet 10.244.0.0/16`). `common.ps1` initialise le pool avant le premier apply, attend Calico + CoreDNS + nodes Ready. kindnet, CIDR/pin incompatible, topologie incompatible ou propriété inconnue = **STOP**, pas de delete/recreate. L'enforcement NetworkPolicy doit être prouvé par les tests finaux. La fixture IT protégée reste inchangée.

## Bail et validation statique

`powershell -NoProfile -File ops/scripts/validate-deploy-static.ps1` vérifie les racines, chemins nominaux et la syntaxe sans moteur, cluster ou cargo.

Avant M2/J2/J3/mesh, le parent renseigne `AIFORALL_RUNTIME_LEASE` avec le chemin d'un JSON non secret : `{"context":"kind-aiforall-local","nodeContainerIds":[]}` pour un cluster absent ; pour une réutilisation, les **IDs exacts** explicitement loués dans l'inventaire préalable. Le bootstrap enregistre les IDs créés. Un nom d'image/container n'est pas une preuve de propriété. Les nœuds arrêtés ne sont jamais réveillés par ce helper.

`kind/cluster-local-full.yaml` cible désormais exclusivement le **nouveau cluster dédié `aiforall-local-full` / contexte `kind-aiforall-local-full`** (1 control-plane + 2 workers), selon l'arbitrage user du 2026-10-04 ; le cluster legacy n'est ni modifié ni remplacé. Le [profil local-full](apps/overlays/local-full/README.md) fournit les sources isolées, collaborateurs in-Kind, PVC et helpers de restauration/récupération. Son endpoint HTTP prévu est `127.0.0.1:18080` (NodePort 30080), API loopback à port aléatoire. `just deploy-local-full` rend les ressources **hors ligne par défaut**. **Pas de lab livré ni de preuve runtime** : compilation, correspondance artifacts/source et preuves intégrées restent au parent ; J3/gold restent explicitement hybrides et sur leur contexte legacy.

Pins vérifiés via les métadonnées publiques Docker Hub le 2026-10-03 : Kind node v1.32.2, Envoy v1.31.2, OpenFGA v1.8.5, PostgreSQL 15.12-alpine3.21, tags + digests d'index. Pins reproductibles existants, **pas** une affirmation de support courant. Les images applicatives mesh utilisent au déploiement un tag de contenu dérivé de l'ID réel, un checksum de configuration dans le PodTemplate et un wait rollout après changement. Les readiness `/ready` des quatre services sont périodiques ; les checks TCP Envoy/ext-authz/plugin prouvent uniquement l'ouverture du transport, pas l'état d'autorisation ou l'admission.

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
| `workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs` | Pont pédagogique M5 → Service Lazaret + plugin isolé |
| `workers/apparatus-operator/tests/apparatus_m6_e2e_chain.rs` | Pont pédagogique M6 → chaîne admit/schedule/invoke |
| Fixture `workers/apparatus-operator/tests/fixtures/kind/` | Intouchable ; Calico + nom `apparatus-p4-it` |
