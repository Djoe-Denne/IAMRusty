# Nettoyage du lancement Kind — plan de préparation

Statut : **exécution directe en cours** (2026-11-09), cadrée par ADR0610 Proposed et contrat `kind-local-native-launcher.md` v1.1. Le GO utilisateur porte sur le nettoyage ; l'utilisateur n'a pas choisi Ensemble. Aucune création de mission ou commande `config/start` n'est requise pour le travail direct. Consultations ponctuelles architecture/review/diagnostic ; la racine écrit et intègre le code.

## Résultat attendu

Une seule entrée locale reconstruit les images et fait tourner les services dans Kind, avec des outils standards. Les ressources de stockage, DB et caches sont conservées par défaut. Une option explicite `--reset-data` permet de les effacer dans le périmètre du lab. Pas de réécriture du déployeur Python en shell/PowerShell, ni de nouveau framework de preuve, d'état ou de reprise.

- Kind/configuration YAML ; Bake/Dockerfiles ; `kind load` ; Helm/Kustomize/kubectl ; OpenSSL et configuration `.cnf` pour les certificats locaux.
- HTTPS pour les flux HTTP ; TLS natif pour les autres protocoles concernés. Secrets et clés hors Git, valeurs Helm, argv et sorties publiques.
- Retirer du lancement les ancrages cluster/daemon, baux/ACK/nonces, preuves spécialisées, publication OCI applicative, collecte et sauvegarde automatiques, refresh image-only et compatibilités sans nécessité fonctionnelle.
- Ne pas supprimer les campagnes/tests spécialisés sous prétexte qu'ils ne font plus partie du lancement. Retirer le code devenu sans consommateur après intégration de ses remplacements.
- La règle utilisateur reste : **capacité provider Oui et outillage spécifique Non ⇒ le mécanisme Python disparaît de la cible**. Si le but n'est pas requis au lancement minimal, aucun remplacement nominal.

## Point de départ et conservation

- Racine : `C:/Users/djden/source/repos/AIForAll`, `master`, HEAD `51996139ec31317aaef85e49ab3c9266591f3902`. Sous-module `rustycog` : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.
- Au préflight de préparation : 106 entrées `git status --porcelain`, candidat antérieur non publié. Deux worktrees/branches historiques : `.worktrees/ops-iac-build-network` et `.worktrees/ops-iac-registry`, tous au même HEAD. Ne pas les effacer ou repartir de HEAD en ignorant leurs livrables.
- Handoffs : `.cursor/handoffs/20261109-ops-iac.md` et `.cursor/handoffs/20261109-ops-python-provider-local.md` ; sauvegardes existantes `.tmp/ops-iac-integration/`. Les preuves offline antérieures sont bornées au candidat qu'elles ont vérifié ; revues interrompues et runtime NOT RUN restent tels quels.
- Avant tout retrait, la racine préserve et vérifie le candidat complet. Aucun stash/reset/commit de changements étrangers. Le mode direct n'exige pas de nouveaux worktrees ou de snapshot destiné à des auteurs isolés ; les livrables historiques restent à intégrer/préserver pour la publication finale.

## Cadrage structurel préalable

Le contrat actuel `docs/contracts/ops-local-full-declarative-iac.md` v1.7 et ADR-0609 sont **Proposed / Partial**, pas Accepted. Ils décrivent encore des garanties que la nouvelle cible ne veut plus maintenir. Le profil `architecte` doit écrire le delta de contrat avant implémentation, sans code applicatif et sans réintroduire ces garanties par défaut.

Points à fixer une fois :

1. Une interface de build/import et une interface de lancement ; ownership des fichiers partagés réservé au lead.
2. Classification déclarative des workloads de calcul et dépendances de données ; portée précise de `--reset-data`.
3. Préservation des données avec Kind : `ops/deploy/kind/cluster-local-full.yaml` ne déclare pas de montage hôte, les PVC actuels demandent `standard`. La solution minimale est de garder les nœuds porteurs d'état et remplacer les Pods applicatifs. Supprimer les nœuds en conservant leurs données exige une persistance externalisée et ne peut être promis par un simple label « stockage ».
4. Rebuild réellement complet : `Dockerfile.build` monte actuellement un cache Cargo target. Un `--no-cache` Docker ou une suppression d'image ne suffit pas ; utiliser un target Cargo neuf pour ce mode, sans purge globale du cache des autres travaux.
5. Limitation **OPS-OPENBAO-262-REDIRECTS** dans `docs/guides/ops-declarative-boundaries.md` : HTTPS ne corrige pas le clone auto-auth 2.6.2. Pas de bump, contournement ou perte de protection implicite. Traiter la frontière concernée sans bloquer les lots indépendants ; aucune version corrigée n'est présumée qualifiée.

## Lots et dépendances

Les noms ci-dessous sont des lots techniques, pas des tâches d'équipe à publier. **La racine prend les deux lots en direct et garde l'intégration.** La proposition antérieure de deux auteurs parallèles reste une possibilité, pas une organisation imposée. Une éventuelle consultation spécialisée ne crée pas implicitement une mission Ensemble ; les décisions structurelles restent à architecte.

### Cadrage — architecte, après préservation de la base

- Livrable : delta d'ADR Proposed/contrat, interfaces des deux auteurs, décision de conservation/reset des données et disposition de la limitation OpenBao.
- Ne modifie pas de code. Le lead intègre le résultat avant de publier les deux lots consommateurs.

### Build — lot exécuté par la racine

- Propriété : `Dockerfile.build`, `ops/deploy/build/**`, inventaire/contexte/chargement d'images et tests de cette frontière, selon contrat intégré.
- Remplacer les collecteurs et pilotes Python par les recettes natives ; conserver les entrées locales non committées et le SDK, exclure secrets et target Windows.
- Livrable vérifiable : build de livraison défini, sorties importables, référence de build non ambiguë et retrait de l'ancien chemin de cette frontière. Les raccords au lanceur sont séquencés avec le lot suivant, sans deuxième écrivain concurrent.

### Lancement — lot exécuté par la racine

- Propriété : lanceur et phases, chart/profils Kind, séparation calcul/données, Secrets/TLS et tests correspondants, selon contrat intégré.
- Utiliser les déclarations et outils natifs ; retirer la machine d'autorité/reprise et les fonctions hors minimum ; préserver les initialisations/migrations réellement nécessaires.
- Livrable vérifiable : parcours normal et option destructive bornée, attente des services prêts, erreurs natives propagées, pas d'adoption des ressources étrangères. La racine effectue les raccords Build/Lancement sur le candidat réellement conservé.

### Intégration et suppression finale — lead, après livraison intégrée des deux lots

- Le lead réserve `justfile`, les interfaces publiques communes, tests/CI partagés, documentation et ledger.
- Examiner le diff réellement conservé avant de migrer ses consommateurs ; ne pas supprimer une façade avant que son remplacement soit disponible et vérifié.
- Retirer les façades, imports, branches de compatibilité et code sans consommateur ; disposer les anciens tests par risque au lieu de simplement les effacer.
- Travail direct par lots ; mutations et compilations Cargo séquentielles. Pas de PM, équipe ou chaîne de reviewers automatique. Si une délégation devient réellement utile, la borner et conserver l'ownership sans imposer Ensemble.

## Vérification proposée pour config

Proposition **proportionate**, sans classification, scores, modèle imposé ni plancher. Les commandes et critères ci-dessous servent au travail direct ; ils ne sont pas soumis à une procédure Ensemble que l'utilisateur n'a pas choisie. Si Ensemble était demandé ultérieurement, sa policy native devrait alors être présentée/approuvée selon son cycle propre.

Commandes/statique à adapter uniquement aux chemins réellement conservés :

- `python -B -m unittest discover -s ops/tests -p "test_*.py"` pour les tests Ops restants ; distinguer skips et tests non exécutés.
- `bash ops/tests/infra-static.sh` : rendus Helm/Kustomize et cohérence publique, sans accès aux secrets.
- `docker buildx bake --print` et dry-run des recettes retenues ; syntaxe des scripts natifs touchés.
- `git diff --check` sur les livrables intégrés ; contrôles de fichiers nouveaux également, pas seulement diff de fichiers suivis.
- Extension utilisateur **TLS natif Redis** : units `lazaret-configuration`/`lazaret-infra`, check/Clippy des trois crates raccordées ; IT réelle ciblée `ops/tests/it-native.ps1 -Suite lazaret -TestBinary redis_tls` via le harness existant (une sélection exacte ajoutée, pas de nouveau runner). Bonne/mauvaise CA/SAN, refus du plaintext sans fallback et sentinelle avant/après transition TLS ; oracles legacy conservés. Cette transition en place ne prouve pas la persistance au redémarrage Kind, qui reste un gate distinct.
- Mesure Python au périmètre constant : tous les `ops/**/*.py` hors `ops/tests/**`, plus le seul entrypoint `ops/tests/support/oauth-idp-simulator.py`. Référence candidate : 7 267 lignes / 29 fichiers / 310 fonctions AST. Compter les relocalisations ; ne pas minifier ou changer le périmètre pour annoncer une baisse.

Critères comportementaux :

1. La commande normale reconstruit les images requises et termine seulement quand les services activés sont disponibles ; une panne de build, bootstrap ou rollout produit un échec réel.
2. Une sentinelle de données de test demeure après rebuild normal ; DB/cache/stockage ne sont pas recréés implicitement.
3. Sans option destructive, aucune suppression de données ; avec option explicite, seules les données de test rattachées au périmètre autorisé sont effacées/réinitialisées. Ne pas tester ce cas sur les données historiques sans disposition/autorisation ciblée.
4. Certificats OpenSSL utilisables pour les noms réellement employés et confiance TLS explicite, sans mode insecure. La limitation Agent reste distincte.
5. Les fonctionnalités ciblées n'ont plus d'ancien moteur Python nominal ni de second moteur impératif équivalent. Les fonctionnalités de tests hors lancement ne sont pas présentées comme migrées si elles sont seulement retirées du chemin nominal.

Une revue indépendante bornée de la surface destructive/secrets/TLS est justifiée après intégration ; briefing fresh persisté. `infra-verifier` après changement infra significatif. Les manques de tests bloquants peuvent être confiés à `test-engineer`, sans lui faire corriger la production. Pas de quatre reviewers systématiques.

## Ressources et clôture

- Aucun runtime à démarrer avant les corrections et gates statiques/unitaires. Avant IT/E2E, charger les skills lifecycle/evidence et vérifier préconditions/baux ; Windows natif pour dev/tests, un seul Cargo avec 24 jobs configurés. Docker construit seulement les images Linux Kind et fournit les tiers des fixtures IT existantes ; IT jamais via Kind.
- Opérations réelles limitées aux cibles locales autorisées et ressources identifiées de ce lab. Aucun prune global, nettoyage d'autres projets ou écriture sur contexte protégé ; registre Docker contemporain pour les ressources créées/redémarrées et cleanup possédé vérifié.
- Fin de mission d'implémentation : intégrer les livrables durables conservés, validations pertinentes, commit/push de `master` et vérification distante ; disposer les branches/worktrees historiques et nouveaux sans perte de données, puis cleanup/archivage natifs. Un blocage restant doit être nommé, pas converti en PASS ou clôture administrative.

## Mode d'exécution retenu

Rester dans cette session, racine exécutante et responsable de l'intégration. Aucune commande `/ensemble-mission` n'est demandée. Le `config/start` natif ne s'applique que si l'utilisateur choisit explicitement Ensemble ; il ne doit pas être présenté comme une condition de tout nettoyage ou de toute discussion sur une délégation.
