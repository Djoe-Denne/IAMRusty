---
title: E2E Ubuntu — Allure et Pages sans faux vert
category: concepts
tags: [testing, ops, security]
sources:
  - docs/missions/20261113-k6-auto-reporting-results.md
  - docs/local/audits/ci-cd/2026-11-13-k6-auto-reporting.md
  - ops/tests/k6/ci/README.md
  - docs/history/ci-k6-reporting-20261113/agents.md
summary: >-
  CI trusted push/master, bundles du même run et Allure public : 34 checks plus contrat parent, zéro raw/timing inventé ; succès E2E distinct de la CI globale et du LCOV Sonar.
created: 2026-11-13
updated: 2026-11-13
---

# Reporting fonctionnel, pas couverture de code

Le workflow réutilisable E2E dépend de **build**, pas des couvertures UT/IT.
Sur push/master trusted il s'exécute en parallèle ; Sonar reste sur sa voie LCOV.
PR/forks ne déclenchent pas cette pile. Le mode manuel saved-images reste séparé.

## Chaîne de confiance

Le consumer accepte les trois bundles non instrumentés de son run/attempt/SHA,
avec contrôles API, digest, provenance et ELF. Il empaquette les binaires déjà
construits, sans second build applicatif. Les probes ont une fermeture indépendante
par reçus avant admission des images et une garde `always()`.

Les deux alias Clippy déclaratifs connus du repo sont admis sans modifier les
builtins build/metadata ; flags LLVM, wrappers, profiles, env/target et alias
shell/home restent refusés. Cette correction lève le blocage du premier run,
elle n'assouplit pas la preuve « binaires non instrumentés ».

## Ce que compte le badge

Les cinq phases portent **34 checks** : health 2, SAN 6, OAuth navigateur 26.
Le parent exige séparément source/harness corrects, résumés stricts, processus
exit 0/cleanup, deltas IAM 7/authz 4/provider 3/PKCE 3/users 1/outbox 0/SAN 1,
restauration TLS et témoins sandbox recevables, zéro ressource possédée active.

Allure a donc **35 résultats : 34 checks + un contrat parent**. Un check absent
est non prouvé/broken. Un contrat parent KO rend le badge rouge même avec 34/34.
Aucun dénominateur de couverture produit exhaustive n'est fourni par ce compteur.

## Publication séparée

Pages reçoit seulement des projections fixes expurgées. Générateur Allure 2.46.2
épinglé par URL/taille/SHA256, extraction sans liens/traversée. Si la génération
HTML échoue, pas de rapport complet publié. Campagne rouge publiable avec état
rouge honnête ; les droits pages-write/id-token appartiennent au job de
publication, pas au navigateur. SHA master et attempt courant gardent le site
contre une publication obsolète. Aucun attachment brut, timing par check ou
historique inter-runs inventé.

## Preuve datée

[Run 43473245852](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43473245852),
source `741d0435` : E2E et couvertures démarrent à 17:33:57 UTC ; Pages publié
à 17:40:47 avant fin des couvertures. H=T=packaging, SDK `93b890ff`. Vérification
indépendante : 14/14 empreintes de campagne identiques, bundle public 111 entrées,
35 test-result et zéro attachment ; scan borné sans token détecté. Ressources
possédées probes/nœud/browser/volume fermées, owned_running 0.

- [Accueil public](https://djoe-denne.github.io/IAMRusty/).
- [Allure](https://djoe-denne.github.io/IAMRusty/allure/).
- [Statut et source du dernier rapport](https://djoe-denne.github.io/IAMRusty/status.json).

Ces adresses servent le **dernier rapport publié**, pas nécessairement ce run
historique ni le HEAD instantané. La CI globale de ce run reste rouge sur une
IT Manifesto, non diagnostiquée dans ce lot. Les badges Sonar montrent ses vraies
métriques ; aucun custom measure E2E fictif ni couverture 90 % revendiquée.

Contexte : [[projects/aiforall/references/session-ci-k6-reporting-20261113]],
[[projects/aiforall/concepts/k6-functional-linux]] et
[[projects/aiforall/concepts/functional-catalogue-and-test-evidence]].
