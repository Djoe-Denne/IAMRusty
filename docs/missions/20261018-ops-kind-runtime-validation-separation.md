# Ordre de mission — Clarifier ops : environnement Kind générique et validation

**PROPOSITION — préparation seulement.** La mission m6 est close. Son GO et sa
policy ne lancent pas cette nouvelle mission. Préparer une policy locale, la
présenter avec `/ensemble-mission config`, puis attendre le `/ensemble-mission start`
utilisateur avant agents/classification/implémentation. Aucun runtime ni nettoyage
pour préparer cet ordre. Aucun agent de ce nouvel audit n'a encore été lancé.

## 1. Demande utilisateur et frontière impérative

L'utilisateur veut un dossier `ops` compréhensible, avec :

1. les fichiers **génériques nécessaires à la construction/livraison et au lancement
   d'un environnement Kind complet** ;
2. les **scripts explicites de validation** : curl/smoke/E2E et lanceurs de suites,
   identifiables et séparés du déploiement.

Choix confirmé : **« Déploiement + validation »**.

Précision utilisateur : les **fixtures des tests d'intégration Rust sont définies
dans les tests et injectées dans leurs dépendances au lancement**. Elles ne sont
pas un contenu à installer dans `ops`. Les tests natifs démarrent leurs dépendances
par testcontainers et préparent eux-mêmes leur état ; ils n'ont pas à déployer une
plateforme Kind pour pouvoir fonctionner.

Un lanceur IT conservé sous `ops` n'est qu'une interface explicite vers les suites
natives Windows : il ne recrée pas leurs conteneurs, jeux de données ou injections.
Un driver E2E/curl peut consommer un environnement Kind déployé séparément ; le
déploiement doit fonctionner sans devoir exécuter ce driver ni une suite Rust.

**Ne pas lancer un nettoyage par nom de fichier.** Un bootstrap OpenFGA, une
migration, une politique réseau ou une préparation de certificats peut être requis
par le vrai runtime ; ce n'est pas une fixture simplement parce que c'est un helper.
Réciproquement, un test vert ou un fichier sous `ops` ne rend pas générique un
scénario qui construit sa propre application de démonstration ou modifie un cluster.

## 2. Ancrage et lectures

- Dépôt : `C:/Users/djden/source/repos/AIForAll` ; HEAD de préparation
  `80918ca30ce6624d297917ffc4c986744e70b1b7`. Revalider au start.
- `AGENTS.md`, `.cursor/rules/infra-safety.mdc`, `agent-handoff.mdc`,
  `docker-resource-registry.mdc` et `.agents/skills/aiforall-test-evidence/SKILL.md`.
- `ops/deploy/README.md`, `ops/deploy/apps/overlays/local-full/README.md`,
  `ops/deploy/PACKAGE-D-CHECKPOINT.md`, `justfile` ; ADR/contrats référencés par
  ces fichiers, lus selon les dépendances trouvées, pas une réexploration globale.
- `docs/missions/20261018-apparatus-m6-proof-hardening-results.md` et ledger m6
  pour les faits/limites acquis. Ne pas rejouer la campagne pour redécouvrir ses faits.
- Nouveau ledger : `.cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md`.
  Lead seul écrivain canonique ; les lecteurs rendent leurs deltas.

## 3. Première photographie — faits, pas verdict d'audit

Inventaire Git initial : **150 fichiers suivis sous ops, dont7 `.pyc` dans
`ops/deploy/__pycache__`**. Aucun changement courant observé sous ops/justfile/.github
ou les tests Apparatus au point de préparation. Les modifications wiki préexistantes
hors périmètre restent préservées. Inventorier aussi les fichiers ignorés/non suivis
par métadonnées, sans lire clés, credentials ou sorties privées générées.

Points de départ observés :

- `ops/deploy/verify-m1.ps1`, `verify-m2.ps1`, `verify-m3.ps1`, `deploy-j2.ps1`,
  `deploy-j3.ps1`, `deploy-mesh.ps1` et recettes `just deploy-*` : jalons/démos à
  classer, pas autant de chemins nominaux à conserver automatiquement.
- `ops/deploy/apps/overlays/kind` contient des stubs et une sonde d'invocation ;
  `kind-demo-monolith` porte une démo hybride hôte/Compose, un gold path et un
  staging du plugin de référence. Ce n'est pas automatiquement le Kind autonome complet.
- `ops/deploy/deploy-local-full.py`, `render-local-full.py`,
  `bootstrap-local-full.ps1`, `local_full_target.py`, images/storage/recovery,
  `kind/cluster-local-full.yaml` et `apps/overlays/local-full/` constituent un
  candidat existant à examiner. README et checkpoint disent **sources/statique**,
  pas lab complet prouvé en runtime. Ne pas promouvoir cette déclaration en succès.
- `ops/scripts/mesh-authn-kind-e2e*.sh`, `mesh-authn-e2e*.sh`,
  `ops/tests/local-full-acceptance.py`, `infra-static.sh`, tests Python des helpers :
  validation, maintenance et tests de scripts à distinguer entre eux.
- `ops/cloud`, `ops/docker`, `ops/openfga`, certificats, scripts de renommage et
  checkpoint historique : usages hors cible Kind potentiellement légitimes ;
  **pas de suppression collective sous prétexte qu'ils ne servent pas m6**.

### Exemple de dépendance partagée à tracer

`justfile:140–156` documente que la fixture `rustycog-testing::TestOpenFga` charge
`ops/openfga/model.json`. C'est une piste de consommation d'un **contrat commun**,
pas la preuve que les données de fixture Rust devraient vivre dans ops. Vérifier le
consommateur réel ; décider un emplacement canonique et une migration des références
si nécessaire, sans dupliquer les modèles ni réécrire le framework implicitement.

## 4. Résultat attendu

Un utilisateur doit pouvoir identifier sans connaissance des anciennes missions :

- comment préparer les artefacts Linux, config/certificats locaux et dépendances ;
- quelle commande lance le profil Kind complet, et ce qu'il inclut/simule/exclut ;
- comment vérifier disponibilité puis parcours métier par des drivers explicites ;
- comment lancer les **IT natives Windows**, séparément, sans Kind ;
- où sont les fixtures Rust et leurs données : dans les suites/harness concernés,
  pas cachées dans un déploiement générique ;
- quels chemins historiques sont encore supportés, déplacés, archivés ou supprimés.

Le nominal ne doit pas importer `tests/`, `.tmp`, `.cursor`, un ledger de mission,
des noms `pool-*`, une ancienne sortie de campagne ou des fichiers privés personnels.
Un protocole de sécurité générique de contexte/UID/propriété reste légitime :
**retirer le couplage au harness, pas les protections opérationnelles**.

## 5. Audit : trois lectures parallèles, strictement RO

Scores complexité/risque indépendants, pas probabilités. Un seul dépôt canonique,
pas de worktree nécessaire pour les lecteurs, pas de Cargo/import exécutable/API.
Chaque lecteur fournit preuves `file:line`, effets transitifs, faits/hypothèses et
delta HANDOFF ; le lead consolide, sans écritures concurrentes d'inventaire.

### A — Runtime et fermeture des dépendances (explore,48/58)

Périmètre : `ops/deploy`, manifests/bases/overlays, images, CNI, certificats,
initialisation de données/config nécessaires au fonctionnement normal.
Livrable : graphe entrypoint→scripts→manifests→images/config ; comparaison
générique/local-full/démos/stubs/hybride ; liste inclut/simule/exclut et points
nécessitant des changements applicatifs hors mission. Ne pas assimiler un rendu
Kustomize valide à un déploiement fonctionnel.

### B — Validation et couplages aux harness (explore,45/55)

Périmètre : `ops/scripts`, `ops/tests`, drivers de preuve dans les overlays et
tests des helpers de déploiement. Pour chaque driver : cible réelle, préconditions,
mutations, assertions, erreurs/skip, fichiers créés et cleanup ; repérer les scénarios
spécifiques m5/m6/gold sans préjuger qu'ils doivent tous disparaître.
Livrable : distinction smoke/curl métier/E2E/IT native/test de script/fixture interne,
avec proposition d'interface runtime vs validation. Pas d'exécution des scripts.

### C — Consommateurs et compatibilité (explore,48/62)

Périmètre en lecture : `justfile`, CI, docs, Dockerfiles/stagings, tests Rust,
fixtures communes/rustycog au pin existant, références de chemins ops.
Livrable : références inverses par candidat au déplacement/suppression, notamment
contrats partagés OpenFGA et fixtures Rust ; effets transitifs DELETE/seed/build
et dépendances cachées sur `.tmp`/hôte/ancien profil. Aucune édition framework/app.

## 6. Manifeste de tri obligatoire avant toute réorganisation

Pour chaque fichier suivi et chaque famille ignorée pertinente : rôle,
producteur/consommateurs vérifiés, dépendances, effet(s), statut de preuve,
destination proposée, décision et justification. Catégories :

- **KEEP-RUNTIME** : runtime/build/configuration/bootstrap génériques ;
- **KEEP-VALIDATION** : drivers explicites et tests de ces outils, séparés du deploy ;
- **SHARED-CONTRACT** : modèles/schémas canoniques consommés aussi par des tests ;
- **MOVE-TEST-INTERNAL** : assets exclusivement propres au harness/tests Rust ;
- **ARCHIVE-HISTORY** : checkpoint, procédure jalonnée ou démo non nominale ;
- **REMOVE-GENERATED** : sorties reproductibles suivies par erreur, dont `.pyc`
  candidats après vérification des références et règles d'ignore ;
- **OTHER-OPS / DEFER-UNKNOWN** : autre mode légitime ou preuve insuffisante,
  à conserver jusqu'à arbitrage explicite.

Aucune déduction «inutilisé» sur une seule recherche vide. Inclure Git/non-Git,
CI/docs/recettes/références dynamiques ; préserver l'historique utile, ne pas le
mettre dans une poubelle puis garder ses liens comme si le nominal l'utilisait.

## 7. Décision, QA et implémentation futures

### D — Contrat de séparation (architecte,60/65 ; après A/B/C)
Consolider les contradictions, choisir un **seul chemin nominal complet** et une
interface claire des validations ; décider l'arborescence minimale à partir des
usages, pas imposer une factorisation. Contrat Proposed sous `docs/contracts/`,
ADR Proposed seulement si décision structurelle nécessaire. Publier manifeste de
tri et compatibilité des commandes ; arbitrage utilisateur avant suppressions
de capacités ou élargissement de périmètre. Ne pas réécrire les ADR historiques.

### Q — QA avant writers (qa-planner,52/60)
Registre obligation→test avec chemins/références, checks statiques hermétiques,
garde de contexte, credentials, ordre build/deploy/validation, comportements
fail-loud ; gaps nommés avec owner/impact/raison/réouverture. Définir les commandes
et oracles attendus, pas une recherche de tests après coup.

### I — Lots d'implémentation après contrat et QA acceptés
- I-runtime : owner unique entrypoints/manifests/build/config,76/75 ; simplification
  sans perte des dépendances, readiness, CNI, sécurité, persistance nécessaires.
- I-validation : owner drivers/interfaces/tests des scripts,72/70 ; séparer le
  lancement du contrôle, rendre les entrées/configs explicites, pas de seed privé
  ou déploiement implicite dans un simple curl.
- I-références/historique :70/68 ; migrations de chemins/recettes/CI/docs selon
  manifeste approuvé, retrait borné des générés ; jamais d'écrasement de travail
  utilisateur. Les frontières exactes et dépendances seront fixées par D pour
  éviter deux écrivains sur un même script/overlay.

Writers indépendants en worktrees isolés, continuité native choisie explicitement,
aucun modèle override. Plancher développement70 respecté. Si un problème exige du
code de production/framework, rendre un lot distinct à approuver, pas le glisser ici.

## 8. Gates et preuves de sortie

1. **Statique d'abord** : inventaire complet, références avant/après, parsing
   PS/Python/shell sans effets de bord, rendus hors ligne sans secrets ni logs
   de clés, tests hermétiques des gardes/consommateurs. Pas de suppression d'une
   assertion gênante pour rendre un script vert.
2. **Natif Windows** : un Cargo à la fois, target E:, tests touchés puis IT finales
   via testcontainers existants. Les IT ne dépendent pas du profil Kind déployé.
3. **Runtime seulement après GO explicite de cette mission**, corrections/gates/IT
   terminées : inventaire et bail, cluster/contexte exacts contractualisés. Le
   profil complet existant cible `aiforall-local-full`, pas un renommage du cluster
   m6 `aiforall-local`. Pas de création/restart/retarget pendant l'audit.
4. Preuve du chemin nominal : déploiement réel, readiness utile, validation curl
   métier explicite avec négatifs discriminants, résultats/counts et captures
   non secrètes ; distinguer smoke, E2E et IT. Aucun succès sur outil absent/skip.
5. Review ciblée correctness/tests/security ; infra-verifier après changement
   infra significatif. Corriger les findings ou accepter/différer explicitement.
6. Nettoyage des seules ressources possédées sans bail actif, IDs/UID observés,
   registre à la création, aucune adoption/prune/reset. Images/cache/volumes
   inconnus conservés et rapportés. Pas d'extinction d'un runtime partagé inconnu.

Critères : chemin nominal documenté et prouvé, manifeste tri complet, fixtures
Rust indépendantes de Kind, validations séparées et vérifiables, aucun chemin
cassé/secret généré suivi, dettes restantes nommées. Les démos encore utiles ou
cloud/autres modes ne disparaissent pas sans décision. «Moins de fichiers» n'est
pas un oracle ; **moins de couplages et une utilisation claire** en est un.

## 9. Politique de mission et hors périmètre

Préparer une nouvelle grille native, sans modifier la policy m6 close ni le global.
Dernière préférence utilisateur connue à représenter/vérifier, pas autorisation :
light `opencode-go/gpt-6-luna`/none, standard `opencode-go/glm-5.3-flash`/max,
strong `openai/gpt-6.1-sol`/xhigh, pas Opus, aucun override. Audits standard,
implémentations strong selon scores ; disponibilités et grille à prévisualiser.
Pas de JEV/helper implicite ni de contournement de la gate si classification bloquée.

Hors mission : runtime pendant préparation, prod/cloud/HA nouveaux, migration
générale RustyCog, réarchitecture applicative, suppression de toutes les démos ou
de tout `ops/cloud` par principe, réparation du harness OpenCode, relance des
quinze anciennes suites Kind, annulation des preuves/acceptations de m6. Aucun
commit/stash/reset/purge des changements préexistants ni push automatique.
