# 79 — Consolidation engagée : accès natif manquant pour préparer les branches

## Objectif et état

Réunir les livrables durables des worktrees sur master, branche principale
explicitement choisie, puis terminer l'archivage. La préparation a été déléguée
à un auteur natif, pendant que le lead examinait les branches et leurs fusions.
La consolidation supplémentaire n'est **pas terminée** ; aucun nouveau push.

La mission native a admis automatiquement la tâche de préparation au membre
pool-33. Une tentative de spawn explicite a refusé de réclamer la tâche déjà
attribuée ; aucun second auteur n'a été lancé. L'ancien champ require_context
n'est pas exposé par la signature actuelle ; un contexte fresh explicite et
ownership par chemins ont été fournis dans la tâche elle-même.

## Premier obstacle : préparation des onze arbres

L'auteur a lu le contrat et les quatre rapports de contexte. Son premier read
de `.git` dans le worktree pool-2 a été refusé par les permissions natives :
`permission.rejected / Permission denied: external_directory`.

Chemin concerné :
`C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-d9lbvu-pool-2/.git`.

Le mandat utilisateur couvre les onze arbres mais n'a pas rendu ces répertoires
accessibles à cette session. L'auteur n'a pas changé d'outil pour contourner ce
refus et n'a touché à aucun fichier/HEAD/index. Il a rendu un résultat failed puis
a été arrêté sans force. La tâche de préparation n'est pas déclarée accomplie.

Action nécessaire pour reprendre cette préparation : accès natif effectif de
l'auteur aux onze répertoires attribués, ou mécanisme natif de travail dans ces
arbres qui respecte les refus existants. Un nouveau GO métier seul ne remplace
pas cette permission. Pas de modification improvisée des profils ou du plugin.

## Deuxième obstacle, distinct : libération native d'un arbre déjà intégré

Le lead a appelé `team_merge` sur le livrable propre « noms serveurs Envoy ».
Résultat : fusion reconnue, mais retrait non terminé ; branches conservées :
`OpenCode native registration remains after public inventory reconciliation;
host idempotent retirement is required`.

Ce défaut ne signifie pas que le code Envoy manque de master ; il empêche la
libération administrative de cette ressource. Il n'autorise ni suppression
manuelle de l'inventaire, ni écriture de base interne, ni retrait forcé. La
correction du host n'est pas un préalable de principe à la préparation du code :
l'accès des auteurs peut être résolu et les livrables préparés indépendamment.

## Investigations indépendantes exécutées

- Inventaire des23refs préservées de mission et de leurs deltas committés.
- Prévisualisations `git merge-tree --write-tree --name-only --no-messages` :
  conflits sur certaines anciennes propositions et copies de code, aucune mutation
  de l'arbre/index canonique. Les versions anciennes ne doivent pas être fusionnées
  sans résolution en faveur du comportement ratifié et des corrections finales.
- Branches pool11/pool17/pool18 : prévisualisations sans conflit. Ce résultat ne
  capture pas les fichiers encore non committés et ne vaut pas intégration.
- Dernier contrôle canonique : HEAD052817d, index vide ; seules les modifications
  étrangères workspace Obsidian et état du sous-module restent dans le diff.

Pas de Cargo/test/runtime/publication, pas de modification applicative. Le lead
ne présente ni la préparation failed ni la fusion partiellement retirée comme
une consolidation générale réussie. Les données et branches restent préservées.
