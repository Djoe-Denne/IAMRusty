# Livrables durables, worktrees et fichiers de campagne

Le lieu de création d'un fichier ne décide pas de sa publication. Le code, la
documentation, les tests de régression et l'instrumentation **durables ou
réutilisables** doivent rejoindre la branche principale, même lorsqu'ils ont été
produits dans un worktree temporaire.

Un worktree est une copie de travail Git isolée. On intègre son **livrable**, pas
son dossier `.git`, sa copie complète du dépôt ou son répertoire d'exécution.
L'exclusion de `.worktrees/` ne prouve pas que son travail a été intégré.

## Conserver dans la branche principale

- Code applicatif, migrations, configuration distribuée et contrats retenus.
- Instrumentation réutilisable : observation bornée, diagnostics sûrs, scripts
  de déploiement et de vérification, règles de contrôle et leurs tests.
- Tests de régression et fixtures **sources** reproductibles, sans secret réel.
  Le fait qu'un outil serve aux tests ne le rend pas jetable.
- Documentation, décisions, limites connues et conclusions utiles des revues.
  Extraire le contenu durable d'un compte rendu mêlant sorties privées et faits.
- Pour un sous-module, le commit publié du SDK et le gitlink exact dans le dépôt
  consommateur ; pas une modification locale non publiée du SDK.

## Garder hors publication

- Logs, captures brutes, résultats et données créées par un run.
- Cookies, tokens, clés réelles, profils navigateur et autres secrets.
- Binaires, caches, environnements locaux, PID, baux et métadonnées Git.
- Scripts exploratoires jetables. Un script devenu permanent doit être nettoyé,
  revu, placé dans le dossier approprié et validé avant publication.
- Propositions remplacées ou refusées : conserver si utile la décision de rejet,
  pas réintroduire leur implémentation comme si elle avait été retenue.

## Vérifier une intégration

1. Inventorier changements committés, indexés, non indexés et fichiers nouveaux
   des worktrees du lot ; examiner aussi les sous-modules.
2. Comparer les contenus à la branche principale, pas seulement les chemins.
   Après squash, une histoire Git séparée n'implique pas un changement perdu.
3. Distinguer contenu identique, version corrigée ultérieurement, alternative
   rejetée et vrai complément absent.
4. Examiner les éléments ignorés pertinents : `.gitignore` ne classe pas leur
   valeur documentaire. Ne pas forcer l'ajout d'un dossier entier.
5. Intégrer les seuls compléments retenus, contrôler la liste indexée avant commit
   et la référence distante après push. Ni fusion globale ni `git add -A` ne
   constituent une preuve de complétude.

Une vérification de chemins ou de fins de ligne seule n'établit pas l'équivalence
fonctionnelle. Préciser les lots couverts ; ne pas prétendre avoir audité tout
ancien worktree enregistré dans Git.

## Clôture obligatoire du lot

Règle projet : `AGENTS.md`, section « Fin de mission — intégration, publication et
nettoyage (2026-10-29) », rappelée dans `.cursor/rules/mission-closeout.mdc`.
La fin d'une mission comprend l'intégration de toutes ses branches, le commit et
le push vérifiés sur la branche principale, puis le nettoyage de ses branches et
worktrees et l'archivage natif. Une sauvegarde seule ou un tableau de tâches vert
ne remplace aucune de ces étapes. Un blocage restant doit être annoncé comme tel.

Exemple : [réconciliation de la campagne authentification](../missions/20261029-auth-security-publication.md).
