# Contrat — ops Kind : runtime autonome, validations explicites

- Statut : **Proposed**, 2026-10-19 ; aucune commande exécutée par D.
- Auteur : pool-4 (architecte) ; intégration canonique par le lead (écriture worktree refusée par le harness, registre : ledger 2026-10-19).
- Jalon : plateforme locale/local-full, hors Apparatus P0–P6.
- Canon : ADR 0606, 0603, 0308, 0200 sous docs/adr/.
- Ordre : docs/missions/20261018-ops-kind-runtime-validation-separation.md §§1,4,6–8.
- Preuves : audits A/B/C consolidés dans .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md, base 80918ca ; ce ledger n’est jamais une entrée d’exécution.
- Annexe : [manifeste et propriétaires](ops-kind-runtime-validation-manifest.md).

## Décisions D1–D6

**D1 — Un nominal : local-full.** Artefacts Linux → configuration/certificats locaux → bail opérateur neuf → bootstrap full → images épinglées → déploiement → validation séparée. Cluster aiforall-local-full, contexte kind-aiforall-local-full, 1 control-plane + 2 workers ; namespaces aiforall-local-full, aiforall-local-apparatus, aiforall-local-plugins. Aucun repli implicite Compose, monolithe, stub ou legacy.

E2 M1/M2/M3 = jalons stubs/lint ; E3 J2 = operator hybride ; E4 J3/gold = démo monolithe avec dépendances hôte ; E5 mesh = autre déploiement local ; E6 schedule-reference-kv = dette démonstrateur. Les conserver comme non nominaux, avec limites/cible explicites ; aucune suppression de capacité ni équivalence full annoncée. Le nominal choisi n’est pas encore prouvé live.

**D2 — OpenFGA reste dans ops/openfga/, copie unique.** model.fga + model.json = SHARED-CONTRACT, pas MOVE-TEST-INTERNAL : génération en paire, consommation runtime et tests. Le framework rustycog reçoit le JSON et n’a aucun chemin ops. Aucun changement gitlink. I-références corrige le fallback justfile:156 pour monter le sous-répertoire ops/openfga du PWD, pas openfga à la racine.

**D3 — Bootstrap nécessaire ≠ validation.** Certificats, initialisation FGA, migrations et Transit restent runtime. prepare-local-full-certs.sh est runtime (écart à la pré-proposition C). openbao-gold-seed.sh reste en place : son usage Compose impose de préserver ce contrat malgré son appel par prove-gold. Pas de fusion avec seed-openbao sans preuve d’équivalence. Le déploiement n’importe aucun driver. La sonde de l’overlay kind sort du lancement mais reste disponible comme validation explicite.

**D4 — Historique ciblé, sans nettoyage exécuté.** Liste proposée : checkpoint D ; outil layout-rewrite avec ses fixtures ; rename-crates ; trois reliquats 0604 de l’annexe. Accord utilisateur avant archivage, particulièrement les reliquats de capacité. Pas d’archivage en bloc des démos/anciens déploiements exécutables. Sept .pyc : REMOVE-GENERATED de routine et ajout d’ignore, non exécuté par D. start-all.* reste DEFER-UNKNOWN, pas « inutilisé ».

**D5 — Deux interfaces, pas de framework de scripts nouveau.** Garder ops/deploy/ pour runtime/autorité de cible, ops/tests/ pour validations/tests d’outils. Pas de bibliothèque shared abstraite : l’import de local_full_target.py par un validateur est un contrat de sécurité légitime. Les fixtures Rust restent dans leurs suites/harness, pas dans ops.

**D6 — L’opérateur Windows responsable de la session produit le bail initial**, pas un writer .cursor ou une ancienne mission. Publier la procédure locale ci-dessous, supprimant le prérequis tacite « parent sait produire le fichier ». Aucun writer applicatif/framework ajouté. Le bootstrap produit ensuite les attestations observées ; ne jamais préremplir IDs, mapping, UID ou hash depuis des noms.

## Portée, limites et contradictions

Impact cross-module/contrat opérationnel : ops, justfile, CI statique, documentation. Aucun port/API Rust, schéma DB, protocole métier ou dépendance structurante nouveau. **Aucune ADR nouvelle** : rangement réversible des outils et précision du contrat 0606, qui reste Proposed/Partial ; ne pas consommer 0607 ni réécrire les ADR historiques.

Sources full incluses : PG15/six DB/PVC, OpenFGA/store, Redis AOF, Zot, OpenBao auth k8s/Transit, quatre queues LocalStack, quatre standalones, Lazaret, sentinel-sync, ext-authz/Envoy, operator/CRD/RBAC, NP/PSS, quotas/PDB. Simulations : SQS/LocalStack, SMTP/MailHog, catalogue stub, OpenBao DEV, Zot gold. Exclus : OAuth vendors désactivés, edge TLS18443, GKE/Flux/HSM/HA, seed artefacts/signatures et invoke plugin réel sans préparation. YAML/rendu/Ready ne prouvent pas les parcours.

Contradictions à exposer :
- justfile:199 dit prove-gold nominal : historique 0605, pas nominal full.
- ADR0606:26 mentionne compilation Docker ; décision utilisateur 2026-10-07 prioritaire : dev/tests Windows natif, Docker seulement livraison Linux et dépendances testcontainers.
- 111→112 objets est expliqué par le Secret CA public ajouté (checkpoint:150,177), pas un oracle de compte fixe.
- Le kubeconfig inexistant du renderer est intentionnel : ne pas le créer.
- validate-deploy-static.ps1:15–26 liste **11** chemins, pas 15 (raccourci audit C).
- generate-test-keys sert des **tests unitaires** sous cfg(test) dans services/IAMRusty/infra/src/signing/probe.rs:256,583–588,657,716, pas une IT testcontainers. Sa mutation .env n’est pas un prérequis runtime.

Les neuf points A restent hors modification applicative : wiring namespace (quatre sources Lazaret), désérialisation/locator plugin_hop.namespace, admission/signature Zot/Cosign, fixture OAuth/PKCE, HA Lazaret process-local, parité OpenBao prod, build Linux, producteur initial leasev2, contraintes hôte/réseau/Calico. A7–A9 sont des préconditions opérateur sans changement applicatif. --namespace-wiring-confirmed n’atteste pas le code embarqué. Scénario empêché ⇒ BLOCKED/NOT RUN, pas extension silencieuse ni assertion supprimée.

## Parcours opérateur nominal à publier par I-runtime

Procédure cible, **non exécutée**. GO runtime spécifique après Q, intégration et IT finales. $session, $lease, $images, $certs, $environment sont des sorties neuves privées, hors Git ; pas des sorties d’ancienne campagne. Jamais logguer clés, tokens ou kubeconfig.

### Artefacts et configuration

Préflight : PowerShell Windows, Python+PyYAML, Bash/OpenSSL, Docker/BuildKit, Kind, kubectl/Kustomize. Outil absent ⇒ échec. Ne pas démarrer le runtime pour une analyse statique. Après GO et bail, construire Linux avec les Dockerfiles existants, sans compose up :

    $env:DOCKER_DEFAULT_PLATFORM = 'linux/amd64'
    docker compose -p aiforall build build-artifacts
    docker compose -p aiforall --profile mesh build iam-service hive-service telegraph-service manifesto-service lazaret-service ext-authz
    docker tag aiforall-lazaret-service:latest aiforall-lazaret-service:local-full
    docker build --platform linux/amd64 -t aiforall-sentinel-sync:local-full -f ops/deploy/build/Dockerfile.worker .
    docker build --platform linux/amd64 --provenance=false --sbom=false -t aiforall-apparatus-controller:j2 -f workers/apparatus-operator/Dockerfile.controller .
    docker build --platform linux/amd64 --provenance=false -t aiforall-zot:gold -f ops/deploy/p4/Dockerfile.zot ops/deploy/p4

La recette contrôle chaque code de sortie natif avant poursuite. Enregistrer les IDs créés selon le registre ; ne pas s’attribuer les préexistants. Les neuf sources sont celles de local-full-images.py:9–11. Aucun target Windows partagé avec Linux. Préparer certificats avec LOCAL_FULL_CERT_DIR=<chemin-neuf> bash ops/scripts/prepare-local-full-certs.sh (syntaxe Bash ; transmettre le chemin Windows explicitement). Deux CA mesh/workload ; projection publique seulement aux validateurs.

Render préalable : python -B ops/deploy/deploy-local-full.py render --output-dir <répertoire-neuf>. Offline ; n’autorise pas le lancement.

### Bail public et premier maillon

L’opérateur écrit un JSON UTF-8 neuf : version=2, cluster="aiforall-local-full", context="kind-aiforall-local-full", task_id=<UUID hex minuscule 32 caractères>, anchor_state="pending_creation", nodeContainerIds=[], kubeconfig_path=<approved_root>/cluster-anchor-<task_id>/kubeconfig.yaml.

approved_root suit local_full_target.approved_root, pas un chemin personnel copié. Sur Windows : `Path.home()/AppData/Local/Temp/opencode` (local_full_target.py:128–129) — le répertoire approuvé de la session, pas un .tmp de mission. Guide : exemple PowerShell de sérialisation avec création exclusive et ACL privées, résolution explicite de cette racine. Le répertoire privé du bail est distinct du répertoire d’ancrage ; ce dernier et son kubeconfig doivent être absents et sont créés exclusivement par bootstrap. Aucun creation, UID, ID CP, hash ou endpoint préfabriqué. Cette étape locale ne contacte ni API ni Docker. Q vérifie bail minimal accepté pour création et tout bail emprunté/legacy/lié/ambigu refusé. Pas d’adoption d’un cluster déjà présent.

    powershell.exe -NoProfile -File ops/deploy/bootstrap-local-full.ps1 -LeasePath $lease -AfterFinalIT
    python -B ops/deploy/local-full-images.py --lease $lease --output $images
    python -B ops/deploy/deploy-local-full.py deploy --lease $lease --image-manifest $images --cert-directory $certs --after-final-it
    python -B ops/deploy/deploy-local-full.py inventory --lease $lease

Ordre impératif : l’épinglage exige un bail déjà ancré. Le bootstrap rappelé par deploy vérifie sans recréer. Conserver image/digest Kind exact, Calico v3.29.7 téléchargé explicitement, port18080 libre ; endpoint/mapping avant GET, UID avant mutation, absence d’override ambiant. Aucun export/adopt/retarget legacy. Foundation → dependencies → bootstrap → applications ; migrations avant scale-out, waits/Jobs utiles, refus Jobs actifs, verrous/rollback borné. Aucun driver/suite Rust requis par deploy.

L’opérateur produit ensuite le descripteur validation depuis son inventaire ; le runtime n’en dépend pas. Aucun import de tests/, .tmp, .cursor, ledger, pool-* ou sorties anciennes. Le kubeconfig privé neuf imposé par le protocole n’est pas un import d’ancienne campagne.

## Interface validation Proposed (pas un CLI déjà livré)

Conserver ops/tests/local-full-acceptance.py comme entrée full. Ajouter --environment <JSON> : version:1, profile:"local-full", context/cluster, lease_path absolu, namespace, apparatus_namespace, plugins_namespace, base_url:"http://127.0.0.1:18080", public_ca_path si nécessaire. Probes/comptes/credentials/cas sont des entrées séparées explicites. Refuser champs inconnus, overrides ambigus, URLs/cibles étrangères ou contradictoires. Ce fichier ne prouve pas la propriété : autorité unique = bail vérifié par local_full_target.

- **Smoke** : python -B ops/tests/local-full-acceptance.py smoke --environment $environment. Readiness/disponibilité des composants requis ; aucune création Pod, seed, migration, set-env, rollout ou modification NP. Ready ne prouve pas métier/AuthZ.
- **Curl métier** : ... route-matrix --environment $environment --cases ops/tests/mesh-public-route-cases.json. Positifs/négatifs discriminants via l’accès contractualisé ; adapter les cas legacy aux namespaces full. Comptes/tokens/probes déjà provisionnés explicites, aucun bootstrap caché.
- **E2E** : ... e2e --environment $environment --suite mesh-authn|apparatus --inputs <JSON-privé>. État métier déclaré, pas modification du déploiement. Ressources de test éventuelles via action séparée prepare-probes, bail/UID enregistrés et cleanup-probes ; jamais dans smoke/curl. Aucun SQL interne/FGA bootstrap caché pour rendre un négatif vert. Setup indispensable non exposé ⇒ setup isolé explicite/réversible ou BLOCKED jusqu’au lot approuvé.

Conserver network-policy, snapshot, rollout-check, recovery-check, restore-check et leur ancien CLI --lease/--namespace comme compatibilité full-only. Préserver assertions/UID/verrous. Les observations restore/recovery ne déclenchent pas les opérations runtime correspondantes.

Code0 seulement si toutes les obligations sont exécutées/satisfaites ; outil/lease/fixture absent ou skip obligatoire ⇒ non-zéro avec raison non secrète. Rapport hash Git, cible/mode, counts attendus/exécutés/échoués/bloqués, LIVE/SIMULÉ/NOT RUN, captures non secrètes ; pas de compte fixe artificiel d’objets. Cleanup uniquement UID/IDs attestés, jamais noms.

### IT natives Windows : entrée distincte, sans Kind

Ajouter ops/tests/it-native.ps1 -Suite <iam|hive|manifesto|telegraph|lazaret> et just it-native suite, vers cargo test -p <iam-service|hive-service|manifesto-service|telegraph-service|lazaret-service> --tests, un package à la fois (Q borne les --test finales pertinentes). Gates statiques/tests de scripts = commandes distinctes. Aucun --workspace implicite, Kind, service applicatif Docker ou seed créé par le lanceur. Testcontainers prépare dépendances/fixtures ; Docker à la demande selon lifecycle : IT ne signifie pas « sans conteneurs ». Target E:, unique slot Cargo. Fixtures dans services/*/tests, tests de crates et rustycog-testing au pin existant.

## Migration, compatibilité et ownership exclusif

Arbre minimal : garder ops/deploy, ops/openfga, helpers runtime ops/scripts ; validations/tests dans ops/tests ; ajouter seulement ops/tests/manifests et éventuellement legacy pour le driver gold. Pas de factorisation globale.

**I-runtime** : entrypoints/helpers/manifests runtime, common.ps1, local_full_target.py, certs/seeds, README runtime (annexe). Retire le lien de sonde de kind/kustomization.yaml, pas le payload confié à I-validation. Documente build/bail/commandes. Aucun writer applicatif, framework ou code de production Rust.

**I-validation** : drivers/tests et déplacements source ET destination. Sonde → ops/tests/manifests/invoke-probe-job.yaml ; deploy/test_local_full_target.py → ops/tests/test_local_full_target.py ; prove-gold-path.ps1 → ops/tests/legacy/prove-gold-path.ps1 (legacy et mutations déclarées préservées). generate-test-keys.sh → services/IAMRusty/infra/tests/fixtures/generate-probe-keys.sh : sortie privée explicite, pas append .env ; injection IAMRUSTY_TEST_RSA8192_PEM_B64 et secret CI préservés ; seulement commentaires/messages sous cfg(test) de probe.rs pour le chemin, pas refonte crypto/tests. Cette migration test-only demande l’accord humain ; sinon garder en place avec étiquette test-only, jamais runtime.

**I-références/historique** : justfile, .github/workflows/ci.yml, docs/skills hors README runtime, archives autorisées, .gitignore, sept .pyc. Lui seul corrige justfile:156. Ne touche aucun script/overlay appartenant aux deux autres, même pour un lien : transmet le remplacement à son owner. Les sources déplacées par I-validation ne sont pas déplacées une seconde fois par I-références.

Ordre : Q/arbitrages → writers isolés → intégration contenu/chemins → références externes → gates statiques/units ciblées → IT Windows finales → GO runtime → E2E. Source/référence migrent dans un même lot intégré ; pas de checker vert obtenu en retirant une assertion.

Conserver deploy-local-full (défaut render offline/actions/arguments), regenerate-openfga-model-json, deploy-m1/m2/m3/j2/j3/mesh, debt-schedule-reference-kv comme compatibilité legacy explicite sans redirection full. prove-gold garde la démo historique, nouveau chemin si déplacement, avertissement non nominal. Ajouter recettes minces kind-full-artifacts/smoke/curl/e2e et it-native, arguments explicites, jamais startup caché.

CI garde bash ops/tests/infra-static.sh (ci.yml:131–140), sans runtime/Cargo dans cette entrée. Préserver fermeture de tous les rendus et liste attendue pour éviter réduction silencieuse par déplacement. Ubuntu CI ne prouve pas Bash Windows. validate-deploy-static.ps1 garde son chemin ; I-validation actualise les chemins et ajoute full/validations sans perdre la couverture des 11 chemins existants :

- workers/apparatus-operator/Dockerfile.controller
- workers/ext-authz/Dockerfile
- runtime/monolith/Dockerfile
- ops/scripts/generate-platform-mesh-certs.sh
- ops/deploy/kind/cluster.yaml
- ops/deploy/kind/cluster-local-full.yaml
- ops/deploy/apps/overlays/kind/kustomization.yaml
- ops/deploy/apps/overlays/kind-mesh/kustomization.yaml
- ops/deploy/apps/overlays/kind-demo-monolith/kustomization.yaml
- crates/apparatus-contracts/Cargo.toml
- crates/apparatus-reference-kv/Dockerfile.http

Marqueur racine Cargo.toml + ops/deploy/kind/cluster.yaml inchangé (common.ps1:6–7). Aucun chemin historique supprimé tout en gardant les consommateurs comme s’il était nominal.

## Gates et arbitrages

Q : tests hermétiques fake-command/zéro runtime pour séparation deploy/driver, imports historiques interdits, ordre bootstrap/ancrage/images, mauvais port avant API, UID stale avant mutation, bail minimal/reuse étrangère, outils absents/erreurs natives, absence de secrets loggés, sorties/cleanup explicites, compatibilité commandes/chemins. Parsing PS/Python/Bash, rendus offline conservés, modèle FGA déterministe et consommateurs inchangés ; import dynamique common.ps1, tests déplacés et générateurs kind-mesh/OpenFGA couverts. Pas « tout cargo ».

Implémentation validée seulement avec nominal lançable sans harness/driver, smoke/curl/E2E distincts et comptés, IT Windows indépendantes de Kind, inventaire sans trous, aucune clé privée générée suivie. Live NOT RUN jusqu’au GO et preuves du hash intégré ; registre/cleanup lifecycle requis.

Accord humain avant I : contrat et QA ; liste ARCHIVE-HISTORY exacte de l’annexe, retrait routine pyc, migration helper test-only/compatibilité. Conservation en place possible sans modifier le nominal. start-all et dépendances applicatives bloquées restent nommées ; aucune acceptation ADR, clôture HA/sécurité ou autorisation distant/production.

--- Fin du contrat ---
