# Clôturer l'authentification — préparation et choix de vérification

## En bref

Le but est de corriger puis démontrer la chaîne d'authentification supportée, sans transformer la mission en qualification de toute la production. L'exploration a confirmé de nombreuses protections et leurs tests existants, mais a relevé une perte de l'autorité d'identité avant les permissions, un signataire distant moins durci et des incohérences dans la preuve réseau/navigateur.

La recommandation est une QA proportionnée : tests ciblés qui distinguent les vraies protections des faux refus, revue de sécurité indépendante et vérification des changements réseau. Une simple passe de connexion nominale ne suffirait pas ; une campagne exhaustive de toute la plateforme n'est pas nécessaire non plus.

## Ce qui a changé depuis l'exploration

- Plan borné persisté dans `../task-plan.md`, avec ownership, dépendances, frontières d'architecture et de runtime, registre des preuves manquantes et tests réutilisables.
- Proposition `verification` enrichie : commandes de référence et critères de réussite. Niveau proportionate conservé ; catalogue, seuil de départ et classification inchangés.
- Aucun worker ni classificateur lancé, aucune tâche publiée, aucun code applicatif changé et aucun test ou runtime démarré dans cette préparation.

## Aperçu de l'exécution proposée

1. Après start, préparer en parallèle les décisions d'architecture et les tests discriminants des défauts observés.
2. Après intégration des entrées, durcir en parallèle le signataire distant et la vérification des serveurs ; corriger les permissions une fois le contrat ratifié.
3. Intégrer puis vérifier sur Windows, avec un seul Cargo et les 24 jobs configurés.
4. Préparer la preuve navigateur/passerelle et son autorisation opérationnelle ciblée, puis effectuer les revues utiles et réconcilier la documentation selon les résultats.

Les consommateurs de livrables isolés ne seront publiés qu'après intégration. Les décisions sur la représentation des identités et sur l'entrée navigateur restent ouvertes. Aucune suppression d'une fonctionnalité, publication distante ou modification de contrat n'est cachée dans ce plan.

## Vérification : choix et conséquences

- **Light** réduirait les contrôles aux vérifications ciblées des auteurs. Pour ces frontières de confiance, il faudrait néanmoins conserver les exigences de sécurité et de réseau du projet ; aucune dérogation implicite n'est proposée.
- **Proportionate, recommandé** conserve les preuves existantes, ajoute les scénarios manquants et une revue indépendante de sécurité, avec revue bornée des oracles et infra-verifier pour le réseau. Pas de pipeline de quatre agents ni de scan global imposé.
- **Reinforced** demanderait un plan QA dédié avant auteurs et une revue indépendante plus étendue, avec davantage de travail de préparation/vérification. Son coût et sa durée ne sont pas chiffrés.

Reporter start laisse les sources et le runtime inchangés ; les risques observés restent ouverts. La configuration enrichie doit être représentée avant approbation. L'utilisateur peut corriger le périmètre ou la QA dans le chat, puis utiliser `config` et `start`.

## Politique réelle proposée

Version 2 ; seuil de départ d'intelligence 50 ; classification désactivée, sans scores de difficulté ; aucune délégation long_run. Le modèle de chaque tâche reste fixe ; son raisonnement peut évoluer dans les variantes admises de ce modèle. Au seuil actuel, les couples Sol high et Sol xhigh atteignent le seuil de planification. Le routage natif décide, pas un override du lead.

Catalogue inchangé, format « variante : score d'intelligence, coût moyen par tâche » :

- `opencode-go/claude-haiku-5-5` : low 29 / 0.02 ; medium 34 / 0.05 ; high 38 / 0.08 ; xhigh 41 / 0.12 ; max 43 / 0.21.
- `opencode-go/deepseek-v4.1-flash` : max 39 / 0.27.
- `zai-coding-plan/glm-5.3-flash` : max 42 / 0.25.
- `opencode-go/glm-5.3-flash` : max 42 / 0.25.
- `openai/gpt-6.1-sol` : low 42 / 0.13 ; medium 48 / 0.21 ; high 50 / 0.32 ; xhigh 51 / 0.39.
- `opencode/gemini-3.8-flash` : medium 40 / 0.93 ; high 41 / 1.24.

Ces métriques ne sont ni des probabilités de réussite ni un budget total estimé. La présence au catalogue ne supprime pas le seuil de départ. Les variantes d'exécution du même modèle restent gérées par la politique native.

## Preuves et limites

Base de l'exploration et de la préparation : application `c72ae362b950c63d6736a6818c68babc5a8d6f40`, SDK `d60a88b2f2f499768a807941a1b4c80c9a366b3f`. Les preuves et chemins pertinents sont dans le plan et le ledger `.cursor/handoffs/20261028-auth-security-closure.md`.

Le rendu hors ligne de 112 objets a montré une entrée TLS avec certificat client obligatoire et cinq destinations sans correspondance SAN explicite. Aucun trafic cluster, aucun nouveau test Rust et aucune exploitation n'ont été exécutés. Les constats statiques ne sont pas des échecs de tests reproduits. Le risque de collision d'identités exige un test discriminant et un choix d'architecture avant correction.

Validation de la préparation : JSON lisible, 14 couples modèle/variante conservés, 14 commandes de référence et 11 critères ; aucun espace final ou caractère de remplacement dans les quatre artefacts. `git diff --check` exit 0. Aucun delta applicatif ; le seul état de travail du SDK reste sa configuration `.serena/project.yml` préexistante, pin d60a88b inchangé. Aucun build/test/runtime ni publication.

Prochaine étape : présenter la proposition QA enrichie avec `/ensemble-mission config`, puis attendre le `/ensemble-mission start` de l'utilisateur. Aucun ancien hash de proposition ne vaut approbation des nouveaux critères.
