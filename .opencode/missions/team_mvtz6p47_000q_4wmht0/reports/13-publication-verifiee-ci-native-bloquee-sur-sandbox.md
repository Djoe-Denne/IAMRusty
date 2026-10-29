# Publication vérifiée ; la CI native échoue sur la preuve de sandbox

## Objectif et état

Les livrables durables sont publiés sur master et la référence distante a été vérifiée. Le workflow Linux natif s'est lancé normalement par ce push, sans dispatch manuel. Il réussit les tests, l'acquisition épinglée, les versions des outils et les deux inspections de scénarios.

L'étape Chromium échoue : la sandbox, le confinement de sécurité du navigateur, n'a pas pu être prouvée. Le résultat est INCONCLUSIVE avec exit2, cleanup des enfants PASS ; aucun artefact n'est publié. Aucun PASS global n'est annoncé. Le refus est conservé, pas contourné.

## Investigation bornée

Le diagnostic public actuel ne distingue pas un refus de politique du runner d'une autre absence de témoin sandbox. Une restriction de namespaces/AppArmor est une hypothèse plausible, pas une cause mesurée. L'auteur va ajouter un diagnostic fermé fondé sur les sorties déjà capturées, avec tests de confidentialité. Aucun log brut, nouvelle infrastructure ou affaiblissement système.

Cette investigation reste utile et autorisée malgré l'acceptance navigateur manquante. HTTP reste indépendamment sans cible publique conforme. La clôture et le nettoyage natif ne sont pas terminés : les worktrees de propositions sont conservés jusqu'à disposition vérifiée.

## Preuves

- Commit publié `919f1bde047ebff0ca50c6edc6d77f8453ce938b`, `refs/heads/master` vérifiée par ls-remote après push exit0.
- [Run natif k6](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40963859987), conclusion failure ; tests/acquisition/versions-inspect success, browser failure, publication skipped.
- Étape browser : SANDBOX_UNPROVEN, Chromium INCONCLUSIVE, cleanup PASS, aucun résultat HTTP.
- Watch natif terminé exit1, pas de polling manuel ; log privé `.tmp/k6-ci-watch-40963859987.log`.
- Source officielle k6 v2.3.0 browser_type.go:439–478 relue : pas de flag sandbox interdit par défaut dans ce bloc. Aucun navigateur WSL lancé avec succès et aucun paquet installé.
