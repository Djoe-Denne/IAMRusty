# 2. Démarrage du portage des scénarios Windows

## Objectif et étape

La révision1 approuvée lance le portage des scénarios d'authentification définis vers k6 Windows, pas une requalification du framework. Le démarrage concerne la correspondance des garanties anciennes et l'adaptation documentaire minimale du contrat. Le code consommateur attendra le contrat intégré et ratifié.

## Changements depuis la préparation

La politique impose OpenAI GPT-6.1 Sol uniquement, sans Astra ni délégation long_run. Les lots destinés à tester les tests, la robustesse du framework et les suites générales de non-régression de l'ancien aperçu sont supprimés. Une adaptation du socle doit être nécessaire à un scénario concret et limitée au plus petit delta utile. La revue indépendante de parité demeure exigée, sans pipeline d'agents supplémentaire automatique.

Deux travaux indépendants sont prêts : inventorier en lecture seule les garanties et contrôles historiques ; rédiger l'extension documentaire minimale du contrat Windows. Leur rapprochement et intégration précèdent les auteurs de code. Le lead examine parallèlement les ancrages documentaires de la cible, sans démarrage de runtime.

## Vérifié au start

Le checkout est toujours sur la même base que la préparation, avec le même SDK. Un seul worktree est présent ; les modifications suivies étrangères concernent le workspace Obsidian et la configuration Serena du sous-module. Elles restent préservées. `git diff --check` a été exécuté avec sortie0 et sans erreur ; aucun test applicatif n'a été exécuté.

## Limites et suite

Aucune cible vivante n'est encore admise, aucune connexion applicative ni k6/Chrome lancé. L'ancienne preuve rapporte le laboratoire arrêté ; cela ne prouve pas son état actuel. Aucun nouveau laboratoire, tunnel, déploiement ou réveil Kind/WSL n'est autorisé implicitement. Les scénarios et la documentation peuvent progresser dans leur périmètre ; la qualification réelle exigera une cible existante et son bail prouvés.

Pas de nouveau GO demandé : poursuite des seuls travaux autorisés, avec barrière d'intégration avant le code.

## Annexe

Révision1/hash8cad30c094e42f69920117ae8e8d7d530fd2cfd930418a3dcfdd49dd78a45d29. HEAD05941d163eb6632f26c89fc7057af03880148a08 ; gitlink02e5db23aac6016aab062c3078696d5f7fd464ef. Commandes : rev-parse HEAD, diff noms/index, status suivi, worktree list, submodule status et diff --check. Aucun runtime créé, aucun bail Cargo acquis.
