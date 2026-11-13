# E2E automatique et rapports publics —13novembre2026

## Résultat mesuré

[CI43473245852](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43473245852),
événement **push/master**, attempt1, source
`741d04359a332de78802ef59cd8b2d1480297787` :job E2E **PASS**, génération Allure
**PASS**, job publication GitHub Pages **PASS**. La **CI globale est rouge** :
le job couverture d'intégration Manifesto échoue indépendamment de ces verdicts.
Vérification indépendante finale **PASS borné à E2E/Allure/Pages** par
infra-verifier, session `ses_e2b39f576ffe6sWFVATRaMqC3i`, via API, artefacts,
sources exactes et site live. Ne pas confondre ce succès avec celui de toute la CI.

## Accès

- [Accueil et métriques E2E](https://djoe-denne.github.io/IAMRusty/).
- [Rapport Allure](https://djoe-denne.github.io/IAMRusty/allure/).
- [Statut JSON](https://djoe-denne.github.io/IAMRusty/status.json).
- [Badge JSON](https://djoe-denne.github.io/IAMRusty/badge.json).
- [SonarQube Cloud](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty).

Les cinq premiers endpoints pertinents (accueil/status/badge/Allure index/widget
summary) sont mesurés **HTTP200**. status.json :passed,34/34,unproven0, run et
SHA ci-dessus. Allure widget :**35passed,0failed,0broken,0skipped** —34checks
fonctionnels et un contrat parent distinct, pas35checks fonctionnels. Les badges
README sont dynamiques ;les six métriques Sonar existent et leurs images SVG
ont été vérifiées, aucun token privé ou valeur de couverture statique.

## Parallélisme et provenance

Les huit jobs build terminent avec succès. Le dernier build finit à17:33:55UTC ;
E2E et les couvertures UT/IT commencent à17:33:57. E2E termine à17:40:27,
publication Pages17:40:31→17:40:47, avant la dernière couverture17:54.
Le reporting n'attend donc ni les couvertures ni Sonar.

Applications T, harness H et packaging proviennent tous de **741d0435**, SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. Les images ont été empaquetées
depuis les bundles non instrumentés du même run/attempt, pas depuis l'ancien
artefact T1ca9a4c8. Aucun second Cargo/build applicatif dans le consumer E2E.
Le fix du collecteur autorise seulement les deux alias Clippy inertiels du
dépôt ;les flags LLVM/wrappers/profiles/env/target restent refusés.

Le commit des16unités runtime-unit-rules80104de4 est ancêtre de cette source.
La couverture unitaire Manifesto est verte et utilise manifesto-infra --lib
--bins ;ces unités ne sont pas confondues avec les22binaries --test coverage90.
Aucun nouveau pourcentage Sonar ni succès exhaustif d'intégration n'en est déduit.

## Campagne fonctionnelle et sécurité

Runfonctionnel `bb1a452516bc45019881ad6337ee0c45`, cinq phases complètes et
rapport PASS :health2checks, SAN bon/mauvais/restauré6checks, OAuth navigateur26.
Deltas acceptés IAM+7/ext-authz+4/provider+3/PKCE+3/users+1/outbox+0/SAN+1,
restauration PASS. Quatre témoins renderer UID1001, userns/pidns isolés,
seccomp2/filtres2contre1, NoNewPrivs1, absence de drapeaux interdits.
Cleanup parent et navigateur PASS, owned_running0/errors[].

Le recoupement indépendant confirme aussi les trois probes de packaging
possédées/retirées (cleanup PASS/owned_running0), puis les reçus moteur de retrait
nœud/volume et le cycle complet du navigateur avant teardown. Les14fichiers
de l'artefact campagne sont bit-identiques aux copies locales (0mismatch).
Le bundle public contient111entrées, dont35résultats de tests Allure et aucune
pièce jointe ;scan borné JSON/properties/HTML JWT/bearer/ghp_/sentinelles sans
détection. Le status.json servi est identique à celui du bundle de ce run.

Politiques du lanceur conservées :non-root, rootfs readonly, capdropALL,
docker-default effectif et profil seccomp ciblé épinglé, pas de mode privilégié
ou de bypass TLS. Réseauhost limité à la VM jetable pour conserver les origines.
Aucun runtime Docker/Kind/WSL local démarré, adopté ou arrêté par ce lot.

Le site ne contient que des projections fermées. Un check absent est non
prouvé/broken, pas PASS ;un contrat parent rouge rend le badge rouge même si
34checks sont verts. Aucun log brut, token/cookie, URL OAuth query, profil NSS,
clé, kubeconfig, HAR, screenshot ou pièce jointe. Les durées de tests et un
historique inter-runs ne sont pas fabriqués. Le job Pages n'a pas exécuté la
campagne et n'a pas transmis son droit pages-write/id-token au navigateur.

## Validation et limites

130helpers CI locaux PASS/0skip ;Actionlint1.7.12 PASS pour les deux workflows.
Archive Allure2.46.2 vérifiée par SHA256 avant extraction ;génération Java réelle
qualifiée par le job E2E et le HTML public, pas revendiquée sur le poste sans Java.
Le premier push7e5ec5e1 avait échoué avant collecte, empêchant E2E et Pages ;
aucun ancien rapport vert n'a été utilisé comme preuve du nouveau run.

Le job [IT Manifesto132053039776](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43473245852/job/132053039776)
échoue sur `t6_post_get_components_contract_bytes_frozen`, dans
services/Manifesto/tests/apparatus_p1_t6_http.rs:120, avec une autre panic Tokio
runtime/task/core.rs:422. Résultat du binary concerné :3passed/1failed. Cause
non attribuée et aucun correctif Rust glissé dans ce lot. Les autres jobs
couverture sont verts, ainsi que merge et scan Sonar ;cela ne transforme pas
cette intégration rouge en réussite globale ou en campagne90% complète.

Le badge E2E représente le **dernier rapport publié**, SHA et run affichés sur
le site, pas une promesse de validation instantanée du HEAD. Un ancien SHA ou
attempt ne peut écraser une publication courante. Pas d'historique Allure
persistant. Sonar reste une mesure distincte du LCOV/qualité :le catalogue API
public ne fournit pas de création de custom measure, et son import générique
exige des fichiers tests ;aucune simulation de métrique34/34 dans des tests Rust.
Contrat d'architecture toujours Proposed.
