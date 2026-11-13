# Campagne CI Ubuntu — health, OAuth navigateur et TLS/SAN

Le workflow `k6-e2e` exécute une pile autonome sur une VM Ubuntu jetable, en
manuel ou automatiquement depuis la CI de base après les builds. Le
[contrat Proposed](../../../../docs/contracts/k6-three-scenario-ubuntu-ci.md)
décrit ses frontières. Première campagne complète mesurée :
[run43461703946](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43461703946),
34checks et corrélations parent PASS, restauration/cleanup PASS.
Voir le [bilan et ses limites](../../../../docs/missions/20261113-k6-ubuntu-e2e-results.md).
Un succès des seuls helpers Python ou du packaging ne qualifie pas l'E2E.

## Éléments livrés

- `evidence.py` : réception fermée des cinq résumés existants, 34 checks,
  codes natifs explicites, deltas OAuth et causalité/restauration SAN.
- `inputs.py`, `config/*.toml.in` : secrets neufs et configurations publiques
  fermées, sans import de configuration opérateur ; chargées par les vrais
  services dans la campagne qualifiée sur les images applicatives indiquées.
- `config/envoy.yaml` : profil navigateur TLS, upstreams mTLS vérifiés,
  ext-authz fail-closed, stripping principal et exemptions exactes OAuth/health.
- Extension opt-in `ciEnabled: true` du chart natif, namespace
  `aiforall-local-full` : alias IAM isolé et vrai workload ext-authz. Sans cette
  option, l'inventaire natif reste inchangé à dix objets. L'option ne constitue
  pas une autorisation d'appliquer ces objets au lab existant.
- `pki.py` : PKI neuve du run, clés de signature JWT et neuf identités TLS,
  dont bon/mauvais SAN sous la même CA ; aucun import de confiance système.

## Réutiliser les sorties des builds, sans relancer Cargo

La matrice `build` de `ci.yml` utilise Ubuntu24.04 et conserve les sorties
**dev/debug non instrumentées** pour IAM (migration comprise), OAuthConnect et
ext-authz. Ce sont les mêmes compilations que les jobs de build, pas celles
de la couverture. Aucun cache ou ancien binaire n'est utilisé comme fallback.
Les trois bundles sont nommés `k6-binaries-<role>-<run_id>-<attempt>` et conservés
sept jours, avec SHA256, provenance, flags et métadonnées ELF.

Le workflow **`k6-images.yml`** est indépendant et manuel. Lorsque ces trois jobs
build sont verts et leurs bundles disponibles, il peut tourner même si les IT
de la CI productrice ne sont pas terminées :

```bash
gh workflow run k6-images.yml --ref master \
  -f producer_run_id="$RUN_ID" \
  -f producer_attempt="$ATTEMPT" \
  -f target_sha="$TARGET_SHA"
```

Les trois variables désignent le run CI, son attempt exact et son SHA applicatif
complet. Aucun choix implicite de « latest ». PR/fork, autre workflow, autre
attempt, build incomplet, expiration, instrumentation LLVM ou corruption donnent
un refus, jamais une reconstruction automatique.

`artifacts.py` vérifie la provenance API et le digest de l'archive avant extraction.
`binaries.py` vérifie ensuite le manifeste fermé, chaque hash et les métadonnées
ELF. `package.py` utilise les Dockerfiles existants avec le runtime Ubuntu24
épinglé, normalise les chemins sans rebaptiser le profil réel en release, et
vérifie les dépendances des quatre exécutables via des probes `ldd -v` isolées
du réseau. Aucun service n'est démarré par cette vérification.

Le résultat est un artefact `k6-images-<run_id>-<attempt>` contenant uniquement
`images.tar.gz`, `provenance.json`, `cleanup.json` et `docker-resources.jsonl`. Il est lui aussi
conservé sept jours. Les probes sans état sont arrêtées et retirées par ID ;
aucun prune ou retrait d'image préexistante. Les images restent sur le runner
jetable et sont exportées en archive, sans push de registry ou secret opérateur.
La fermeture Python est doublée d'une étape indépendante `always()` : reçus
contemporains et IDs/labels recoupés, arrêt gracieux des seules probes possédées,
inventaire final. Un label sans reçu permet de détecter une fermeture incertaine,
jamais d'adopter la ressource. Conflit ou preuve incomplète bloque le succès ;
une annulation dure de la VM peut toujours empêcher la collecte finale.

La provenance distingue **applications T / SDK S / harness H**. H peut différer
de T pour corriger les tests sans reconstruire les applications, à condition que
la seam d'inventaire soit compatible. L'entrypoint IAM provient toujours de T.
Un vert de ce workflow prouve seulement packaging et résolution ABI, **pas E2E**.

### Première réutilisation réellement vérifiée

[Run indépendant43420550908](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43420550908),
job129secondes (11:52:50→11:54:59UTC, 2026-11-13) : trois images empaquetées
depuis les bundles du runCI43415018528/attempt1, quatre ELF résolus, trois probes
retirées et cleanup indépendant PASS. Aucun Cargo/unit/IT dans ce consumer.
Applications `1ca9a4c8`, SDK `93b890ff`, harness `8f614eb3` : réutilisation H≠T
effective. Ce temps observé n'inclut pas une campagne E2E et n'est pas un SLA.

Artefact réutilisable : **`k6-images-43420550908-1`**, ID **`14033787265`**,
expiration annoncée **2026-11-20 11:54:52UTC**. Archive `images.tar.gz` SHA256 :
`8dfd67758e2f2bafefc0c5178f8c8cab9a4725f8cfc570ed570a55afb0596e57`.
Les tags et IDs des trois images sont dans sa provenance ; la campagne E2E
vérifie ce bundle, sans reconstruire les applications.

## Exécuter la campagne avec des images sauvegardées

Depuis un checkout autorisé du dépôt, choisir un **run de packaging** réussi,
son attempt exact et le SHA applicatif complet attesté par ses images :

```bash
gh workflow run k6-e2e.yml --repo Djoe-Denne/IAMRusty --ref master \
  -f image_run_id="$IMAGE_RUN" \
  -f image_attempt="$IMAGE_ATTEMPT" \
  -f target_sha="$TARGET_SHA"
```

Le consumer vérifie workflow/run/attempt/IDs/hash/provenance avant utilisation.
Expiration ou incompatibilité = refus ; aucun fallback Cargo. Un nouveau run
de packaging peut réutiliser les bundles producteur encore valides. Si ceux-ci
ont aussi expiré, une nouvelle production nécessite son propre mandat ; pas
de reconstruction cachée. Le run ci-dessus ne lance ni UT/IT applicatifs ni
couverture LLVM, et n'utilise aucune image applicative instrumentée.

`campaign.py` crée le Kind mono-nœud possédé, les dépendances, secrets et PKI
neufs, les forwards et le profil NSS privé. Il exécute health, SAN bon/mauvais/
restauré, puis le vrai scénario OAuth navigateur, et reçoit les34checks nommés
ainsi que les deltas exacts. Une sortie k6 nulle seule ne suffit jamais.

`campaign_browser.py` construit uniquement l'image d'outillage k6/Chromium avant
les mesures. Le navigateur tourne en Docker non-root, rootfs readonly,
capdropALL, NoNewPrivs et docker-default ; profil seccomp ciblé/pinné sans
privileged ni sysctl global. Le témoin exige renderer possédé, namespaces user
ET pid distincts, seccomp2 avec filtre additionnel et aucun drapeau de bypass.
Les titres Chromium Linux réécrits sont décodés sans exécution de commande.

Réseauhost uniquement dans la VM GitHub jetable pour conserver les origines
IAM127.0.0.1:18443/IdPlocalhost:18445 et l'isolation du cookie host-only. Seule
la case privée RW est montée sous `/run/case` (chemin court pour les sockets
Chromium) avec trois fichiers d'entrée publics readonly ; aucun socket Docker,
kubeconfig ou clé privée. Les scripts métiers et leurs assertions ne sont pas
remplacés par une sonde. Le workflow `k6-browser-container.yml` reste un diagnostic
manuel distinct, explicitement NOT_E2E_SANDBOX_TRIAL, jamais une qualification OAuth.

Le parent tente la restauration SAN avant teardown, ferme les enfants et le
conteneur navigateur, puis le cluster/volume reçus. Chaque retrait est lié aux
IDs/reçus contemporains, pas aux seuls labels. Une étape indépendante always
revérifie le cleanup. Seuls les fichiers publics explicitement allowlistés sont
uploadés dans `k6-e2e-<run_id>-<attempt>` pour sept jours : aucun raw log, profil,
token/cookie, URL OAuth query, kubeconfig ou clé. Aucun runtime local n'est
démarré ou nettoyé par cette campagne hébergée.

## Déclenchement automatique et rapports

Sur `push/master`, `ci.yml` appelle ce même workflow après `build`, en parallèle
de `coverage-unit` et `coverage-integration`. Le job E2E ne dépend pas des
couvertures, et Sonar ne dépend pas de l'E2E. Les PR/forks n'ont pas accès à
cette pile hébergée :les bundles restent limités aux builds push/master.

Le mode automatique acquiert les trois bundles non instrumentés du **même
run/attempt/SHA**, avec les mêmes contrôles API/digest/ELF que le mode manuel.
Il empaquette les images sans Cargo, ferme les probes et vérifie leur cleanup
avant admission locale. Le SHA applicatif est donc celui de la CI courante,
pas celui du run historique43420550908. Le mode manuel saved-images demeure
disponible séparément ; aucun fallback «latest», cache ou recompilation cachée.

Les résultats publics expurgés alimentent :
- `k6-e2e-report-<run>-<attempt>` :HTML Allure, page d'accueil, status.json/badge.json
  et résultats Allure JSON, sans attachments/logs privés.
- Le résumé du job GitHub :statut et34contrôles prouvés.
- [GitHub Pages](https://djoe-denne.github.io/IAMRusty/) et son badge README :
  dernier run automatique publié, source/harness/SHA et lien au run affichés.

Allure contient les34contrôles fonctionnels et un cas distinct pour le contrat
parent (deltas, sandbox, restauration, cleanup). Les checks absents sont **non
prouvés/broken**, pas des passes inventées. Même avec34checks verts, un contrat
parent rouge rend le badge rouge. Les timings et l'historique de tests ne sont
pas inventés ;aucun historique inter-runs persistant n'est configuré ici.

Le générateur Allure2.46.2 est acquis en répertoire temporaire, par URL/version,
taille et SHA256 épinglés dans allure.lock.json, puis extraction sans liens ni
traversée. Aucun outil global/local Java installé par le parent ;le CLI Java
s'exécute sur le runner Ubuntu. Un échec de génération empêche la publication
d'un rapport HTML incomplet. Les preuves brutes autorisées de la campagne sont
toujours conservées dans l'artefact existant, séparé du site.

La publication Pages est un job distinct aux droits pages/id-token, sans
secret transmis à la campagne. Elle fonctionne aussi pour un E2E échoué afin
de montrer son état rouge, mais jamais pour une PR/fork. Un ancien SHA ou
attempt ne remplace pas le rapport du master courant ;les jobs sont sérialisés.
Les droits de test restent contents/actions lecture. Les preuves de packaging
auto (provenance/cleanup/reçus, pas les images) sont conservées séparément dans
`k6-packaging-evidence-<run>-<attempt>`.

### Sonar et E2E :deux métriques distinctes

Les badges Sonar natifs exposent le quality gate, la couverture, les ratings et
les duplications du vrai projet `Djoe-Denne_IAMRusty`. Pas de token dans le README.
Le [catalogue API SonarCloud](https://sonarcloud.io/api/webservices/list) consulté
le13novembre2026 ne propose pas de création de métrique custom. L'import
[Generic execution](https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/test-coverage/generic-test-data.md)
est un format de résultats lié aux **fichiers tests analysés**, pas une métrique
arbitraire34/34 ;le périmètre Rust actuel n'est pas élargi pour le simuler.
Les E2E sont donc exposés dans Allure/Pages/JSON, sans les mélanger au LCOV UT/IT.

## Validation hors cluster

Outils existants explicites : `AIFORALL_HELM` et `AIFORALL_K6`, sinon recherche
PATH (replis Windows existants conservés pour les tests locaux). Les outils ne
sont pas téléchargés par les tests. Les scripts CI devront vérifier les pins
avant de définir ces variables ; leur présence seule ne prouve pas la version.

```text
python -B -m unittest discover -s ops/tests/k6/tests -p "test_ci_*.py" -v
```

Les tests de rendu utilisent réellement Helm ; ceux du projecteur exécutent
réellement k6 sans requête réseau ni lancement Chromium. Un outil absent donne
un skip explicite, jamais une preuve de qualification.

## Portée de la qualification

Le résultat est lié aux **images applicatives T**, au SDK S et au harness H
indiqués dans provenance.json, pas implicitement au HEAD applicatif courant.
La campagne finale utilise T=`1ca9a4c8`, S=`93b890ff`, H=`bc143cc8` ; les corrections
coverage90 postérieures à T ne sont pas qualifiées par ces images. Pas de mesure
Sonar/90%, de campagne maintenance/mTLS complète, de charge ou de qualification
de tout le lab. Aucun workflow historique suspendu n'est réactivé et aucun
statut ADR/contrat Proposed n'est promu par un run vert.
