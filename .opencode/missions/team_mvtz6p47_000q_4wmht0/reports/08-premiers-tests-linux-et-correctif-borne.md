# Premier test Linux : un refus de chemin à corriger

## Objectif et étape

Après la revue statique ciblée sans finding, le lead a lancé les tests du socle sous Ubuntu24.04/WSL, avec un utilisateur non privilégié. L'objectif est de vérifier les comportements POSIX que Windows avait ignorés, avant les vrais outils et Chromium.

## Résultat observé

37tests ont été exécutés :1échec et2erreurs, exit1. Le lanceur rejette une sortie dont le parent est privé parce que son grand-parent usuel est public. Trois tests s'arrêtent donc avant d'exercer les processus attendus. Les succès Windows ne pouvaient pas détecter ce défaut ; aucun résultat Linux vert n'est annoncé.

L'environnement correspond aux prérequis : Ubuntu24.04, x86_64, UID1000, Python disponible. Il s'agit d'un défaut du dispositif sur un cas courant à couvrir, pas d'une panne Docker, d'une preuve HTTP rouge ou d'un problème de permissions à contourner.

## Suite recommandée, déjà dans le mandat

L'auteur reprend dans sa même session pour corriger uniquement les validations de sortie et leurs tests. Préserver les exigences de propriétaire/droits/liens ; ne pas rendre /tmp ou le runner artificiellement privés et ne pas affaiblir les contrôles pour masquer l'échec. Ajouter une régression ciblée et rendre les tests négatifs réellement discriminants.

Le lead rejouera les commandes Linux concernées puis fera contrôler le delta sensible. Aucun nouveau GO de jalon ni pipeline supplémentaire. Les qualifications outils, navigateur et HTTP ne sont pas acquises et ne seront pas fusionnées dans un PASS partiel.

## Runtime et preuves

La distribution ARTIST-Ubuntu était arrêtée avant ce préflight. Le diagnostic metadata en root a identifié un compte existant artist ; seul ce compte non-root a exécuté les tests. Aucun compte créé, aucun réglage global changé. Docker Desktop et son unique conteneur préexistants restent étrangers à la mission, aucun container créé/redémarré. Bail parent .tmp/k6-linux-qualification-mvtz6p47.json ; restitution après campagne bornée et inventaire.

Trace utile complète, commande et fichiers/lignes : .cursor/review-briefings/20261029-k6-linux-first-run-path-invalid.md. Source db16aa7. Aucun k6/Chromium ni requête applicative exécuté.
