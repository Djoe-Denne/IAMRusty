---
title: Session ops OAuthConnect — GitHub, simulated et HTTPS
category: references
tags: [oauth, ops, security, testing, visibility/internal]
sources:
  - docs/history/ops-oauthconnect-20261108/README.md
  - docs/history/ops-oauthconnect-20261108/infra-review.md
  - docs/history/ops-oauthconnect-20261108/coverage.json
  - docs/missions/20261108-ops-oauthconnect.md
summary: >-
  Session et reviewer infra distillés : profil GitHub+simulated HTTPS publié, secrets OpenBao conservés, preuves offline et limites live. Deux sessions bornées, pas de transcripts bruts.
provenance:
  extracted: 0.98
  inferred: 0.02
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# Session ops OAuthConnect du 8 novembre

Le lot adapte les ops à [[projects/oauthconnect/oauthconnect|OAuthConnect]],
avec les identifiants GitHub existants et une instance `simulated` destinée à un
futur E2E. Il ne poursuit pas automatiquement les anciens lots de cutover ou de
qualification fournisseurs.

## Décisions et correction de direction

L'utilisateur choisit OAuth simulé, pas OIDC signé. La distribution de secrets
OpenBao reste obligatoire ; deux HMAC distincts et références GitHub conservées.
Le test de la configuration réelle refuse les callbacks HTTP sous `verified`.
Plutôt qu'approuver une exception loopback dans le validateur, l'utilisateur
demande **« Ne serait-il pas plus simple d'avoir un HTTPS ? »**.

Le nouveau profil passe donc en HTTPS public sur 127.0.0.1:18080, aligne callbacks,
cookie/issuer IAM et consommateurs, et conserve le mTLS upstream. Ce changement
ne s'applique pas silencieusement aux profils legacy HTTP. La technique et la
procédure sont intégrées dans
[[projects/oauthconnect/concepts/local-ops-and-secret-distribution]] et
[[projects/aiforall/skills/deploying-oauthconnect-local]].

## Preuves et pièges conservés

- Première configuration rouge ⇒ HTTPS vert, sans assouplir les attentes ni la
  vérification TLS GitHub.
- Suite Python complète : 215 tests, 213 réussites/2 ignorés ; après deux nouveaux
  oracles, 18/18 OAuthConnect et 19/19 OpenBao, dont extension PKI native synthétique.
  Les skips sont cache Cargo et registre Docker opt-in, pas une réussite live.
- Rust : 39 tests config verts, puis 3 tests ops de la configuration/mappings réels.
  Un wrapper PowerShell avait rendu 1 malgré libtest vert ; décodage UTF-16LE et
  commande native finale séparent le problème d'enveloppe du résultat de test.
- Clippy/fmt ciblés et hook fmt normal passent après correction des nouveaux
  tests. Avertissement SDK préexistant conservé ; sous-module inchangé.

Ne pas additionner ces nombres à ceux des anciens lots IAM/Connect pour annoncer
une chaîne E2E qui n'a pas été exécutée.

## Contribution du sous-agent

`infra-verifier` a relu indépendamment le diff contre `8575620c`, en lecture seule,
sans Cargo/runtime/secret opérateur. Conclusion : **PASS statique/offline**,
live **INCONCLUSIVE**. Contrôles : origine et TLS, registry/HMAC, SA/audiences et
droits OpenBao, seed/reprise CAS/UID, PKI, images/provenance et legacy.

F1 faible latent reste explicite : disparition future de la route relink GitHub
source ⇒ `StopIteration` au rendu. La route actuelle existe et est testée.
F2/F3 informatifs : 8080 non exposé et feuilles de 90 jours selon la politique existante.
Le verdict n'efface pas ces observations et ne prouve aucun handshake réel.

## Couverture native et publication

Racine `ses_e449ec10affeZ5b0fUHSKL1PZI`, unique enfant direct
`ses_e447e48acfferuDDDS3orLXEC6`. Frontière fixe :
**2026-11-08T14:32:54.690Z**, demande d'ingestion. Pages API épuisées : 3+2,
inventaire enfants : 2 pages. 175 messages examinés, 30 messages/parties visibles
projetés ; 198 parties reasoning et 262 parties outils exclues. Les choix de
formulaire sont des décisions observées pendant la session, pas des exports de
messages `user`. Projections privées non publiées ; aucun export de l'ancien
lot de 37 sessions, déjà couvert par
[[projects/aiforall/references/session-oauthconnect-iam-20261108]].

Implémentation `f78108b`, clôture `1866517`, `origin/master` vérifiée. À cette
clôture : arbre propre, seul master/checkout, SDK inchangé, aucun runtime créé,
aucune branche/worktree de mission, aucune équipe à archiver.

## Reste à qualifier

Build Linux, seed/Agent réels, certificat admis par le navigateur, callback
GitHub enregistré, connexion fournisseur, tunnel simulated et chaîne E2E.
**Non exécutés**, pas en attente automatique de lancement. ADR0413 demeure
Accepted/Partial. Voir [[projects/aiforall/concepts/runtime-validation-separation]].

Sources durables : `docs/history/ops-oauthconnect-20261108/` et
`docs/missions/20261108-ops-oauthconnect.md` ; aucun transcript brut ou secret
n'est nécessaire pour relire les décisions et leurs limites.
