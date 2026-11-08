---
title: OAuthConnect local — HTTPS et distribution des secrets
category: concepts
tags: [oauth, security, ops, architecture, visibility/internal]
sources:
  - docs/missions/20261108-ops-oauthconnect.md
  - docs/history/ops-oauthconnect-20261108/README.md
  - docs/history/ops-oauthconnect-20261108/infra-review.md
  - ops/deploy/local_full_oauthconnect.py
  - ops/deploy/apps/overlays/local-full/oauth-connect/connect.toml
  - ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py
  - ops/deploy/apps/overlays/local-full/helpers/seed-openbao.sh
summary: >-
  Le profil local GitHub+simulated aligne HTTPS, IAM et HMAC par instance ; OpenBao distribue les secrets au boot. Livraison et revue hors ligne, pas déploiement prouvé.
provenance:
  extracted: 0.97
  inferred: 0.03
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# OAuthConnect local : HTTPS et secrets

Le profil ops **`--oauth-connect`** prépare le vrai service
[[projects/oauthconnect/oauthconnect|OAuthConnect]] avec deux instances OAuth :
`github` et `simulated`. Il est distinct des flags legacy `--github-openbao` et
`--oauth-proof` ; les combiner est refusé. Il n'est ni un retrait des anciens
services ni une bascule automatique d'un environnement installé.

## Une entrée HTTPS, plusieurs frontières

L'origine publique du nouveau profil est **`https://127.0.0.1:18080`**. Envoy
présente sa feuille existante portant le SAN IP. Le navigateur n'a pas à fournir
de certificat client ; les transport_socket mTLS upstream restent inchangés.
Le port hôte n'indique pas le protocole à lui seul : 18080 est HTTPS dans ce
profil et HTTP dans les profils historiques.

Les callbacks exacts GitHub et simulated, `jwt.public_base_url`, l'issuer IAM et
ses consommateurs sont alignés. Le HTTPS de l'origine sélectionne la politique
cookie sécurisée IAM. La CA doit être admise explicitement par le navigateur et
le callback HTTPS accepté par l'application GitHub enregistrée : aucune de ces
opérations n'a été exécutée. Voir
[[projects/oauthconnect/concepts/tls-hmac-and-signatures]].

GitHub conserve `verified`, `user:email`, `/user` et l'email primaire de
`/user/emails`, sans fallback email public. L'instance `simulated` conserve
PKCE S256 et un `email_verified` booléen ; seul son transport HTTP local porte
`isolated_test`. Son autorisation à `http://127.0.0.1:18081/oauth/authorize`
exigera le tunnel loopback ancré du futur scénario. Le déployeur ne l'ouvre pas.
Ce fournisseur ne simule pas Discovery/JWKS/ID tokens OIDC signés.

## Le coffre, pas une injection directe

Les chemins KVv2 GitHub **`github-connect/oauth`** et **`github-connect/iam`**
restent les références existantes. Le fournisseur simulé utilise les chemins
**`oauth-connect/simulated/oauth`** et **`oauth-connect/simulated/iam`**.
Les client secrets et les deux HMAC sont dédiés et distincts.

- **Connect** lit les quatre documents avec une identité Kubernetes projetée,
  rôle/audience dédiés et Agent produisant un token-file ; résolution au boot.
- **IAM** lit seulement les deux HMAC, jamais les client secrets vendor.
- **Simulateur** lit seulement son client secret OAuth, pas celui de GitHub ni
  les HMAC IAM.
- **Seed** possède les droits exacts lecture/création et metadata nécessaires
  au bootstrap ; aucun rôle applicatif ne gagne une policy wildcard.

Les configurations publiques ne portent que références et markers. Les snapshots
IAM/simulateur et le token-file vivent dans des volumes Memory. L'entrée privée
du seed est toutefois un Secret Kubernetes lié au Job par UID/TTL : **ce n'est
pas une garantie RAM-only dans etcd**.

## CAS, redémarrage et epoch

Le seed vérifie les quatre préimages avant d'écrire la première. Valeur identique
réutilisée, absence créée par CAS0, conflit ou suppression logique refusés. Une
perte d'ACK ou un échec partiel ne déclenche ni écrasement ni effacement de ce qui
a déjà pu être commité.

Le coffre local est volatil. `local-up` conserve les références aux **deux**
fichiers privés en lecture seule, sans empreinte des valeurs secrètes. Une epoch
nouvelle exige des sources inchangées, un seed contrôlé, puis le restart des
seuls Lazaret, IAM, OAuthConnect et simulateur sous contrôle UID/resourceVersion.
Une rotation n'est pas un effet secondaire autorisé d'une reprise.

## Preuve acquise et activation restante

La livraison `f78108b`, clôturée en `1866517` sur `origin/master`, dispose de tests
offline config/mappings, rendu, PKI synthétique, seed/reprise et revue infra
indépendante PASS statique. Le port 8080 du dual-bind n'est pas exposé par le
Service ; la durée des nouvelles feuilles suit les 90 jours locaux existants.

Build Linux, coffre/Agent réels, handshake navigateur, GitHub réel et chaîne E2E
restent **non exécutés**. Les preuves n'autorisent donc pas à présenter le profil
comme opérationnel live. ADR0413 demeure Accepted/Partial.

- [[projects/aiforall/skills/deploying-oauthconnect-local]] — procédure et gates.
- [[projects/aiforall/concepts/runtime-validation-separation]] — préparation,
  déploiement et validation ne sont pas la même action.
- [[projects/aiforall/references/session-ops-oauthconnect-20261108]] — décisions,
  résultats et revue du sous-agent.
