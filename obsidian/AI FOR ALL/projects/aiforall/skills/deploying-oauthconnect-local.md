---
title: Préparer le profil local OAuthConnect GitHub et simulated
category: skills
tags: [oauth, ops, kubernetes, security, visibility/internal]
sources:
  - ops/deploy/README.md
  - ops/deploy/local_full_oauthconnect.py
  - ops/deploy/local_full_environment.py
  - ops/scripts/prepare-local-full-certs.py
  - docs/missions/20261108-ops-oauthconnect.md
  - docs/history/ops-oauthconnect-20261108/infra-review.md
summary: >-
  Préparer images, TLS, deux sources privées et reprise OpenBao du profil --oauth-connect ; contrôler CA/callback GitHub avant une activation et des preuves live séparées.
provenance:
  extracted: 0.98
  inferred: 0.02
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# Préparer OAuthConnect en local

Cette recette décrit un chemin livré et testé **hors ligne**, pas un déploiement
déjà qualifié. Le détail opérateur versionné est `ops/deploy/README.md`, section
« OAuthConnect : GitHub + fournisseur simulé, entrée HTTPS ».

## Avant toute activation

1. Choisir explicitement `--oauth-connect`, incompatible avec les deux profils
   legacy. Le service [[projects/oauthconnect/oauthconnect|OAuthConnect]] prépare
   `github` et `simulated`, sans importer une autorité GitLab sous un autre slug.
2. Vérifier le bail original et les UID du seul environnement local-full autorisé.
   Aucun apply global ne doit contourner une garde d'ownership/quiescence ni
   prétendre migrer un environnement existant.
3. Vérifier l'enregistrement GitHub existant pour les callbacks HTTPS exacts :
   `https://127.0.0.1:18080/iam/api/auth/github/callback` et
   `https://127.0.0.1:18080/iam/api/auth/github/relink-callback`. Le déployeur ne
   modifie pas l'application OAuth GitHub.
4. Prévoir la confiance explicite du navigateur envers `mesh/ca.crt`. N'utiliser
   ni `-k` ni une désactivation CA/SAN comme substitut.

## Préparer des entrées, pas des secrets en argument

Conserver le fichier GitHub privé en lecture seule et préparer un fichier simulé
distinct de même schéma JSON : `version=1`, `client_id`, `client_secret`,
`hmac_secret`. Le client simulé accepte 8–128 caractères `[A-Za-z0-9_-]`, un secret
de 16–256 caractères sans contrôle et un HMAC de 64 caractères hexadécimaux minuscules. Générer des valeurs
aléatoires dédiées et ne pas publier ces fichiers ; les quatre valeurs secrètes
doivent être distinctes.

Le runtime ne reçoit que les chemins `--github-input-file` et
`--simulated-input-file`. La distribution reste celle du coffre ; voir
[[projects/oauthconnect/concepts/local-ops-and-secret-distribution]]. Une source
modifiée exige une rotation explicite, pas une réparation automatique du seed.

## Ordre des outils

- **Rendu offline** : `deploy-local-full.py render --oauth-connect`. Aucune
  inspection ou mise en route Docker/Kind.
- **PKI explicite** : `prepare-local-full-certs.py --oauth-connect`, répertoire
  privé hors Git. Extension additive `extensions/openbao-oauthconnect/`, ancienne
  PKI préservée ; ensemble partiel refusé, pas réparé en place.
- **Image** : `build/images.py plan|build --oauth-connect`, puis consommation
  explicite du reçu par `local-full-images.py --oauth-connect`. La cible `oauth`
  exporte `oauth-connect-service`, sans migration ni réutilisation aveugle d'un
  vieux binaire. Pas de dev Cargo dans Docker ; ces builds sont ceux de livraison.
- **Déploiement** : bail, images immuables, certificats et deux fichiers privés
  explicites. Gates runtime/IT et restrictions de contexte restent applicables.
  Le README donne les commandes complètes ; aucune n'a été exécutée live dans
  la session source.
- **Reprise quotidienne** : enregistrer les deux sources au premier `local-up` ;
  nouvelle epoch OpenBao ⇒ reseed puis restart contrôlé des dépendants. Le
  refresh `--targets oauth` est image-only, pas une première installation.

## Qualifier séparément

Tester coffre réel, Agent, TLS public admis par le navigateur, HMAC/registry IAM,
puis parcours GitHub et simulé. Le scénario simulated doit ouvrir explicitement
le tunnel ancré loopback18081 ; il n'est pas lancé par le profil. Les anciens
scripts E2E HTTP/GitLab ne sont pas automatiquement compatibles ou qualifiés.

Une revue statique PASS ne ferme pas ces étapes. Voir
[[projects/aiforall/references/session-ops-oauthconnect-20261108]] et
[[projects/aiforall/concepts/runtime-validation-separation]].
