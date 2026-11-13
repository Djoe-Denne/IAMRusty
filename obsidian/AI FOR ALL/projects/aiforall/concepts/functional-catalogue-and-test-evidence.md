---
title: Catalogue fonctionnel — exigences, assertions et résultats
category: concepts
tags: [testing, architecture]
sources:
  - docs/discovery/iam-oauth/README.md
  - docs/discovery/iam-oauth/catalogue.json
  - docs/history/ci-k6-reporting-20261113/README.md
summary: >-
  Un catalogue indépendant rend visibles les comportements non testés ; LLM propose avec sources, assertions caractérisent, CI actualise les résultats. Pilote IAM/OAuth limité à quatre fonctionnalités et treize critères.
created: 2026-11-13
updated: 2026-11-13
---

# Ne pas confondre cartographie et exécution

Un LLM peut proposer domaines, critères et correspondances à partir des routes,
contrats, code et assertions. Cette proposition reste révisable : un comportement
accidentel n'est pas automatiquement une exigence ratifiée. Des IDs stables
survivent aux renommages ; faits, déductions et correspondances partielles sont
explicités. Pas de reclassement silencieux à chaque campagne.

Allure regroupe les labels fournis (Epic → Feature → Story) ; il ne découvre pas
le métier et ne constitue pas à lui seul le catalogue des fonctionnalités.
Une fonctionnalité sans test peut disparaître d'un rapport : le catalogue doit
exister **indépendamment** des résultats. Rust/Cargo nécessite une correspondance
ou intégration explicite ; un tag k6 seul ne génère pas un résultat Allure.

## Pilote IAM/OAuth, pas inventaire IAM

Quatre fonctionnalités, treize critères avec références code/ADR/assertions.
Les correspondances sont proposées, pas un score global de couverture produit.
Le pilote a révélé deux oracles trop larges, caractérisés sans changer la logique :

- Q01 : LOGIN sans email = **401 / authentication_failed** ; erreur repository
  = **500 / login_failed**, mapping vérifié séparément.
- Q02 : complétion = **200** ; le check k6 ne tolère plus 201.

Jalon `a2c0f80` : deux unités et 17 IT PASS, pas de nouveau navigateur à cette
étape. La visualisation initiale est un support de lecture, pas une preuve de
rendu contrôlé ni une intégration Allure du catalogue entier.

Une ligne exécutée ne prouve pas une assertion et un refus TLS utile ne fait pas
nécessairement monter le coverage Rust. Distinguer critère prévu, test présent,
test exécuté et comportement réellement vérifié. Le refus d'instrumenter LLVM
dans ce lot conduit au reporting fonctionnel séparé, pas à un pourcentage inventé.

Voir [[projects/aiforall/concepts/e2e-allure-pages-reporting]],
[[projects/aiforall/concepts/qa-proportionate-proof]] et
[[projects/aiforall/references/session-ci-k6-reporting-20261113]].
