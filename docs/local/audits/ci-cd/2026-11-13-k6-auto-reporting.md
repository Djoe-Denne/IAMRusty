# CI-CD — E2E automatique, Allure, Pages et badges Sonar

- Mode :implémentation demandée,13novembre2026.
- Base :186a63c0 ;aucune modification des worktrees/QA des missions runtime-unit.
- Périmètre :workflows CI/E2E, admissions de provenance, projection publique,
  badges README. Ni correction métier Rust, ni seuil/exclusion Sonar changé.

## Livrables

`ci.yml` appelle `k6-e2e.yml` après build, en parallèle des couvertures UT/IT.
La voie automatique n'accepte que son propre run/attempt/SHA push/master, puis
les admissions API existantes (workflow CI, repo, builds, artefacts immuables,
digest et ELF). Le mode manuel reste strictement séparé. Packaging des binaires
déjà compilés, sans Cargo ;cleanup des probes avant admission des images et
always indépendant. Artefact de preuves packaging dédié.

128helpers CI PASS/0skip. Actionlint1.7.12 sur les deux workflows modifiés PASS
(validateur temporaire officiel par digest ;ShellCheck/Pyflakes non exécutés).
Admission CI refuse PR/fork/branche étrangère, ancien run/attempt/SHA, image
corrompue/provenance incompatible. Les jobs de publication sont séparés des
tests et ne leur transmettent aucun secret ni droit pages-write.

`reporting.py` produit un compteur34checks et35résultats Allure (34checks +contrat
parent distinct), un accueil HTML et une métrique compatible Shields. Pas de
timings inventés. Un contrat parent rouge conserve le badge rouge même avec
34checks PASS ;source/payload absent =incomplete/broken. Projection sur le vrai
run43461703946 mesurée :34/34 et contrat PASS. Aucun raw message/attachment ou
fichier privé copié ;sentinelles négatives couvertes par tests.

Allure2.46.2release10novembre2026, archive30601528octets, SHA256
83021e2c02e9a3487df249ef7bbf51c1b9f23b722bd288c6d1785609303e0df8 :
téléchargement/extraction réels vérifiés en Temp/opencode, sans installation
globale. Java n'est pas disponible sur ce poste ;HTML complet à vérifier dans
la CI Ubuntu, aucun PASS de rendu HTML local revendiqué.

## GitHub Pages et Sonar

GO utilisateur explicite «active gh page». API création réussie, build_type=workflow,
public=true, URL https://djoe-denne.github.io/IAMRusty/ . Aucun site préexistant
remplacé (GET initial404). Contenu encore à déployer lors de la préparation de
ce bilan. Droits pages-write/id-token limités au job de publication trusted,
garde contre ancien master/attempt et sérialisation. Échec E2E publiable en
rouge ;échec de génération HTML non publié comme un rapport complet.

Six badges natifs Sonar testés HTTP200/SVG et métriques API présentes pour
Djoe-Denne_IAMRusty. Ils exposent l'état réel, y compris le quality gate ERROR
observé au préflight ;aucune modification de politique pour masquer cet état.
Le catalogue API actuel ne contient que des actions GET api/metrics, pas de
custom_measures/création. Le format generic execution supporté par Sonar exige
des fichiers tests du périmètre d'analyse ;il n'est pas détourné pour rattacher
arbitrairement les checks JS à des fichiers Rust. Allure/Pages/JSON retenus.

## Sources vérifiées

- [Sonar badges](https://docs.sonarsource.com/sonarqube-cloud/managing-your-projects/managing-your-project-as-developer.md).
- [Sonar generic test data](https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/test-coverage/generic-test-data.md).
- [Catalogue API SonarCloud](https://sonarcloud.io/api/webservices/list), consulté le13novembre2026.
- [Support Sonar — custom measures non supportées](https://community.sonarsource.com/t/cannot-add-custom-measure-using-sonar-cloud-io/1499),
  historique seulement, recoupé avec le catalogue actuel.
- [Allure results](https://allurereport.org/docs/how-it-works-test-result-file/),
  [release2.46.2](https://github.com/allure-framework/allure2/releases/tag/2.46.2).
- [GitHub Pages — custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

## Limites de validation avant publication du code

Le nouveau déclenchement automatique, la génération Java et le déploiement Pages
ne sont pas encore mesurés sur un run du commit publié. Aucun historique Allure
persistant entre runs. Le badge correspond au dernier rapport publié (SHA et
run affichés), pas à une promesse que le checkout courant est déjà testé.
Aucun runtime Docker/Kind/WSL local touché ;ressources des autres lots préservées.

## Revue indépendante avant publication

infra-verifier, session `ses_e2b39f576ffe6sWFVATRaMqC3i`, a vérifié en RO le diff
gelé :graph parallèle, refus PR/fork, liaison run/attempt/SHA et cleanup avant
admission, permissions de publication isolées, reporter fail-closed/absence raw,
garde Pages commit/attempt et archive Allure vérifiée. Aucun finding bloquant.
Le comportement GITHUB_JOB du workflow appelé est recoupé avec le runner GitHub
et les docs (job interne e2e, pas nom affiché du caller). Pin Allure live recoupé.
L'API Pages confirme build_typeworkflow/public, statut encore null. Cette revue
reste statique ;les128tests/actionlint sont mesurés par le parent, non rejoués
par le vérificateur. Aucun succès réel de génération/déploiement encore déduit.

La première tentative de publication a été différée avant staging à cause du
bail Cargo d'un autre lot. Après libération explicite et revue, préflight libre,
index vide ;HEAD80104de4/origin/master confirmés (lot runtime-unit publié par son
propriétaire). Ces changements étrangers restent hors du commit CI/reporting.

## Premier déclenchement automatique :refus du collecteur avant E2E

Publication7e5ec5e1d68d03d8648e094f5e8152b3d496f11b vérifiée origin/master,
13chemins seulement, hook fmt PASS. [Run43470457080](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43470457080)
créé automatiquement sur **push**, sans dispatch manuel. Infrastructure/static
et ses helpers passent ;les compilations des trois producteurs terminent en
dev avec succès mais la collecte refuse **CARGO_CONFIG_OVERRIDE** dans IAM,
OAuthConnect et ext-authz. Par dépendance, couvertures/E2E/Pages sont skipped.
Sonar job success n'est pas une mesure de couverture de ces tests sautés.
Aucun HTML Allure/déploiement Pages qualifié par ce run.

Cause locale établie :le garde TOML de binaries.flags n'acceptait pas la section
alias des nouveaux raccourcis clippy-ci-production/clippy-ci-tests. Ces alias
n'ont aucun effet sur les appels explicites aux commandes build/metadata ;la
[référence Cargo](https://doc.rust-lang.org/cargo/reference/config.html#alias)
confirme qu'un alias ne peut pas redéfinir un builtin.

Correction bornée :section alias autorisée uniquement dans config.toml du
dépôt (index0), exactement les deux noms connus, vecteurs de strings commençant
par clippy, hash du fichier toujours capturé. Aucun alias shell, autre commande
ou home override autorisé. Flags LLVM/wrappers/profiles/env/target et clés
build.rustflags restent refusés. Aucun changement de la politique Clippy ou des
alias eux-mêmes.130helpers CI PASS/0skip, dont la vraie config du dépôt admise,
et les cas négatifs shell/build/env/target/home toujours rejetés.
Revue indépendante du seul delta d'admission en cours ;nouveau run à mesurer
après publication. Aucun Cargo de compilation ni runtime local lancé.

Revue indépendante du delta terminée sans finding bloquant :exception limitée
à l'index0 et aux deux vecteurs clippy, hashes conservés, autres rejets intacts.
Le producteur appelle cargo build littéral et n'invoque pas les alias. Les
arguments des alias peuvent varier sans effet sur ce chemin ;aucune garantie
nouvelle sur l'exécution future de ces raccourcis n'est déduite. Alias home
restent volontairement refusés. Le succès réel de la nouvelle CI reste requis.

## Run automatique qualifié pour E2E/Allure/Pages

Correctif741d04359a332de78802ef59cd8b2d1480297787 publié et vérifié origin/master,
hook fmt PASS. [CI43473245852](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43473245852)
déclenchée automatiquement sur push :huit builds verts, E2E et couvertures
démarrent tous à17:33:57UTC. E2E PASS à17:40:27 et Pages PASS à17:40:47,
avant fin couverture17:54. Générateur Allure réel et dépôt réutilisable qualifiés
par ces jobs. Applications/harness/packaging tous liés à741d0435, même run/attempt,
SDK93b890ff ;pas l'ancienT1ca9a4c8. Deltas et sandbox parent PASS, cleanupPASS0.

GETpublic HTTP200 pour accueil/status.json/badge.json/allure/index.html/widget
summary. Compteurpassed34expected34 ;Allure35passed0broken/failed/skipped,
car un contrat parent est séparé des34checks. Bilan durable :
[résultats auto/reporting](../../../missions/20261113-k6-auto-reporting-results.md).
Vérification indépendante finale live/artefacts/provenance/cleanup en cours.

**CI globale failure** :coverage-integration Manifesto (job132053039776), test
t6_post_get_components_contract_bytes_frozen, panic testapparatus_p1_t6_http.rs120
et Tokio task/core.rs422 ;3passed1failed dans ce binary. Pas de cause imputée
au lot CI sans analyse ;pas de réparation métier autorisée implicitement.
Les dix autres jobs couverture et Sonar sont verts, mais aucun90% ni réussite
globale annoncés. Relecture du gros log faite en streaming filtré après un
premier dépassement maxBuffer, sans dump/raw export. Aucun runtime local touché.

## Réception indépendante et clôture du périmètre

infra-verifier confirme sur API/artefacts/site live :graph et timing réels,
E2E et publication success, H==T==packaging741d0435 du même run/attempt,
trois probes packaging fermées, témoins sandbox et cinq processus propres,
nœud/browser/volume retirés avec owned_running0. Campagne14/14SHA256 égaux aux
copies locales ;bundle public111entrées/35résultats tests/0attachment, scan borné
JWT/bearer/ghp_/sentinel sans détection. Statut public identique à l'artefact,
badge34/34 vert, Allure35passes, aucune durée/historique fabriqué (widgettime{}).

La garde Pages est recoupée statiquement et indirectement par le bon artefact
déployé, pas par lecture de son log. Les130tests du parent n'ont pas été rejoués
par le reviewer. CI globale failure conservée distinctement, IT Manifesto non
diagnostiquée/réparée. Aucun90%Sonar, promotion ADR ou global green revendiqué.

Livrables :code CI/reporting/README publiés7e5ec5e1, correctif collecteur741d0435,
bilan de mission et runbook consolidés. Aucun code métier étranger embarqué,
pas de branche/worktree de mission créé, aucun runtime local démarré ou arrêté.
Publication documentaire finale et référence distante à vérifier pour clôture.
