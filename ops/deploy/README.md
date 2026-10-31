# Déploiement local : nominal local-full et compatibilité legacy

Contrat : [runtime autonome, validations explicites](../../docs/contracts/ops-kind-runtime-validation-separation.md) · [profil local-full](apps/overlays/local-full/README.md) · [ADR-0606](../../docs/adr/0606-local-full-kind-isole.md).

Le **seul parcours nominal est local-full** : cluster `aiforall-local-full`, contexte `kind-aiforall-local-full`, un control-plane et deux workers. Il ne réutilise ni ne remplace le cluster legacy `aiforall-local`. Aucun repli vers Compose, une démo monolithe ou des stubs. Les commandes ci-dessous sont une procédure opérateur, **pas une preuve live** ; un rendu valide ou des Pods Ready ne prouvent pas un parcours métier.

## 1. Préflight Windows, sans démarrage implicite

Exécuter depuis la racine du dépôt dans une nouvelle session PowerShell Windows (5.1 ou ultérieur). Python **3.11+**, PyYAML, OpenSSL **3+**, Docker avec Compose/BuildKit, Kind et kubectl avec Kustomize intégré sont requis. Outil absent ou commande native en échec ⇒ **STOP**, jamais skip réussi. La préparation des certificats appelle OpenSSL directement, sans Bash/WSL ; sous Windows, elle détecte aussi OpenSSL fourni par Git. Le préflight ci-dessous ne demande aucun démarrage de Docker/Kind/WSL :

```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
foreach ($tool in @('powershell.exe', 'python', 'docker', 'kind', 'kubectl')) {
    Get-Command $tool -ErrorAction Stop | Out-Null
}
function Assert-NativeSuccess([string]$Step) {
    if ($LASTEXITCODE -ne 0) { throw "$Step : échec natif ($LASTEXITCODE) ; STOP" }
}
python -B -c "import sys, tomllib, yaml; assert sys.version_info >= (3, 11)"
Assert-NativeSuccess 'Python/PyYAML'
docker compose version
Assert-NativeSuccess 'Compose'
docker buildx version
Assert-NativeSuccess 'BuildKit/buildx'
kubectl version --client
Assert-NativeSuccess 'kubectl/Kustomize'
kind version
Assert-NativeSuccess 'Kind'
```

Les analyses statiques restent sans moteur. **Avant toute construction Linux ou lancement**, obtenir le GO explicite de cette mission, après corrections, intégration, gates statiques et IT Windows finales. Inventorier les ressources préexistantes et prendre un bail de session ; consigner les IDs effectivement créés dans le registre de ressources du projet, sans s'attribuer les préexistants. Un nom, tag ou digest ne prouve pas la propriété. Les flags `-AfterFinalIT`/`--after-final-it` ci-dessous attestent cette gate, ils ne l'exécutent pas.

## 2. Sorties neuves privées, puis artefacts Linux

Dans la même session, résoudre explicitement la racine autorisée de `local_full_target.approved_root()` : sous Windows, **`Path.home()/AppData/Local/Temp/opencode`** (`local_full_target.py:128–129`). Ne pas copier un chemin personnel. Le répertoire approuvé doit déjà exister ; ne pas le remplacer par un lien. Réserver un répertoire de session neuf avec ACL limitées à l'opérateur et SYSTEM, avant d'y écrire des données :

```powershell
$approvedRoot = python -B -c "from pathlib import Path; print((Path.home() / 'AppData/Local/Temp/opencode').resolve())"
Assert-NativeSuccess 'Racine approuvée'
$approvedRoot = $approvedRoot.Trim()
if (-not (Test-Path -LiteralPath $approvedRoot -PathType Container)) { throw 'Racine approuvée absente' }
$taskId = [guid]::NewGuid().ToString('N')
$session = Join-Path $approvedRoot "local-full-$taskId"
New-Item -ItemType Directory -Path $session -ErrorAction Stop | Out-Null # pas -Force
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$system = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$acl = [Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$inherit = [Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
foreach ($identity in @($sid, $system)) {
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
        $identity, 'FullControl', $inherit, 'None', 'Allow'))
}
Set-Acl -LiteralPath $session -AclObject $acl
$lease = Join-Path $session 'lease.json'
$images = Join-Path $session 'images.json'
# Stable certificate state, independent of the fresh runtime/session lease.
$certs = Join-Path $approvedRoot 'aiforall-local-full-certs'
$render = Join-Path $session 'render'
$environment = Join-Path $session 'environment.json'

function Write-NewJson([string]$Path, $Value) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($Value | ConvertTo-Json -Depth 8))
        $stream.Write($bytes, 0, $bytes.Length)
    } finally { $stream.Dispose() }
}
```

Après GO, construire les images **Linux de livraison pour Kind**, pas les tests de développement. Ces six commandes contractuelles utilisent les Dockerfiles existants, **sans `compose up`** ni service applicatif Docker démarré :

Exécuter ces builds depuis un **worktree Git de livraison propre**, avec le
gitlink SDK exact matérialisé, jamais depuis le workspace canonique contenant
des preuves privées ou modifications utilisateur. Avant tout transfert Docker,
créer exclusivement dans ce worktree une copie de
`ops/deploy/build/delivery-context-ignore.txt` sous le nom `.dockerignore` et
comparer leurs SHA256. Refuser un fichier destination préexistant ; ne pas copier
le `.dockerignore` local ignoré du workspace canonique. Le Dockerignore spécifique
du controller applique les mêmes exclusions d'états d'agents et de worktrees.
Cette copie de métadonnées versionnées est le seul input généré admis ici ; elle
ne contient ni certificat, secret, kubeconfig ou sortie de campagne.

```powershell
$env:DOCKER_DEFAULT_PLATFORM = 'linux/amd64'
docker compose -p aiforall build build-artifacts
Assert-NativeSuccess 'build-artifacts'
docker compose -p aiforall --profile mesh build iam-service hive-service telegraph-service manifesto-service lazaret-service ext-authz
Assert-NativeSuccess 'Services mesh'
docker tag aiforall-lazaret-service:latest aiforall-lazaret-service:local-full
Assert-NativeSuccess 'Tag Lazaret'
docker build --platform linux/amd64 -t aiforall-sentinel-sync:local-full -f ops/deploy/build/Dockerfile.worker .
Assert-NativeSuccess 'sentinel-sync'
docker build --platform linux/amd64 --provenance=false --sbom=false -t aiforall-apparatus-controller:j2 -f workers/apparatus-operator/Dockerfile.controller .
Assert-NativeSuccess 'apparatus-controller'
docker build --platform linux/amd64 --provenance=false -t aiforall-zot:gold -f ops/deploy/p4/Dockerfile.zot ops/deploy/p4
Assert-NativeSuccess 'Zot'
```

Les neuf sources attendues sont énumérées dans `local-full-images.py` (dont `aiforall-ext-authz:local`). Ne jamais monter ni partager le target Windows `E:\cargo-target\AIForAll` avec un build Linux. Le développement et les IT se compilent nativement sous Windows, un seul Cargo à la fois.

## 3. Certificats et rendu offline

Le générateur commun Python/OpenSSL crée les deux CA et les certificats à la première invocation ; les suivantes vérifient et réutilisent le même répertoire privé **hors Git**, sans modifier les clés. Son parent doit déjà exister. ACL Windows limitées à l'opérateur/SYSTEM, permissions POSIX privées ; un état incomplet, expiré, lié ou incohérent est refusé, jamais réparé/écrasé silencieusement. Aucun démarrage runtime requis pour cette préparation :

```powershell
powershell.exe -NoProfile -File ops/scripts/prepare-local-full-certs.ps1 -OutputDirectory $certs
Assert-NativeSuccess 'Certificats local-full'
python -B ops/deploy/deploy-local-full.py render --output-dir $render
Assert-NativeSuccess 'Rendu offline'
```

Équivalent **Linux/CI** : `python3 -B ops/scripts/prepare-local-full-certs.py --output-dir <répertoire-privé>`. L'entrée Bash existante reste disponible : `LOCAL_FULL_CERT_DIR=<répertoire-privé> bash ops/scripts/prepare-local-full-certs.sh` (interpréteur configurable par `PYTHON`). Les trois entrées exécutent le même générateur.

Options : `-OAuthProof` en PowerShell / `--oauth-proof` en Python ou Bash ajoute le certificat GitLab **à la création initiale**. Une demande OAuth sur un répertoire de base déjà créé est refusée sans ajout de clés : choisir une nouvelle destination explicite. Un ensemble OAuth complet peut être réutilisé pour le profil de base. `-OpenSslPath` / `--openssl` (ou `OPENSSL`) sélectionne un exécutable ; `-PythonPath` sélectionne Python côté PowerShell. CA365jours et certificats90jours, comme le générateur précédent ; renouvellement/rotation restent des actions séparées, non implémentées ici.

Les sorties restent `mesh/{ca,iam-service,hive-service,telegraph-service,manifesto-service,envoy-mesh,ext-authz,mesh-client}.{crt,key}` et `workload/{ca,server}.{crt,key}`, plus les CSR/extensions/sériaux. Deux CA distinctes sont produites (mesh transport et workload) ; fournir aux validateurs uniquement le certificat/chaîne **public**, jamais `ca.key`, une clé leaf ou le répertoire privé complet. Aucun matériau n'est committé ou affiché. Les erreurs OpenSSL après protection du répertoire conservent leurs détails dans `openssl-error.log` privé et remontent un code non-zéro ; un échec partiel est conservé pour diagnostic, pas repris automatiquement.

En CI, créer une destination temporaire par **job E2E**, la conserver pour tous les consommateurs de ce job puis la retirer en fin de job ; aucune clé dans les caches ou artifacts. Le job `infra-static` fait seulement une génération/réutilisation Linux réelle, sans cluster ; ce n'est pas une preuve E2E. Ne pas imprimer le contenu des clés, tokens, kubeconfigs ou manifests rendus ; les sorties de rendu restent privées hors Git. Ce générateur est local/test, pas un émetteur PKI cloud ni un distributeur KMS. Documentation OpenSSL : [req](https://docs.openssl.org/3.5/man1/openssl-req/) et [x509](https://docs.openssl.org/3.5/man1/openssl-x509/).

Le rendu est **offline par design**. `render-local-full.py:23–26` impose le chemin **inexistant** `ops/deploy/offline-no-kubeconfig` pour empêcher le fallback vers des credentials ambiants : **ne pas créer ni « réparer » ce fichier**. Aucun cluster, bail ou appel API n'est requis pour `render` (action par défaut de `just deploy-local-full`). Le rendu n'autorise pas le lancement.

## 4. Bail JSON v2 écrit par l'opérateur

Le bail initial est une sortie neuve de l'opérateur Windows, sans import d'un harness de test ou d'une ancienne campagne. Son répertoire privé `$session` est **distinct** du répertoire d'ancrage ; ce dernier et son kubeconfig doivent être absents, créés exclusivement par bootstrap. Création JSON UTF-8 exclusive, sans Docker ni API :

```powershell
$anchorDirectory = Join-Path $approvedRoot "cluster-anchor-$taskId"
if (Test-Path -LiteralPath $anchorDirectory) { throw 'Ancrage déjà présent ; aucune adoption' }
$kubeconfig = Join-Path $anchorDirectory 'kubeconfig.yaml'
if ($env:KUBECONFIG -or $env:KUBERNETES_MASTER) { throw 'Override ambiant interdit ; ouvrir une session propre' }
Write-NewJson $lease ([ordered]@{
    version = 2
    cluster = 'aiforall-local-full'
    context = 'kind-aiforall-local-full'
    task_id = $taskId
    anchor_state = 'pending_creation'
    nodeContainerIds = @()
    kubeconfig_path = $kubeconfig
})
```

`task_id` est un UUID hexadécimal minuscule de 32 caractères. Ne préremplir **aucun** `creation`, UID, ID control-plane/nœud, mapping, endpoint ou hash ; aucun export/adopt/retarget d'un cluster existant. Le bail reste modifiable par bootstrap ; celui-ci atteste les IDs observés, valide endpoint/mapping avant GET puis UID avant mutation, et protège/scelle le kubeconfig. Bail emprunté, legacy, ambigu ou chemin lié ⇒ échec. Les contextes legacy `kind-aiforall-local`, protégés `kind-apparatus-p4-it`/`rancher-desktop` et toute cible étrangère ne sont pas des substituts full.

## 5. Bootstrap → images → deploy → inventory

Port HTTP `127.0.0.1:18080` libre, image/digest Kind exact et Calico **v3.29.7** téléchargé explicitement par le bootstrap sont requis ; API loopback à port aléatoire. Ne pas libérer un port en arrêtant son propriétaire inconnu. Exécuter dans cet ordre, avec le bail neuf et seulement après les IT finales :

```powershell
powershell.exe -NoProfile -File ops/deploy/bootstrap-local-full.ps1 -LeasePath $lease -AfterFinalIT
Assert-NativeSuccess 'Bootstrap full'
python -B ops/deploy/local-full-images.py --lease $lease --output $images
Assert-NativeSuccess 'Épinglage images'
python -B ops/deploy/deploy-local-full.py deploy --lease $lease --image-manifest $images --cert-directory $certs --after-final-it
Assert-NativeSuccess 'Deploy full'
python -B ops/deploy/deploy-local-full.py inventory --lease $lease
Assert-NativeSuccess 'Inventaire full'
```

**L'épinglage exige le bail déjà ancré** ; ce n'est pas une préparation avant bootstrap. Il épingle les neuf images existantes par ID réel, sans build/pull. Le bootstrap rappelé par `deploy` vérifie l'ancrage sans recréer. L'application respecte foundation → dependencies → bootstrap/Jobs → applications, migrations avant scale-out, waits et refus des Jobs actifs ; verrous et rollback restent bornés. Ne pas contourner un verrou résiduel après un échec : faire inspecter la propriété. Un bootstrap partiel conserve ses IDs ; aucun retry/reset/adoption automatique.

Le runtime n'importe ni driver de validation ni suite Rust, ni sorties historiques privées. Le modèle OpenFGA canonique reste l'unique copie dans `ops/openfga/`. Inclus : PG15/six DB/PVC, OpenFGA/store, Redis AOF, Zot, OpenBao auth k8s/Transit, quatre queues, quatre services standalone, Lazaret, sentinel-sync, ext-authz/Envoy, operator/CRD/RBAC, NP/PSS/quotas/PDB. **Simulations locales** : SQS/LocalStack, SMTP/MailHog, catalogue stub, OpenBao DEV et Zot gold. Exclus par défaut : OAuth vendors, edge TLS18443, cloud/HA/HSM, signatures/artefacts et invoke plugin réel sans préparation. `--isolated-plugins`/`--namespace-wiring-confirmed` ne sont pas des preuves du binaire embarqué ; ne pas les ajouter pour contourner une capacité manquante. Scénario empêché ⇒ **BLOCKED/NOT RUN**, pas assertion retirée.

## 6. Validation séparée et IT natives

Après l'inventaire, l'opérateur produit le descripteur validation depuis la cible observée (pas depuis des noms supposés). Le runtime **n'en dépend pas**. Vérifier l'inventaire puis créer un JSON privé neuf, avec le helper exclusif ci-dessus :

```powershell
Write-NewJson $environment ([ordered]@{
    version = 1
    profile = 'local-full'
    context = 'kind-aiforall-local-full'
    cluster = 'aiforall-local-full'
    lease_path = $lease
    namespace = 'aiforall-local-full'
    apparatus_namespace = 'aiforall-local-apparatus'
    plugins_namespace = 'aiforall-local-plugins'
    base_url = 'http://127.0.0.1:18080'
})
```

Ajouter `public_ca_path` seulement si nécessaire, pointant un certificat public. Les champs inconnus, overrides ambigus et cibles/URLs contradictoires sont refusés ; le descripteur ne prouve pas la propriété, **seul le bail vérifié fait autorité**. Interface du lot validation (à utiliser après son intégration) :

- **Smoke** : `python -B ops/tests/local-full-acceptance.py smoke --environment $environment` — readiness/disponibilité (`get`/`wait`), aucune mutation ni préparation cachée. Ready ne prouve ni métier ni AuthZ.
- **Curl public full** : `python -B ops/tests/local-full-acceptance.py route-matrix --environment $environment --cases ops/tests/mesh-public-route-cases-local-full.json` — contrôles publics seulement, sans credentials ni seed. Le full livré déclare `connectors=[]` : les deux démarrages OAuth attendent exclusivement **422** et sont comptés dans `oauth_login_unavailable` (parcours indisponibles, pas logins réussis). La matrice `mesh-public-route-cases.json` conserve **303** pour un profil connecté ; elle n'est pas la matrice nominale full. Le rapport distingue `coverage.public` (PASS/LIVE possible) de `coverage.authenticated` (**NOT RUN**, setup **BLOCKED**) ; les positifs/négatifs authentifiés relèvent du canal E2E ci-dessous, pas du PASS public.
- **E2E** : `python -B ops/tests/local-full-acceptance.py e2e --environment $environment --suite mesh-authn --inputs <JSON-privé>` (ou `--suite apparatus`) — état métier déclaré. Ressources de test via actions séparées `prepare-probes`/`cleanup-probes`, IDs/UID attestés ; jamais préparées par smoke/curl. Setup indisponible ⇒ BLOCKED.

Contrôler chaque `$LASTEXITCODE` comme ci-dessus. Exit 0 seulement si toutes les obligations sont satisfaites ; outil/fixture absent ou skip obligatoire ⇒ non-zéro. Rapporter hash Git, cible/mode, attendus/exécutés/échecs/bloqués et LIVE/SIMULÉ/NOT RUN, sans secrets ni nombre artificiel fixe d'objets. Les modes `network-policy`, `snapshot`, `rollout-check`, `recovery-check`, `restore-check` gardent leur CLI full-only `--lease`/`--namespace` ; observer n'exécute pas une restauration/récupération runtime.

**IT Windows, entrée distincte sans Kind** : `powershell.exe -NoProfile -File ops/tests/it-native.ps1 -Suite iam` (ou `hive`, `manifesto`, `telegraph`, `lazaret`). Le lanceur délègue à `cargo test -p <package-service> --tests`, un package à la fois, sans workspace implicite ni applicatif Docker. Les fixtures testcontainers existantes démarrent leurs dépendances tierces à la demande ; les IT ne nécessitent pas le profil full déployé. Un seul slot Cargo, target Windows sur E: ; elles précèdent le GO runtime.

Le lanceur impose **`RUST_TEST_THREADS=2` par défaut**, quel que soit le nombre de CPU ou la valeur ambiante, pour borner les tests simultanés de chaque binaire libtest et limiter la pression de démarrage sur le daemon. Override explicite : **`AIFORALL_IT_TEST_THREADS`**, entier strictement positif sur 32 bits ; valeur invalide ⇒ STOP avant Cargo ou démarrage de l'observateur. La valeur effective est affichée avant les suites. `RUST_TEST_THREADS` est restaurée en `finally`, y compris sur échec ; l'override n'est pas modifié. Exemple dans la même session PowerShell :

```powershell
$previousThreadsOverride = $env:AIFORALL_IT_TEST_THREADS
try {
    $env:AIFORALL_IT_TEST_THREADS = '4'
    & ./ops/tests/it-native.ps1 -Suite manifesto,hive
} finally { $env:AIFORALL_IT_TEST_THREADS = $previousThreadsOverride }
```

Ce cap ne modifie ni l'herméticité ni les oracles et n'autorise jamais plusieurs Cargo simultanés. Il borne les tests actifs par binaire, **pas à lui seul tous les conteneurs accumulés** : l'arrêt de campagne et le cycle de vie des fixtures restent nécessaires. Les preuves et ledgers append-only de chaque invocation sont sous **`.tmp/itfixtures-<runid>/`** ; mesurer le pic et vérifier l'inventaire final plutôt que déduire un nombre de conteneurs du cap.

Après succès **ou échec**, arrêter gracieusement uniquement les ressources possédées devenues inutiles et sans bail actif, par IDs/UID attestés ; vérifier et rapporter les fuites. Perte annoncée de leur mémoire volatile, aucune suppression/prune/reset ni extinction d'un runtime partagé inconnu. Conserver et signaler les ressources sans propriété prouvée. Ni les noms ni ce README n'autorisent une cible distante/production.

## Compatibilité legacy : milestones ADR-0603 (non nominaux)

**`just deploy-m1`** valide des manifests sans cluster, pas le profil complet. Lazaret = gateway P3, **pas** Factory (P5/P6, hors scope). Les anciens parcours ci-dessous gardent leurs cibles et limites ; ils ne redirigent pas vers full.

Canon historique : [ADR-0603](../../docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md) · [contrat platform-local-v1](../../docs/platform-local-v1-implementation-contract.md).

| Milestone | Commande | Ce que ça **prouve** |
|---|---|---|
| **M1** | `just deploy-m1` | 7 namespaces, 4+1 stubs, NP plugins→Lazaret:8080, `ops/deploy/p4` autonome. YAML sains. |
| **M2** | `just deploy-m2` | Validation explicite historique `aiforall-local` : applique la carte stub puis le Job séparé `invoke-probe`, GET `/lazaret/invoke` → 200. Pas le nominal full. |
| **M3** | `just deploy-m3` | Shim Helm **tiers** Envoy Gateway lintable + tableau ci-dessous. |
| **J2** | `just deploy-j2` | Image locale `aiforall-apparatus-controller:j2` (`kind load`, pas de registry) + pod operator **Ready** (binaire `apparatus-controller`, plus `pause`). |
| **J3** | `just deploy-j3` | Overlay **démo** `ops/deploy/apps/overlays/kind-demo-monolith/` : image `aiforall-oodhive-monolith:j3` + Job POST `/lazaret/invoke` → **401** JSON. **Pas** le canon 0601 (4+1). **Pas** une réécriture de `overlays/kind` (M2 nginx). |

M2 **non nominal** : `verify-m2.ps1` applique l'overlay `kind` **sans Job au lancement**, puis supprime/recrée/attend explicitement la sonde depuis `ops/tests/manifests/invoke-probe-job.yaml`. La commande M2 est une invocation de validation explicite, pas le parcours runtime full. Son ancien skip exit 0 sur outil manquant n'est jamais l'oracle du nominal full. **Aucun reset, suppression ou redémarrage automatique de cluster.** Le parent conserve le bail runtime et assure le stop gracieux des IDs possédés en fin de tests.

J2 : `docker build` + `kind load docker-image … --name aiforall-local` + `kubectl apply -k ops/deploy/p4`. Overlay `images:` tag **non-`latest`**, `imagePullPolicy: Never`. Watch CR dans ns `apparatus-system` (inchangé côté Rust). **Ne pas retarget** `apparatus-p4-it`. Si kind/docker/kubectl manque → **STOP** (exit 1).

J3 : `docker build` + `kind load` tag **`j3`** (≠ `latest`) + `kubectl apply -k ops/deploy/apps/overlays/kind-demo-monolith` (contexte `kind-aiforall-local`). Readiness = **GET /health**. Infra DB/FGA = Compose hôte (`just up-infra`) via **`host.docker.internal`** (Docker Desktop Windows) ; pas de Postgres in-kind. Si kind/docker/kubectl ou Postgres :5432 manque → **STOP** (exit 1). Re-appliquer `just deploy-m2` restaure le stub nginx (fichiers `overlays/kind` inchangés).


**CNI :** `aiforall-local` = **Calico v3.29.7** (`disableDefaultCNI` + `podSubnet 10.244.0.0/16`). `common.ps1` initialise le pool avant le premier apply, attend Calico + CoreDNS + nodes Ready. kindnet, CIDR/pin incompatible, topologie incompatible ou propriété inconnue = **STOP**, pas de delete/recreate. L'enforcement NetworkPolicy doit être prouvé par les tests finaux. La fixture IT protégée reste inchangée.

## Bail legacy et validation statique

`powershell -NoProfile -File ops/scripts/validate-deploy-static.ps1` vérifie les racines, chemins nominaux et la syntaxe sans moteur, cluster ou cargo.

Avant M2/J2/J3/mesh, le parent renseigne `AIFORALL_RUNTIME_LEASE` avec le chemin d'un JSON non secret : `{"context":"kind-aiforall-local","nodeContainerIds":[]}` pour un cluster absent ; pour une réutilisation, les **IDs exacts** explicitement loués dans l'inventaire préalable. Le bootstrap enregistre les IDs créés. Un nom d'image/container n'est pas une preuve de propriété. Les nœuds arrêtés ne sont jamais réveillés par ce helper.

Ce bail legacy n'est **pas** le bail JSON v2 full des sections 4–5. Le profil complet et ses helpers utilisent exclusivement `kind/cluster-local-full.yaml` ; J3/gold restent hybrides sur leur contexte legacy. Le nominal full ci-dessus reste **NOT RUN** jusqu'aux preuves intégrées.

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
| Cible Kind du fixture IT | Défaut `aiforall-local` ; `APPARATUS_KIND_CLUSTER` est normalisé (trim), mais seule cette cible est autorisée. Le contexte `kind-aiforall-local`, `aiforall-local-full`, les noms protégés (`apparatus-p4-it`, `kind-apparatus-p4-it`, `rancher-desktop`) et toute autre cible sont refusés avant tout appel externe ; une valeur explicitement vide/invalide est une erreur. |
| `workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs` | Pont pédagogique M5 → Service Lazaret + plugin isolé |
| `workers/apparatus-operator/tests/apparatus_m6_e2e_chain.rs` | Pont pédagogique M6 → chaîne admit/schedule/invoke |
| Fixture `workers/apparatus-operator/tests/fixtures/kind/` | `ensure()` réutilise uniquement `aiforall-local` existant avec Calico prêt et sans kindnet ; sinon échec explicite (fail-loud). Jamais de création/suppression de cluster ni d'installation de Calico par les tests ; le YAML est une référence seule alignée sur `ops/deploy/kind/cluster.yaml`. |
