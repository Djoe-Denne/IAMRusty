# Plan QA — axe 2 : chaîne E2E Kind réelle `apparatus_m6_e2e_chain` sur `kind-aiforall-local` + re-ciblage NON destructif du fixture (P4→P5)

- **Lot** : axe 2 de la mission (ledger `.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md`, P4 `task_mvcqvggw_000l_h3ormo`). Tentatives précédentes pool-7/8/9 échouées sur discipline I/O (hang shell) — artefact écrit aussitôt, section par section. Tâche : P4-(2).
- **Propriétaire** : `qa-planner` (ce plan). Exécution : P5 (`test-engineer`), **GO utilisateur explicite requise**. Audit : P6 (`correctness-reviewer`), findings référenceront `E-<n>` de CE fichier.
- **HEADbase vérifié le 2026-10-18** : `dad816eb0c20b60e6a6e6435f9fbb2a294b78699` (`git rev-parse HEAD` ; lot G-17 clos `665110c`/`f9d0aa7`, commit `dad816e docs(qa): clos le lot G-17 apres revues P3a et P3b`). Pin rustycog gitlink `160000 41a86dba0a24844f900783e19ef2405319d58b3f` (`git ls-files --stage rustycog`), immobile. Porcelain parent : `?? .opencode/missions/` seulement.
- **Planification seule (P4)** : aucun runtime (Docker/Kind/WSL/kubectl), aucun cargo, aucun code prod ni test. Le re-ciblage est **conçu** ici, **pas codé**. Les faits runtime (état Docker/jour J) sont des gaps « à vérifier à P5 ».
- **Niveaux** : pin IT réelle (ADR 0200–0202, mocks sortants seulement) — l'axe 2 est l'IT **Kind réelle** (raison d'être de l'axe, GO utilisateur) préfacée d'un fix mécanique test-only (`chain.rs`) et d'une preuve de compilation. Aucune stratégie de test écrite possédée par des humains au-delà des ADR : l'absence reste nommée, pas comblée.

## Sommaire
- §0 Contradiction statu quo ↔ ordre de mission (règles « fixture intouchable »)
- §1 Faits vérifiés (file:line, base `dad816e`)
- §2 Design du re-ciblage (conception — pas de code)
- §3 Étape A — matrice de risques
- §4 Étape B — inventaire des tests existants
- §5 Étape C — registre de gaps E-1…E-12
- §6 Étape D — portes, commandes exactes P5, preuves par hop
- §7 Runtime et barrières P5
- §8 Budget
- §9 Incertitudes et décisions restant à prendre
- §10 Journal des mises à jour

## 0. Contradiction statu quo ↔ ordre de mission — périmètre du re-ciblage

**Fait** : le fixture courant cible le cluster **protégé** `kind-apparatus-p4-it` (`tests/fixtures/kind/mod.rs:9`) et, si le cluster existant n'est pas conforme (API down ou kindnet), le **supprime** (`delete_cluster`, `:50`→`:99-112`, `kind delete cluster --name` `:101`) puis le recrée (`:53-54`→`:1096-1111`). Le statu quo est donc : (a) inutilisable pour l'axe 2 (l'ordre cible `kind-aiforall-local`) et (b) destructif en l'état. Le handoff 0412/0605 le documentait déjà : `apparatus_m6_e2e_chain interdit : fixture … vise apparatus-p4-it et delete/recreate` (`.cursor/handoffs/20261010-0412-0605-e2e-closeout.md:71`).

**Faits — règles pré-mission « fixture intouchable »** (citées, non rouvertes en général) :
- `ops/deploy/README.md:52,55` : « Cluster IT apparatus-p4-it ≠ aiforall-local (ne pas retarget) » ; « Fixture workers/apparatus-operator/tests/fixtures/kind/ | Intouchable ; Calico + nom apparatus-p4-it ».
- `docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md:30,50` : « pas Calico sur aiforall-local ; pas retarget fixture » (borné à l'overlay J3, hors jalon).
- `docs/platform-local-v1-implementation-contract.md:88` / `platform-local-prod-mimic-implementation-guide.md:5` / `platform-local-gold-case-implementation-guide.md:94,173` : « casser fixture apparatus-p4-it » dans les NE-DOIT-PAS.
- Briefings 2026-09-26 (tests:89, correctness:63) : « ne pas proposer edits sous fixtures/kind/ ».

**Résolution de mission (décision humaine citée, non rouverte ici)** : le ledger tranchait déjà « L'axe 2 commence par un re-ciblage borné du fixture : nom paramétrable (défaut `aiforall-local`) et `ensure()` non destructif… plan QA préalable (P4), pas d'exécution avant plan commité » (ledger `:44-49`) ; P4 a été publiée après P3 sans BLOCK (ledger `:272-276`). Les règles « intouchable » pré-mission sont dépassées **pour ce re-ciblage borné seul**, sous trois conditions verrouillées : (a) ce plan commité par le lead avant toute écriture, (b) **GO utilisateur explicite avant tout run** (le run m6 touche le cluster partagé), (c) diff borné aux fichiers de fixtures (aucun site d'appel, aucun `src/`, aucun doc).

**Ce plan ne couvre PAS** (exclusions de la tâche, citées) : `kind-aiforall-local-full` (GO séparée), lab 0412 (GO séparée), réactivation m5/t10 Kind-defer G-17.10 (choix utilisateur), axe 4 implémentation, gold path 0605, promotion ADR 0412, `kind-apparatus-p4-it` et `rancher-desktop` (jamais ciblés, jamais renommés dans le plan).

Si le constat P5 est « la chaîne = une simple curl probe » (renforcement P7/axe 4 : ADR Proposed ou contrat) : **signaler, ne pas écrire ici** (porte §6.5).

## 1. Faits vérifiés (file:line, base `dad816e`)

### 1.1 Fixture Kind (`workers/apparatus-operator/tests/fixtures/kind/mod.rs`)

- `:1` doc « Cluster Kind `apparatus-p4-it` (IT T10) » ; `:9` `pub const CLUSTER_NAME = "apparatus-p4-it"` ; `:11` env binaire `APPARATUS_KIND_BIN` ; `:12` chemin codé `C:\Users\djden\bin\kind.exe` ; `:13` `IMAGE_TAG = apparatus-p4-platform-plugin:t10` ; `:15-17` Calico v3.29.7 URL GitHub pinée.
- **`ensure()` `:35-65`** : assert Docker (`:36-40`) → `resolve_kind_bin` (`:41`) → `cluster_exists` (`:42`) → si existe : export kubeconfig + probe, `reusable = API up && pas kindnet` (`:48`) ; si **non reusable → `delete_cluster()` + `create_cluster()`** (`:49-52`) ; si absent → `create_cluster()` (`:53-54`) ; puis export kubeconfig, `wait_api_server`, `install_calico_and_wait`, `apply_operator_manifests` (`:56-64`).
  - **Précision de lecture** : le test de réutilisation (`:48`) ne vérifie **pas** la présence Calico (c'est `install_calico_and_wait` qui l'applique si absent, `:220-231`) ; il vérifie API up + absence de kindnet. La formulation du ledger « API up + Calico présent » est approchée — la logique exacte est celle-ci.
- `:72-97` `apply_operator_manifests` : `kubectl apply -f {CARGO_MANIFEST_DIR}/k8s` + `wait --for=condition=Established crd/admissionrecords.apparatus.aiforall.dev --timeout=90s`.
- `:99-112` `delete_cluster` (`kind delete cluster --name {CLUSTER_NAME}` `:101`) ; `:1096-1111` `create_cluster` (`kind create cluster --name {CLUSTER_NAME} --config tests/fixtures/kind/cluster.yaml` `:1098-1100`).
- `:213-233` `install_calico_and_wait` : kindnet → **Err fail-loud** (`:214-219`) ; Calico absent → **apply depuis l'URL GitHub** (`:220-231`) + `FELIX_IGNORELOOSERPF` (`:180-210`).
- `:235-315` `wait_cni_ready` : rollouts `calico-node`/`calico-kube-controllers` + pods Ready + CoreDNS + nodes + re-check kindnet (`:308-313`).
- `:322-348` `build_and_load_platform_plugin` : `docker build -t {IMAGE_TAG} {CARGO_MANIFEST_DIR}/testdata/platform-plugin` (`:323-335`) + `kind load docker-image {IMAGE_TAG} --name {CLUSTER_NAME}` (`:336-339`) + `cri_pin_from_node` (`:1136-1152`).
- `:350-411` `probe_http_from_cluster` (pod sonde `t10-zot-probe` ; consommé par t10, **pas** par m6).
- `:427-468` `load_and_pin_image` (`kind load docker-image {tag} --name {CLUSTER_NAME}` `:456-459` + pin CRI `:1154-1183`).
- `:470-519` `ensure_zot_http_mirror` : mirror containerd `certs.d/host.docker.internal:{port}` + restart containerd + waits (consommé par t10/t11, pas m6).
- `:526-536` `assert_registry_cri_absent` (`crictl inspecti` doit échouer — anti faux vert ; t10).
- `:539-599` `push_platform_plugin_to_zot` (build nonce + `docker save` + skopeo/crane vers zot + digest 64 hex) ; `:601-604` images `skopeo:v1.17.0`/`crane:v0.20.3` + creds `signer`.
- **`export_kubeconfig` `:1113-1134`** : nom de fichier temp **hardcodé** `"apparatus-p4-it.kubeconfig"` (`:1114`, littéral, PAS dérivé du const) ; `kind export kubeconfig --name {CLUSTER_NAME}` `:1119-1121`.
- Sites de nommage du nœud : `{CLUSTER_NAME}-control-plane` `:476`, `:527`, `:1137`, `:1159` ; **eprintlns hardcodés** « apparatus-p4-it: » `:221`, `:236`.

### 1.2 Les 17 occurrences (grep exact, base `dad816e`)

| Fichier | Occurrences | Lignes |
|---|---|---|
| `tests/apparatus_m6_e2e_chain.rs` | 1 | `:59` |
| `tests/apparatus_m5_invoke_isolated.rs` | 1 | `:452` |
| `tests/apparatus_m3_install_fail_closed.rs` | 4 | `:297, :352, :471, :522` |
| `tests/apparatus_m4_desired_state_bridge.rs` | 2 | `:331, :503` |
| `tests/apparatus_p4_t10_platform_workload.rs` | 4 | `:124, :311, :391, :596` |
| `tests/apparatus_p4_t11_runtime.rs` | 4 | `:450, :472, :500, :527` |
| `tests/fixtures/kind/cluster.yaml` (commentaire) | 1 | `:5` (« Calico piné dans KindCluster::ensure ») |
| **Total** | **17** | 16 appels `ensure()` + 1 commentaire |

16 appels = les 15 tests Kind-defer du lot G-17 (G-17.10 : m3×4, m4×2, m5×1, t10×4, t11×4) + m6 (`:59`, axe 2). Aucune autre occurrence du littéral `apparatus-p4-it` sous `tests/` hors le fixture lui-même (`mod.rs:9, :221, :236, :1114` ; `cluster.yaml:3, :5`) — les autres occurrences repo-wide sont documentaires (AGENTS.md `:50`, ADR-0008 `:11,:85`, ADR-0603 `:45`, ADR-0604 `:30,:50`, 0605 `:34`, ops `:21,:52,:55`, handoffs/briefings).

### 1.3 `tests/fixtures/chain.rs` — helper périmé (prérequis mécanique)

- `:47-51` `repo_root()` = `CARGO_MANIFEST_DIR` + `join("..")` = `workers/` depuis le refactor layout `f060d47` (2026-10-03) — un niveau trop bas ; le doc-comment `:47` dit « Racine du workspace (parent de workers/apparatus-operator/) » : **l'intent est le vrai root, le code est périmé**.
- `:53-57` `reference_kv_dir()` = `repo_root().join("apparatus-reference-kv")` = `workers/apparatus-reference-kv` — **inexistante** (la vraie : `crates/apparatus-reference-kv/`).
- Consommateurs (uniques) : `m6_e2e_chain.rs:54` → `descriptor_from_disk()` → `chain.rs:62-63` `fs::read_to_string(...).unwrap_or_else(panic "apparatus.toml: …")` → **panic à 0,00 s au démarrage, avant tout Docker/Kind** (famille ROUGE G-17, éteinte par `665110c` pour m1/m3/m4/m5/t11 seulement — chain.rs était hors périmètre). Autres usages : `:100` (`passing_input`), `:220` (staging `crates/apparatus-contracts`), `:223` (`copy_tree(reference_kv_dir())`).
- Cibles réelles vérifiées (glob) : `crates/apparatus-reference-kv/apparatus.toml` + `Dockerfile.http` + `Dockerfile` existent ; `crates/apparatus-contracts/Cargo.toml` existe.
- **Mécanique du fix `665110c`** (briefing tests Q1 :37-38, briefing correctness archive) : `CARGO_MANIFEST_DIR/../..` puis `join("crates")`. Pour chain.rs : `repo_root()` `:50` → `join("../..")` ET `reference_kv_dir()` `:56` → `repo_root().join("crates").join("apparatus-reference-kv")` (2 éditions mécaniques dans CE fichier, zéro site d'appel touché). NB : `:220` `repo_root().join("crates").join("apparatus-contracts")` devient correct automatiquement post-fix (l'AIForAll/crates est la vraie). Non-toucher : `fixtures/zot/mod.rs:101`, `t6:59` (package-relatifs corrects — briefing correctness archive).

### 1.4 `tests/apparatus_m6_e2e_chain.rs` — un seul test, hops verrouillés

- **UN SEUL test** : `m6_e2e_0002_0008_chain` (`:51-53`, `#[tokio::test(flavor="multi_thread")]` + `#[serial]`). Aucun `#[ignore]` dans `tests/` (grep exact : 0 match). Inclusions : `:44-46` `fixtures` = `with_kind.rs` (re-export base T6 + `pub mod kind`), `:48-49` `mod chain`.
- Flow complet (ancres) : `:54` descriptor_from_disk (manifeste disque) ; `:55-57` assert_descriptor_shape (sha256+64hex, jamais latest) ; `:59` `KindCluster::ensure()` **fail-loud si cluster indisponible** ; `:60-62` build_and_load_platform_plugin ; `:63` docker_build_reference_kv_http (skip si tag déjà présent, `chain.rs:168-175`) ; `:64-80` load_and_pin_image + assertions pin (`@sha256:` présent, sans `latest`, ≠ digest catalogue) ; `:82-84` **SignRegistryStack::start (zot réel + Transit)** ; `:85-121` push_and_sign_envelope réel + assertions (verify positif `:97-100`, artifact_type `:101`, digest OCI enveloppe ≠ catalogue ≠ pin CRI `:102-121`) ; `:123-145` passing_input + stub Manifesto (list_ready == [ReadyBinding] digest 0002) ; `:147-175` **store JSON disque** (admit → AdmissionStatus::Valid `:149-151`, bind_cri `:152-154`, bind_envelope `:155-157`, would_schedule `:158-161`, lecture brute du fichier `contains("VALID") && contains(catalog)` `:163-168`, ré-open + get VALID `:169-175`) ; `:177-190` WorkloadReconciler::connect(kubeconfig) + pré-suppression pod/CR homonymes ; `:192-209` reconcile_ready → **Scheduled** (pod_name exact, refus = panic) + wait_pod_running 180 s ; `:211-246` pod image == pin CRI + labels project/binding ; `:248-263` CR `admissionrecord` `spec.descriptorDigest` == catalogue ; `:265-285` **port-forward** `PLUGIN_INVOKE_PORT` + wait_tcp + CountingLocator ; `:287-315` enroll + session Lazaret réels ; `:349-366` **deny → HTTP 403 + hops==0** ; `:368-390` **allow → HTTP 200 + hops≥1 + `result.hostname` == pod_name** ; `:392-425` **canary T11** : `kubectl exec wget -S -T5 CANARY_URL` → jamais reached (`wget_reached_target` : HTTP 200/404 = échec) + signal blocage ou échec du wget + « wget: not found » = refus ; `:427-436` wait canary Ready best-effort (résultat ignoré) ; `:437-438` drop(port-forward, stack).
- `CANARY_URL` = `http://apparatus-system-canary.apparatus-system.svc.cluster.local:8080/` (`chain.rs:44-45`) ; `CANARY_LABEL` = `app=apparatus-system-canary` (`:42`).

### 1.5 Canon et état du cluster partagé `aiforall-local`

- `ops/deploy/kind/cluster.yaml:3` `name: aiforall-local` ; `:4` **Calico v3.29.7 installé par `deploy-j2.ps1`** ; `:6` « kindnet interdit : n'enforce pas NetworkPolicy ; conflit = STOP, jamais de reset » ; `:8` podSubnet local `10.244.0.0/16` (host.docker.internal=192.168.65.254 traité comme pod par Calico) ; `:10` `disableDefaultCNI: true` ; `:13-14` node control-plane `kindest/node:v1.32.2@sha256:142f5435…` piné.
- Ledger `:189-190` : Docker tier `aiforall-local-control-plane` (kindest/node) **Up ~2 h au 2026-10-17, non possédé** — fait runtime à re-vérifier à P5 le jour J (froid possible après redémarrage Docker).
- `AGENTS.md:50` : liste close des contextes diagnostiques = `kind-aiforall-local`, `kind-aiforall-local-full` ; **jamais `kind-apparatus-p4-it` ni `rancher-desktop`**.
- ADR-0008 `:11` : la chaîne M1–M6 est la preuve P4-core historique (A-DEC 2026-09-22) sur l'IT `apparatus-p4-it` + Calico v3.29.7 — le run sur `aiforall-local` est un **nouveau contexte de preuve** ; la preuve historique reste valide et intouchée.
- `ops/deploy/README.md:21` : J2 = `docker build` + `kind load docker-image … --name aiforall-local` + `kubectl apply -k ops/deploy/p4`.
- **Compatibilité nommage nœud** (inférence, confirmable jour J seulement) : le fixture calcule le nœud comme `{CLUSTER_NAME}-control-plane` (`:476,:527,:1137,:1159`) ; pour `aiforall-local` → `aiforall-local-control-plane` — EXACTEMENT le nom du container Docker constaté au ledger. Cible du calcul cohérente par construction kind (1 node control-plane dans cluster.yaml).

### 1.6 Inventaire manifests operator (`workers/apparatus-operator/k8s/`, appliqués par `apply_operator_manifests`)

| Fichier | Contenu | Effet sur le cluster |
|---|---|---|
| `00-namespaces.yaml` | ns `apparatus-system` (label `apparatus.aiforall.dev/ns: system`) + `apparatus-plugins` (label `…/ns: plugins`) | additif (si absents) / patch labels (si présents) |
| `01-serviceaccounts.yaml` | 4 SA `apparatus-build` (automount false), `apparatus-admit`, `apparatus-controller`, `apparatus-gateway` dans `apparatus-system` | additif |
| `02-rbac.yaml` | Role+RoleBinding `apparatus-admit` (ns system : CR create/get/update/patch) ; Role `apparatus-controller` ×2 (ns system : CR get/list/watch ; ns plugins : pods/services/jobs) + RoleBindings | additif |
| `03-crd-admissionrecord.yaml` | CRD `admissionrecords.apparatus.aiforall.dev` v1alpha1 (spec.descriptorDigest/policyVersion/reportDigest requis, status.phase) | cluster-scoped, idempotent |
| `04-networkpolicy.yaml` | `plugins-isolate` (ns plugins : ingress+egress restreints, DNS kube-system UDP/TCP 53, intra-ns, gateway :8080) + `system-deny-plugins` (ns system : ingress uniquement ns system) | additif, **détermine l'oracle canary** |
| `05-system-canary.yaml` | Deployment `apparatus-system-canary` (image `apparatus-p4-platform-plugin:t10`, **`imagePullPolicy: Never`** `:22-23`, `httpd -f -p 8080`, readiness tcp) + Service `apparatus-system-canary:8080` | additif ; pod canary ImagePullBackOff jusqu'au `kind load` de l'image |

Consts consommées par m6 : `SYSTEM_NAMESPACE=apparatus-system` (`controller.rs:35`), `PLUGINS_NAMESPACE=apparatus-plugins` (`:37`), `PLUGIN_INVOKE_PORT=8080` (`:39`).

### 1.7 Collisions nomées avec l'overlay plateforme J2 (`ops/deploy/p4/`) — analyse hors-machine

- **CRD homonyme identique** : `ops/deploy/p4/crd-admissionrecord.yaml:1-2` = « Copie locale de workers/apparatus-operator/k8s/03-crd-admissionrecord.yaml (source IT, lecture seule) » — contenu identique (lecture des deux fichiers) → re-apply du CRD fixture = no-op matériel. Reconfirmation jour J : `kubectl get crd admissionrecords.apparatus.aiforall.dev -o yaml` avant/après (lecture seule).
- **Namespaces homonymes, labels différents** : J2 crée `apparatus-system`/`apparatus-plugins` (namespace.yaml `:25,:33`) avec labels J2 (`app.kubernetes.io/part-of`, `aiforall.io/trust-plane`, `…/ns: plugins`) ; le fixture applique les mêmes ns avec **moins** de labels → un `kubectl apply` client-side peut **effacer les labels J2** (last-applied-configuration) — mutation superficielle mais réelle d'objets partagés (les netpols J3 peuvent matcher `trust-plane`). Mitigation = D-6 (apply server-side, field-manager dédié) + inventaire avant/après (§6.4).
- **SA : aucune collision** — J2 = `controller`/`admit-sign`/`build` dans `aiforall-apparatus` (serviceaccounts.yaml `:6,:16,:26`) ≠ SA fixture `apparatus-*` dans `apparatus-system`.
- **Opérateur plateforme actif** : J2 déploie `aiforall-apparatus-controller:j2` (operator.yaml, tag `:17-19` dans kustomization) qui watch les AdmissionRecords dans `apparatus-system` → co-existence avec le reconciler in-test de m6 (pod/CR homonymes par design : même code de nommage, même spec image issue du CR) — risque R-08, attendu bénin, constaté jour J.
- **Hors chemin m6** : `zot.yaml`/`Dockerfile.zot` (zot in-cluster J2 ≠ zot testcontainers), `admit-job.yaml` (prove-gold, « pas au deploy-j2 » kustomization `:15`), `allow-admit-zot-np.yaml` (nom ≠ `plugins-isolate`/`system-deny-plugins`) → additif pur, non appliqué par le fixture.
- **Canary** : AUCUN canary dans l'overlay J2 (12 fichiers, aucun `*canary*`) → création propre par le fixture, pas de collision.


## 2. Design du re-ciblage (conception — PAS de code dans ce lot)

**Séparation** : D-1…D-5 = design fixé par ce plan (issus des oracles 1–3 de la tâche) ; D-6 = **recommandation** (décision lead à l'intégration) ; D-7 = barrières (décisions humaines déjà prises). Ce qui suit n'est pas du code : c'est le contrat que P5 implémente et que P6 audite.

### D-1 — Point de nommage unique

- La constante `CLUSTER_NAME` (`:9`) devient un point unique de résolution : **défaut "aiforall-local"** (contexte `kind-aiforall-local`), overridable par la variable d'environnement `APPARATUS_KIND_CLUSTER` (variable proposée ; nom exact figé à l'implémentation, sans nouvelle dépendance crate).
- Tous les sites internes du fixture dérivent de CE point : `kind load` `:337`/`:457`, nœuds `{name}-control-plane` `:476/:527/:1137/:1159`, `kind create` `:1099` (voir D-2 : chemin supprimé), `kind export kubeconfig --name` `:1119-1121`, nom de kubeconfig temp `:1114` (**aujourd'hui un littéral indépendant — à dériver du même point**), eprintlns `:221`/`:236` (littéraux — à dériver), `cluster.yaml:3` (name + commentaire `:5` mis à jour ; le fichier reste consommé uniquement par le chemin de création, supprimé du flux ensure — cf. D-2).
- **Invariant** : `kind-apparatus-p4-it` n'apparaît plus NULLE PART dans le fixture (grep post-fix = 0 dans `tests/fixtures/` ; le cluster protégé n'est jamais ciblé).

### D-2 — `ensure()` non destructif (nouvelle sémantique)

1. `assert_docker_running` conservé (`:36-40`) — fail-loud Docker.
2. `resolve_kind_bin` conservé (`:41`, `:1041-1064`).
3. `cluster_exists` (`kind get clusters`, `:1080-1094`) : **absent → Err explicite** (« cluster {name} absent — démarrer le runtime est une opération runtime, pas un effet de test »). **Jamais de `kind create` depuis le fixture**.
4. Export kubeconfig (nom dérivé du point unique).
5. `wait_api_server` borné 180 s (`:143-170` conservé) : injoignable → Err fail-loud.
6. `has_kindnet` → **Err conservé** (`:135-137`, `:214-219`) — conformité `ops/deploy/kind/cluster.yaml:6`.
7. **`has_calico_node` absent → Err** (nouveau) : **jamais d'installation Calico depuis le fixture** sur le cluster partagé — l'installation Calico appartient au propriétaire du cluster (`deploy-j2.ps1`, cluster.yaml `:4`) ; absent = fail-loud STOP (oracle 5 de la tâche).
8. `wait_cni_ready` conservé (`:235-315`, lecture/waits seulement).
9. `apply_operator_manifests` conservé (`:72-97`, additif, test-owned) — variante D-6.
**Suppressions** : `delete_cluster` (`:99-112`) ; `create_cluster` (`:1096-1111`) ; la branche d'installation Calico (`:220-231`). Le flux devient strictement : Docker → binaire kind → cluster présent → API up → CNI conforme → Ready → manifests. **« Prêt → réutilise ; pas prêt → fail-loud »** : toute divergence = Err, jamais une remédiation destructive.

### D-3 — Invariants de compatibilité (16 sites d'appel + 1 commentaire)

- AUCUN des 16 appels `KindCluster::ensure()` (§1.2) n'est modifié ; aucune signature publique ne change (`ensure()`, `kubectl`, `spawn_kubectl`, champ public `kubeconfig`).
- Preuves attendues post-fix : (a) `git diff` borné à `tests/fixtures/kind/mod.rs` + `tests/fixtures/kind/cluster.yaml` + `tests/fixtures/chain.rs` (3 fichiers, aucun `src/`, aucun fichier IT) ; (b) compilation des cibles IT (E-4) qui lie les 16 sites ; (c) grep nominal : 16 occurrences inchangées pré/post.

### D-4 — kubeconfig et collatéraux

- Fichier kubeconfig temp = `{nom}.kubeconfig` dérivé du point unique (jamais un deuxième littéral) ; l'ancien fichier `apparatus-p4-it.kubeconfig` éventuellement présent dans `%TEMP%` n'est **ni lu ni supprimé** (hors périmètre).

### D-5 — Portage IT Linux CI (inférence, à confirmer)

- `ops/deploy/kind/cluster.yaml:5` mentionne « IT Linux CI garde 192.168.0.0/16 » (subnet du fixture) : un usage CI du fixture est plausible mais non démontré dans ce lot. Le défaut `aiforall-local` + override env `APPARATUS_KIND_CLUSTER` couvre les deux sans brancher le code. **Inférence non confirmée** — si le CI est réel, sa config env reste un lot dédié ; ce lot ne touche pas la CI.

### D-6 — Apply server-side avec field-manager dédié (recommandation — décision lead)

- Risque mesuré §1.7 : un `kubectl apply` client-side peut effacer des labels posés par J2 sur les ns homonymes (`apparatus-system`/`apparatus-plugins`). **Recommandation** : variante `kubectl apply --server-side --field-manager=apparatus-it-fixtures` dans `apply_operator_manifests` — préserve les champs possédés par d'autres managers, garde l'additivité. Si le lead refuse (simplicité), fallback = apply client-side + inventaire avant/après des labels des 2 ns + restauration J2 (re-apply du fichier ns) **uniquement** sur décision utilisateur. Décide : lead à l'intégration P5.

### D-7 — Barrières (décisions humaines déjà prises)

- Jamais `kind-aiforall-local-full` (GO explicite seule) ; jamais `kind-apparatus-p4-it` / `rancher-desktop` (intouchés, jamais ciblés par le nom) ; jamais `kind delete`, `kubectl delete namespace`, prune, `down -v` ; jamais d'install Calico depuis le fixture (D-2.7) ; jamais de `docker rm -f` hors le `docker_rm` ciblé du code fixture existant (hors chemin m6).

## 3. Étape A — Matrice de risques du lot

Impact (gravité d'un échec) × probabilité (churn, branches, catégories historiques du dépôt : mock qui masque l'invariant, races, rotation token, scoping — briefing test-reviewer) scorés séparément. Churn git faible sur `fixtures/kind/`+`chain.rs` depuis `665110c` (0 commit) ; la probabilité vient surtout du **premier run réel de m6** et des objets partagés du cluster.

| Id | Comportement / contrat | Impact | Prob. | Score | Zone | Profondeur attendue |
|---|---|---|---|---|---|---|
| R-01 | Re-ciblage résiduel destructif : `delete_cluster`/`create_cluster` ou install Calico survivants dans le flux ensure → remédiation automatique contre `aiforall-local` | 5 | 3 | **15** | critique | design D-2 + audit diff + comptage flux (E-1/E-2) |
| R-02 | Collision/overwrite sur objets homonymes J2 : strip de labels des ns, CRD, RBAC/SA (§1.7) | 4 | 3 | **12** | haut | inventaire avant/après + D-6 (E-2) |
| R-03 | Fix mécanique `chain.rs` : ancre faux (mauvaise racine/chemin) | 3 | 2 | 6 | moyen | fix test-only + asserts loud existantes |
| R-04 | Skip silencieux : cible non compilée (features manquantes) → 0 test avec exit 0 | 4 | 2 | 8 | moyen | preflight `--list` + comptage rendu (E-4) |
| R-05 | Oracles par hop de m6 : admission JSON VALID disque, schedule réel, invoke 403/200, canary — preuve fausse (tautologie, mock du hop) | 5 | 3 | **15** | critique | IT Kind réelle + comptage + compteur hops (E-6…E-11) |
| R-06 | CNI du cluster partagé sans netpol (kindnet) → oracle canary infalsifiable | 4 | 2 | 8 | moyen | in-test (`wget_reached_target` catch) + preflight `kubectl get ds` fail-loud (E-11) |
| R-07 | Budget froid : builds Docker ×2 + pulls + `kind load` + compile m6 (kube/k8s-openapi, lazaret, axum-test) jamais compilée | 2 | 4 | 8 | moyen | budget §8, buffer |
| R-08 | Co-régulation : opérateur plateforme J2 (`aiforall-apparatus-controller`) + reconciler in-test sur les mêmes CR/pod nommés | 3 | 3 | 9 | moyen | constat jour J (§6.4), attendu bénin (même spec) |
| R-09 | Idempotence re-run m6 : pré-suppression pod/CR (`:183-190`), TempStorePath isolé, zot singleton | 2 | 2 | 4 | bas | fumée re-run (buffer) |

Score PM (P4 : complexité 50, risque 50, manuels) : la matrice concentre la criticité sur le **blast radius cluster partagé** (R-01/R-02) plutôt que sur la complexité de code — cohérent avec la tâche, aucune contradiction structurelle à trancher : signalé, board non modifié.

### Modes de défaillance (items ≥ 10)

| Id | Déclencheur | Blast radius | Détection actuelle | Mitigation actuelle | Gap |
|---|---|---|---|---|---|
| R-01 | Un site de remédiation (`delete_cluster` `:50`, `create_cluster` `:53-54`, apply Calico `:220-231`) survit au re-ciblage et s'exécute contre `aiforall-local` (ex. : API momentanément down ou kindnet détecté → delete du cluster partagé) | Destruction du cluster plateforme local partagé, non possédé — perte d'état pour toute l'équipe (non possédé ⇒ aucune reprise garantie) | AUCUNE automatisée (le fixture ne distingue pas partagé/protégé) | AUCUNE à ce jour | **E-1/E-2** : flux ensure borné + audit diff qui prouve l'absence de toute voie destructive |
| R-02 | Apply client-side sur ns homonymes J2 (labels différents §1.7) ; CRD différé ; RBAC/SA inattendus | Strip de labels `trust-plane`/`part-of` sur 2 ns ; CRD schema écrasé si divergent (contenu actuellement identique) | Aucune (le fixture n'inventorie rien) | Aucune | **E-2** : inventaire avant/après + D-6 server-side (décision lead) |
| R-05 | Chaîne m6 faussement verte : store mémoire au lieu de disque, `would_schedule` contourné, invoke sans port-forward réel (hops compteur triché), canary oracle affaibli (200/404 accepté) | La preuve P4-core serait ré-attestée à tort sur un nouveau cluster — érosion de confiance ADR-0008 (état actuel : un seul test m6 jamais exécuté, aucune preuve récente) | Asserts in-test existants et denses (§1.4) ; comptage libtest | Aucune additionnelle | **E-5…E-11** : commandes exactes + comptage rendu + compteur hops + ROUGE protocol |

### Passe heuristique (états / frontières / ordre / idempotence)

- **États** : cluster prêt/absent/non-conforme (D-2) ; zot/transit singleton warm ou froid ; store JSON frais vs résidus homonymes (pré-suppression `:183-190`) ; canary `ImagePullBackOff` transitoire jusqu'au `kind load` (benin, wait `:427-436` best-effort) ; Docker tiers Exited vs Up.
- **Frontières** : kubeconfig exportable/injoignable ; bind du port local `:0` éphémère (`:265-268`) ; `hops==0` vs `hops>=1` ; digest catalogue ≠ digest CRI ≠ digest enveloppe (3 identités distinctes assertées `:76-120`) ; wget présent dans l'image CRI (`:414-417`).
- **Ordre** : push→sign→admit→schedule→invoke→canary séquencé par le code du test (§1.4) ; pré-requis compile (`--list`) AVANT tout run Kind (E-4) ; skill lifecycle AVANT tout démarrage (§7).
- **Idempotence** : re-run m6 = pré-suppression bornée pod/CR (`:183-190`, noms dérivés du digest — jamais des noms d'autrui), apply idempotent, store temp unique (`TempStorePath` `:118-147`) ; re-run OK attendu (R-09, buffer).

## 4. Étape B — Inventaire des tests existants

- **Le lot contient 1 test à exécuter** : `m6_e2e_0002_0008_chain` (§1.4) — il porte déjà les oracles de tous les hops ; P5 n'écrit AUCUN test sauf gap découvert à l'exécution (buffer, décision lead).
- **Fix mécanique préalable** (E-3) : `chain.rs` (2 éditions) — pas un test, un prérequis de compilation de la cible m6. Pattern = celui de `665110c` (m1/m3/m4/m5/t11) ; les asserts loud existantes restent la détection (aucune dilution).
- **Couverture par niveau** : IT Kind réelle (m6, 1 test) — c'est le SEUL niveau exécuté ; IT testcontainers (t6 9/9, m1 1/1) : preuves transport de référence déjà persistées lot G-17 (aucun re-run, briefing tests « Do not redo ») ; statiques m3/m4/t11 filtrés : déjà verts sur master (logs `.tmp/g17-p2fix-master-*`) — pas re-jugés ici.
- **Defer inchangé** : les 15 tests Kind-defer G-17.10 (m3×4, m4×2, m5×1, t10×4, t11×4) restent **non exécutés** dans ce lot (l'ordre n'exécute que m6) — le re-ciblage les rend **compilables** (E-4 prouve la liaison), pas exécutés. Aucun `--include-ignored`, aucun filtre qui les masquerait en pass silencieux.
- Orphelins / preuves périmées : aucun dans le périmètre (les 15 tests Kind ne sont pas orphelins — leur déférage est tracé G-17.10, briefing tests Q2/Q3) ; statiques hors axes m2/t12/t2/t7 : résidus décision utilisateur, inchangés.

## 5. Étape C — Registre de gaps

Toutes les obligations sont **explicites** (oracles 1–7 de la tâche P4-(2)), sauf E-9 (inférence structurelle : le kubeconfig exporté par le fixture alimente le client kube in-test — confirmée par lecture `m6:177-180`). L'ancêtre = G-17.10/G-17.11 (GAP-defer) du plan G-17 ; ce lot comporte la preuve E2E sur un cluster autorisé.

| Gap | Obligation (source) | Intent de test / preuve positive | Preuve existante | État | Priorité | Niveau | Effort |
|---|---|---|---|---|---|---|---|
| E-1 | Re-ciblage nommé : défaut `aiforall-local`, override env, point unique (oracle 1, explicite) | Design D-1 ; preuve = diff borné 3 fichiers + grep `apparatus-p4-it` = 0 dans `tests/fixtures/` | néant (contradiction §0) | **GAP** | critique | fix test-only (fixture) | M |
| E-2 | `ensure()` non destructif : prêt→réutilise, pas prêt→fail-loud, jamais delete/create/install (oracle 1, explicite) | Diff prouve suppression `delete_cluster`/`create_cluster`/apply-Calico ; flux = 9 étapes D-2 ; inventaire cluster avant/après (§6.4) ; négatif naturel « cluster absent → Err explicite » si rencontré | néant | **GAP** | critique | fix test-only + inventaire jour J | M |
| E-3 | Fix mécanique `chain.rs` (`repo_root` `:50` + `reference_kv_dir` `:56`) sans toucher les asserts (oracle 2, explicite) | Diff = 2 éditions ; anchors invariants (`:62-63`, `:100`, `:220`, `:223` inchangés) | néant (famille ROUGE pré-fix, éteinte pour les 5 frères seulement) | **GAP** | haut | fix test-only | S |
| E-4 | Preuve de compilation AVANT tout run Kind + comptage exact (oracle 2, explicite) | `cargo --test apparatus_m6_e2e_chain -- --list` → exactement 1 test `m6_e2e_0002_0008_chain` ; lie les 16 sites d'appel ; aucune compilation du reste | néant (m5 N-1 : jamais compilée ; m6 jamais lancé) | **GAP** | critique | compile-only (slot Windows) | S |
| E-5 | Commandes exactes P5 : features `admit,controller` (Cargo.toml `:71-72`, vérifié), `--locked`, `--test-threads=1`, UN seul cargo (oracle 3, explicite) | Désignation §6.2 ; piége skip silencieux traité par E-4 | néant | **GAP** (désignation) | haut | process | — |
| E-6 | **H1 pin zot réel** : enveloppe poussée+signée sur zot réel (SignRegistryStack `:82-84`), verify positif, digest OCI ≠ catalogue ≠ pin (oracle 4, explicite) | Extraits du run : `catalog_digest == descriptor`, `signature_verified`, `artifact_type`, tri-identités distinctes (`:93-121`) | transport déjà prouvé t6 9/9 (lot G-17) — re-prouvé ici DANS le contexte m6 sur le cluster cible | **GAP** | critique | IT Kind réelle | M (inclus au run) |
| E-7 | **H2 JSON VALID persisté disque** + ré-open (oracle 4, explicite) | Lecture brute du fichier `contains("VALID") && contains(catalog)` (`:163-168`) + `get` == Valid (`:169-175`) + bind_cri/bind_envelope (`:152-157`) | néant (m6 jamais exécuté) | **GAP** | critique | IT Kind | S |
| E-8 | **H2b would_schedule + schedule réel** : Scheduled avec pod_name exact (oracle 4, explicite) | `would_schedule` (`:158-161`, `:175`) + `reconcile_ready` → Scheduled ≠ Refused (`:194-205`) + wait_pod_running 180 s + pod image == pin + labels + CR digest (`:206-263`) | néant | **GAP** | critique | IT Kind | M |
| E-9 | **H3 invoke Lazaret via port-forward réel** : deny 403 + hops==0 ; allow 200 + hops≥1 + `hostname == pod_name` (oracle 4, explicite) | HTTP 403/200 rendus + compteur `CountingLocator` (`:283-285`, `:362-365`, `:381-390`) ; kubeconfig in-test = export du fixture (inférence confirmée lecture `:177-180`) | néant | **GAP** | critique | IT Kind | M |
| E-10 | **H4 canary T11 wget bloqué, jamais reached** (oracle 4, explicite) | `kubectl exec wget -S -T5` → `wget_reached_target` = faux (200/404 refusés) + signal blocage ou échec + wget présent (`:392-425`) | néant | **GAP** | critique | IT Kind | S |
| E-11 | Aucun skip silencieux + fail-loud cluster indisponible + ROUGE protocol (oracles 4–5, explicites) | Comptage libtest rendu par le run (`1 passed; 0 failed`) ; absence de run ≠ pass ; tout FAIL = preuve ROUGE persistée (repro + sortie + file:line) avant tout correctif ; bug prod = ROUGE → P8 | convention G-17 éprouvée | **GAP** | critique | protocol | — |
| E-12 | Runtime/barrières jour J : lifecycle skill, Docker tiers Up constaté, inventaire avant/après, registre Docker du jour, jamais full/protégés/Calico-install (oracle 5, explicites) | Checklist §7 + §6.4 ; registre `.tmp/docker-resources-20261018.log` (ressources créées/refroidies post-17) | ledger `:189-190` (tiers Up au 2026-10-17) — re-vérifier jour J | **GAP** | haut | protocol | S |

**Séparation faits / inférences / recommandations / décisions** : §1 = faits ; D-1…D-5 = design issu des oracles (obligations explicites) ; D-5/D-6/D-7 inférences et recommandations marquées ; E-9 contient une inférence confirmée par lecture ; le GO utilisateur (porte §6.1) et le choix D-6 = décisions humaines. Aucun état COVERED : la preuve passe par l'exécution P5 — un registre utilisable contient ici uniquement des GAP actionnables.

## 6. Étape D — Critères d'entrée / sortie, commandes exactes P5

### 6.1 Porte d'entrée (vérifiée par le lead avant P5)

- **Ce plan commité par le lead** après relecture (aucun commit par ce worker).
- **GO utilisateur explicite pour P5** — condition absolue (le run m6 touche le cluster partagé `kind-aiforall-local` : manifests operator additifs + pod/CR test). Pas de GO = pas de run, pas de `kind`, pas de `kubectl`, pas de Docker de prod de test.
- Base : `dad816e` propre (porcelain = `?? .opencode/missions/` seulement) ; pin rustycog `41a86dba` immobile ; `.cargo/config.toml` intouché.
- Slot Cargo Windows **libre** : un seul `cargo` à la fois, séquentiel, jamais parallèle avec un autre travail cargo du lead ou d'un autre membre.
- Fixtures/données : rien à créer — manifests `k8s/` en repo, creds dans les fixtures (`signer`, root token dev), images construites par les fixtures elles-mêmes.
- Images/pulls requis (froid) : `ghcr.io/project-zot/zot:v2.1.8`, `openbao/openbao:2.6.2` (testcontainers), `quay.io/skopeo/stable:v1.17.0` ou `gcr.io/go-containerregistry/crane:v0.20.3` (un des deux, pull si absent `:613-630`), builds locaux `apparatus-p4-platform-plugin:t10` (testdata) et `apparatus-reference-kv:m6` (staging chain.rs), et Calico **déjà** Ready sur le cluster partagé (aucun install).

### 6.2 Commandes exactes P5 (oracle 3 — hôte Windows, ordre séquentiel, UN SEUL cargo)

```text
# 0) preflight lecture (SANS cargo) — voir §6.4 : docker ps, kind get clusters, kubectl get ds/get ns/get deploy/get crd (lecture seule)

# 1) preflight compile + comptage exact (E-4) — PAS d'exécution de test :
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --list
# attendu : exactement 1 test listé : m6_e2e_0002_0008_chain (comptage rendu = preuve pas de skip silencieux)

# 2) run m6 (APRÈS GO utilisateur + preflight vert) :
cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --test-threads=1
# attendu : "test result: ok. 1 passed; 0 failed; ..." — comptage recopié intégralement
```

- Features **fixées depuis `Cargo.toml:70-72`** (`required-features = ["admit", "controller"]`, vérifié par lecture — pas d'inférence). Aucun `--all-features`, aucun `--include-ignored`, aucun filtre de nom : le fichier entier n'a qu'UN test, exécuter la cible = exécuter ce test exactement.
- `--test-threads=1` : le test est `#[serial]` et multi_thread ; threads=1 supprime tout entrelacement de logs captures.
- Piège skip silencieux (hérité du plan G-17 §1.5) : avec features manquantes, cargo saute la cible avec un warning seulement — la preuve est le **comptage rendu** (`--list` puis `1 passed`), jamais l'absence d'erreur.
- target-dir `E:\cargo-target\AIForAll` : mécanisme **externe au dépôt** (constat G-17 §1.5/§9.3 — absent du `.cargo/config.toml` committé) ; NE PAS modifier `.cargo/config.toml` ; le mécanisme d'environnement du jour J reste du ressort du lead/host (barrière inchangée).
- Le binaire m6 compile aussi avec `#[serial]` partagé des autres cibles ? Non pertinent : chaque `[[test]]` est un binaire séparé ; le slot Windows fait un cargo à la fois.
- m5 et t10 : **non exécutés** (Kind-defer G-17.10, choix utilisateur hors lot) — le `--list` m6 ne les compile pas (cibles séparées, features distinctes).

### 6.3 Preuves par hop du run m6 (oracle 4 — extraits recopiés par hop, ROUGE protocol)

| Hop | Oracle précis | Ancres m6 / fixtures | Preuve à recopier dans le Journal |
|---|---|---|---|
| H1 pin zot réel | Enveloppe poussée+signée sur zot réel ; verify positif ; 3 identités distinctes (catalogue / pin CRI / digest OCI) | `:82-84` stack ; `:85-92` push+sign ; `:93-121` asserts | Ligne `signature_verified` + digest OCI `sha256:` 64 hex + catalogue 0002 (`eprintln` `:57`) |
| H2 JSON VALID persisté (disque) | Store fichier (pas mémoire) : VALID + digest catalogue dans le raw ; ré-open == Valid | `:147-175` (+ `chain.rs:118-147`) | Extrait du fichier JSON (contient VALID + digest) + `bind_cri`/`bind_envelope` Ok |
| H2b schedule réel | `would_schedule` vrai ; reconcile → Scheduled (pod_name exact, jamais Refused) ; pod Running ≤180 s ; image == pin CRI ; labels project/binding ; CR `descriptorDigest` == catalogue | `:158-161`, `:175`, `:194-209`, `:211-263` | jsonpath image + labels + CR digest recopiés |
| H3 invoke Lazaret (port-forward) | deny → HTTP **403** ET compteur hops == **0** ; allow → HTTP **200** ET hops ≥ **1** ET `result.hostname` == pod_name | `:265-285` pf+compteur ; `:349-366` deny ; `:368-390` allow | Statuts 403/200 + valeur `hops` avant/après + hostname du body |
| H4 canary T11 bloqué | `wget -S -T5 CANARY_URL` : jamais `wget_reached_target` (HTTP 200/404 = échec du test) + signal blocage ou exit non-zéro + « wget: not found » = refus | `:392-425` (+ `chain.rs:537-552`) | Sortie wget brute (stdout+stderr) recopiée |

**Protocole ROUGE** (hérité G-17, oracle 5) : tout FAIL = preuve persistée (sortie complète, repro, file:line) **avant** tout correctif ; un test qui révèle un bug prod reste ROUGE jusqu'à un lot correctif dédié (P8) ; **aucun skip silencieux** — si un hop ne peut pas s'exécuter (CNI absent, cluster indisponible, image absente), c'est un fail-loud tracé, jamais un pass ou un omit.

### 6.4 Inventaire cluster jour J (lecture seule, avant/après)

- AVANT (preflight) : `docker ps` (tiers `aiforall-local-control-plane` Up ?) ; `kind get clusters` ; `kubectl get ds -n kube-system` (calico-node ? kindnet ?) ; `kubectl get ns` ; `kubectl get crd admissionrecords.apparatus.aiforall.dev` ; `kubectl get deploy -n apparatus-system` (canary préexistant ? opérateur J2 ?) ; `kubectl get sa,role,rolebinding -n apparatus-system` ; labels des 2 ns (`kubectl get ns apparatus-system -o yaml` — labels J2 §1.7) ; `kubectl get pods -n apparatus-plugins` (résidus ?).
- APRÈS (delta borné) : les mêmes lectures + liste des objets créés/modifiés par le fixture (CRD no-op attendu, 2 ns labels, SA/RBAC additifs, netpols, canary deploy+svc, pod m6 + CR test) ; delta = créations attendues, mutations = seulement labels ns (D-6 fallback) — tout autre delta = STOP + escalation.
- Résidus acceptés et tracés : pod m6 + CR AdmissionRecord restés en place post-run (le test ne nettoie pas après lui, `:427-438`) — delta enregistré dans l'inventaire APRÈS ; suppression ciblée (pod/CR exactement nommés) UNIQUEMENT sur autorisation lead (jamais de delete namespace).

### 6.5 Porte de sortie (auditée par P6 `correctness-reviewer`)

- E-1…E-4 cochés : diff re-ciblage borné (3 fichiers fixtures) audit + preuve compile `--list` (1 test rendu).
- m6 **1/1 vert** (premier run ou triage documenté) avec les preuves par hop H1/H2/H2b/H3/H4 recopiées (§6.3).
- Inventaire avant/après rendu, delta borné, registre `.tmp/docker-resources-20261018.log` à jour.
- Aucun FAIL sans preuve ROUGE persistée avant correctif ; aucun skip silencieux ; m6 fail-loud si cluster indisponible (E-11).
- Constats de renforcement (si « chaîne = simple curl probe » : netpol non prouvé, isolation faible) → **signal P7/axe 4** (ADR Proposed ou contrat) — jamais écrit dans ce plan ni dans le code du lot.
- P6 rendu : PASS ou findings traités, références `E-<n>` de CE fichier.

## 7. Runtime et barrières P5 (oracle 5)

- **Skill `.agents/skills/local-runtime-lifecycle/SKILL.md` chargé AVANT tout** démarrage Docker/Kind/WSL (lead et P5 ; canon `.cursor/rules/local-runtime-lifecycle.mdc` + `.cursor/rules/infra-safety.mdc`).
- **Docker tiers Up constaté par P5 le jour J** (fait runtime, jamais posé ici) : si Docker Desktop arrêté → démarrage sous lifecycle skill ; le tiers `aiforall-local-control-plane` Up au 2026-10-17 (ledger `:189-190`, non possédé) est un constat daté, à re-vérifier (froid possible). Le re-ciblage (D-2) **réutilise** le cluster s'il est prêt, sinon fail-loud — jamais de delete, jamais de create spontané.
- **Inventaire cluster avant/après** : §6.4 (lecture seule). Aucun `kubectl exec` hors celui du test, aucun `get secret`, aucune commande destructive.
- **Registre Docker append-only du jour** : `.tmp/docker-resources-20261018.log` pour les ressources **créées ou refroidies post-17** (les paires zot/openbao Exited(0) du 2026-10-17 restent dans le registre de ce jour-là) — id à la création, `safe_to_delete` ; les ressources déjà Up non possédées restent hors possession (handback, pas d'arrêt sauf bail parent).
- **JAMAIS `kind-aiforall-local-full`** (autorisée seulement par GO explicite utilisateur, dossier 0412 séparé) ; **JAMAIS `kind-apparatus-p4-it` ni `rancher-desktop`** (protégés : jamais ciblés, jamais renommés, jamais lu via kubeconfig) ; **jamais d'install Calico sur le cluster partagé** (lecture `kubectl get ds` ; calico-node absent = fail-loud STOP — la conformité Calico appartient à `deploy-j2.ps1`).
- **Aucun nettoyage** : pas de prune, pas de `down -v`, pas de `docker rm -f`, pas de `kubectl delete namespace` ; les `docker_rm` ciblés du code fixture existant (zot/transit) sont hors chemin m6 ; arrêts gracieux seulement (lifecycle).
- Fuite possible connue : le child `kubectl port-forward` est killé par `Drop` (`chain.rs:346-364`) — vérifier post-run qu'aucun `kubectl port-forward` n'est resté en process (Windows `tasklist | findstr kubectl`, lecture) et le consigner.

## 8. Budget

- **Setup (ligne propre)** : Docker Desktop (lifecycle) + pulls zot/openbao/skopeo-ou-crane + `docker build` platform-plugin (testdata, léger) + `docker build` reference-kv `:m6` (staging workspace — compile Rust IN-Docker : poste majeur au froid) + `kind load` ×2 + waits CNI/rollouts (Calico déjà Ready → rapide) + preflight inventaire. Estimation : **30–40 %** du wall-clock du lot (builds + compile m6 au froid : kube/k8s-openapi/lazaret/axum-test jamais compilés ensemble).
- **Planifié ~75 %** : fix mécanique (3 fichiers fixtures, E-1…E-3) + preflight compile `--list` (E-4) + 1 run m6 avec preuves par hop (E-6…E-10) + inventaire avant/après + registre + Journal.
- **Buffer nommé ~25 %** : triage du premier run (famille ROUGE attendue possible — m6 n'a jamais tourné), re-run idempotence (R-09), preuve négative « cluster absent → Err » si rencontrée naturellement, capture `docker events`/events k8s pour R-08 (co-régulation), delta labels ns si fallback client-side (D-6).
- **Coupe au-delà de la capacité, par risque × effort** : (1) ne jamais couper E-1/E-2 (R-01 critique, design+audit) ni E-4 (R-04/R-05 critique) ; (2) les captures enrichies R-08 partent avant le re-run ; (3) le re-run idempotence part avant les captures ; (4) **jamais le buffer, jamais un item de zone haute pour garder des items moyens**. Recommandation au lead : le noyau (fix + compile-list + 1 run m6 + preuves) suffit à la porte ; tout le reste est buffer.
- Coût distinct **hors budget de ce lot** : la remédiation ROUGE éventuelle = P8 (nouveau plan amendé d'abord), jamais absorbée par ce lot.

## 9. Incertitudes et décisions restant à prendre

1. **GO utilisateur explicite P5** — la run m6 applique des manifests operator (additifs) et crée pod/CR sur le cluster **partagé** `kind-aiforall-local`. Décide : utilisateur (avant toute admission P5).
2. **D-6 (apply server-side vs client-side)** — décision lead à l'intégration (risque strip labels ns homonymes J2 §1.7). Recommandation : server-side + field-manager dédié.
3. **État runtime jour J** — Docker Desktop / cluster `aiforall-local` / CNI Calico / opérateur J2 actif : constats P5 (gaps « à vérifier à P5 ») ; un cluster absent n'est PAS créé par le fixture — la création éventuelle (cluster.yaml `ops/deploy/kind/`) est une opération runtime sous lifecycle + GO, jamais un effet de test.
4. **Nom du nœud `aiforall-local-control-plane`** — inférence confirmée par convention kind + cluster.yaml (1 node control-plane) ; vérification Docker day-J (lecture `docker ps`).
5. **Usage CI Linux du fixture** (D-5) — inférence non confirmée ; l'override env couvre les deux sans code branché ; une éventuelle config CI = lot dédié.
6. **Constat « chaîne = simple curl probe »** — si l'isolation netpol s'avère décorative (par ex. route egress DNS contournable), signaler pour l'axe 4/P7 (renforcement, ADR Proposed ou contrat) — NE PAS l'écrire ici.
7. **Résidus post-run m6 (pod + CR)** — delta tracé (§6.4) ; suppression ciblée éventuelle = autorisation lead ; aucune remédiation spontanée.
8. **Statiques hors axes** (m2, t12, t2, t7) — inchangés, résidus hors mission (décision utilisateur, tracée au Journal G-17).

## 9bis. Décisions du lead (2026-10-18 — GO P5 reçu)

Ancres spot-vérifiées par le lead avant commit : `chain.rs:50` (un seul `..`) et `:56` (sans
`crates/`), `m6:51-53` (UN test) et `:59` (ensure), `Cargo.toml` m6 =
`required-features = ["admit", "controller"]`, canary `k8s/05` `image: apparatus-p4-platform-plugin:t10`
+ `imagePullPolicy: Never` (:22-23), littéraux `apparatus-p4-it` = exactement
`cluster.yaml:3` + `mod.rs:1,9,221,236,1114` (aucun autre site). Tout concorde avec §1.

1. **GO utilisateur P5 ENREGISTRÉ** (2026-10-18, chat : « Je donne mon go pour P cinq »).
   Périmètre : run réel m6 sur `kind-aiforall-local` (manifests operator additifs + pod/CR test).
   **Hors GO** : `kind-aiforall-local-full` et lab 0412 (GO dédiés séparés), réactivation
   m5/t10 Kind-defer. P5 est publiée dès ce commit, sans nouvelle demande à ce jalon.
2. **D-6 : ADOPTÉ** — `apply --server-side --field-manager=apparatus-it-fixtures` dans
   `apply_operator_manifests`. L'inventaire avant/après des labels des 2 ns homonymes reste
   obligatoire (§6.4) : delta « strip de labels » = STOP + escalation, jamais masqué.
3. **Négatif E-2 explicité (durcissement lead)** : AVANT le run réel, un run m6 avec
   `APPARATUS_KIND_CLUSTER=bogus-inexistant` doit échouer **fail-loud à `ensure()`** (Err
   explicite « cluster absent », jamais un panic chain.rs) **sans créer/supprimer rien** —
   `kind get clusters` identique avant/après. Puis unset. Preuve `.tmp/m6-neg-cluster.log` +
   diff. C'est la preuve runtime de R-01, à côté de l'audit diff.
4. **Résidus post-run (pod/CR) : LAISSÉS** sur le cluster, delta tracé (§6.4) ; suppression
   ciblée seulement sur autorisation lead explicite (idempotence re-run gère le delete).
5. **m5 `--list` : HORS lot** (inférence N-1 close par le diff ; jamais payée sauf demande).
6. **Worktree P5 = true** (écrivain : 3 fichiers fixtures ; run m6 depuis le worktree avec son
   diff ; merge lead ensuite ; slot Cargo unique).
7. Ordre P5 : skill lifecycle → preflight inventaire AVANT (lecture seule) → `--list` →
   négatif bogus-cluster → run réel → inventaire APRÈS + delta → check fuite
   `kubectl port-forward` → registre du jour → rendu.
8. **GO in-worktree pool-14 (P8-min, 2026-10-18, après ROUGE run2 + arbitrage utilisateur)** :
   run2 = ensure OK (re-ciblage prouvé en réel) puis panique `chain.rs:234` — build CRI
   `apparatus-reference-kv:m6` exit 1 : `Dockerfile.http:16` copie
   `crates/apparatus-reference-kv/apparatus.toml` (chemin racine-repo) dans le contexte
   **mini-workspace** déclaré par son propre en-tête `:1` — inconstructible des deux côtés ;
   **régression `f060d47`**, trois consommateurs cassés (`chain.rs:176-228`/m6, `m5:201`,
   `schedule-reference-kv.ps1:151-152`). Correctif GO : **révert 1 ligne `:16`** →
   `COPY apparatus-reference-kv/apparatus.toml /apparatus.toml` (contrat déclaré, tous les
   consommateurs). Le miroir test-side staging est REFUSÉ (masquerait la régression, hors
   liste E-3). Le auto-repair #1 (env KUBECONFIG) reste valide. Puis re-run réel complet ;
   merge unique ensuite ; P6 audit du diff total.
9. **E-13 (P8-min, ajout au registre)** : obligation = révert `Dockerfile.http:16` ; preuve
   positive = image `apparatus-reference-kv:m6` construite + `kind load` + pin `@sha256:` +
   hops suivants verts ; classification = régression artefact repo (jamais un bug fixture ni
   prod logic). **Durcissement tracé hors lot** : `bin_ok("kubectl")` devrait sonder
   `kubectl version --client` (faux « not found » possible) — petit lot fixture futur, noté
   pour P6.
10. **E-14 (P8-min suite, tranché lead après ROUGE run3 + vérification ancres)** :
    run3 = révert :16 validé en runtime (COPY apparatus.toml OK + apt wget OK) mais échec au
    build stage-1 : le **mini-workspace embarqué** (`chain.rs:181-216`, staging test-side) est
    périmé — `crates/apparatus-reference-kv/Cargo.toml:11,24-25` hérite
    `reqwest`/`rcgen = { workspace = true, optional = true }` (feature http), **absents** du
    `[workspace.dependencies]` du staging (ne contient que serde/serde_json/toml/thiserror/
    uuid/sha2/semver/axum/tokio) → parse fail. Le staging n'a JAMAIS compilé (la dérive était
    masquée par le panic chain.rs à 0 s pré-fix). Correctif = **2 entrées verbatim** du
    manifest racine (`Cargo.toml:88` reqwest 0.12.0 json/rustls-tls ; `:142` rcgen 0.13.2
    x509-parser) ajoutées au `[workspace.dependencies]` du staging — zéro invention, zéro
    autre changement. Décision : tranchable (mécanique, miroir, test-side) — pas une
    contradiction structurelle non tranchable. Budget worker : auto-repair **#2 et dernier** ;
    toute cause nouvelle après = STOP définitif + arbitrage user.

## 10. Journal des mises à jour


- 2026-10-18 : création du plan (P4-(2), base `dad816e`, pin `41a86dba`, worktree false). Faits §1 vérifiés par lecture (fixture kind `mod.rs`/`cluster.yaml`, 17 occurrences grep, m6 intégral, chain.rs intégral, Cargo.toml, manifests k8s/ 6 fichiers, overlay J2 3 fichiers collés, glob testdata/crates) + git (HEAD, porcelain, gitlink) ; AUCUN runtime/cargo/kubectl touché. Matrice R-01…R-09 (critiques R-01/R-05, haut R-02), modes pour ≥10, registre E-1…E-12 tous GAP, design D-1…D-7, commandes §6.2, preuves par hop §6.3, inventaire §6.4, budget §8, barrières §7. Écriture via MCP context-mode (ctx_execute python fs) — aucune écriture shell.
- 2026-10-18 (lead) : ancres spot-vérifiées (§9bis). GO P5 enregistré ; D-6 adopté (server-side, field-manager dédié, inventaire labels obligatoire) ; négatif bogus-cluster explicité comme étape P5 obligatoire ; résidus post-run laissés + delta tracé ; m5 --list hors lot ; worktree P5 = true ; ordre P5 figé. Plan commité par le lead avant publication P5.
- 2026-10-18 (lead, après ROUGE run2) : §9bis.8/.9 — GO in-worktree pool-14 pour le révert P8-min de `Dockerfile.http:16` (régression f060d47, 3 consommateurs) ; re-run réel ensuite ; E-13 ajouté ; durcissement `bin_ok` noté hors lot pour P6.
- 2026-10-18 (lead, après ROUGE run3) : §9bis.10 — E-14 tranché : staging chain.rs périmé (reqwest/rcgen absents du workspace.dependencies embarqué, hérités par la crate), 2 entrées verbatim racine (reqwest 0.12.0 json/rustls-tls, rcgen 0.13.2 x509-parser). Ancres vérifiées lead (chain.rs:192-201, crate Cargo.toml:11/24-25, root Cargo.toml:88/142). Auto-repair #2 et dernier du worker.
- 2026-10-18 (lead, récupération de livraison P5) : run4 vérifié dans le log UTF-16 du worktree pool-14 : `test m6_e2e_0002_0008_chain ... ok` ; `1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 311.79s`. Aucun re-run. Code récupéré après shutdown du worker sans HANDOFF final : commit worktree `8c5b260`, intégration via `team_merge`, master **`e66b736`**, patch binaire identique. Diff total : quatre fichiers, E-13/E-14 inclus. Rustfmt ciblé et diff-check verts ; hook global os-206 reproduit, contournement transitoire au commit, sans réglage global persistant.
- Preuves et limites de livraison : `.tmp/m6-lead-recovery-20261018.md`. Pod plugin Running/Ready, image CRI pinée, CR VALID même digest/labels, canary Ready observés en lecture seule après le run. H1/H2/H3/H4 reposent sur les assertions exécutées ; captures brutes autonomes du JSON/HTTP/hops/wget absentes de la livraison (pas d'extraits inventés). Le `--list` m6 ne compile pas les quinze autres cibles : les formulations D-3/E-4 « lie les 16 sites » ne constituent pas une preuve de leur compilation.
- Cleanup récupéré par le lead : zot/OpenBao identifiés via événements Docker create exacts, arrêtés gracieusement et vérifiés exited 0 ; cinq helpers avec die/destroy retrouvés. `.tmp/docker-resources-20261018.log` régularisé **tardivement**, omissions worker non effacées ; BuildKit opaque UNKNOWN+ask. Cluster partagé et pod/CR conservés conformément §9bis.4. Binaire du run issu du target local du worktree sur C:, conformité au target E: non démontrée. **Porte P6 encore ouverte** : ne pas transformer le seul vert libtest en acceptation finale de tous les E-x.

