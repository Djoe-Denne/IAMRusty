---
title: >-
  OAuth State and CSRF Protection — trois contextes distincts
category: concepts
tags: [security, oauth, csrf, iam, visibility/internal]
sources:
  - docs/adr/0409-confiance-callback-oauth-idp-connect.md
  - docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md
  - docs/contracts/iam-oauthconnect-v2.md
  - services/IAMRusty/http/src/oauth_browser.rs
  - services/IAMRusty/domain/src/entity/oauth_transaction.rs
summary: >-
  State signé, liaison au navigateur et nonce d'authentification sont distincts ; le writer durable, non un cache mémoire, consomme et purge avant complete.
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.0
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-11-08
---

# OAuth State and CSRF Protection

## Ne pas appeler tous ces éléments «le nonce»

- Le **state signé** porte et protège le contexte OAuth interprété par IAM : provider, intention, expiration et liaison à la transaction. Il traverse le navigateur et le fournisseur, puis revient au callback.
- Le **contexte navigateur/cookie** lie le callback au navigateur ayant initié le START. Un state signé seul n'établit pas cette propriété.
- Le **nonce d'authentification v2** est un tirage indépendant32octets OsRng, base64url canonique sans padding, persisté dans IAM. OAuthConnect le transmet au fournisseur OIDC puis vérifie sa présence/égalité dans l'ID token. Ce n'est ni le nonce du state ni le cookie.
- Le **verifier PKCE** est encore un autre secret : le START n'expose que son challenge S256. Le verifier original ne part dans complete qu'après consommation commitée.

Ces contrôles couvrent des frontières différentes ; les réutiliser comme une seule valeur rendrait leur séparation de responsabilités illusoire. ^[inferred]

## Ce qui rend le callback consommable une seule fois

La photographie de septembre décrivant `USED_NONCES` process-local n'est plus la cible actuelle. ADR0412 est **Accepted / Implemented** pour les preuves locales qualifiées ; elle impose une transaction persistante liée au navigateur et partagée par les replicas via le writer.

Le record garde les empreintes du state et du contexte navigateur, provider canonique, opération, UUID cible éventuel, redirect exact, expiration et état de consommation. Le delta v2 ajoute le nonce d'authentification nullable pour compatibilité des lignes anciennes. Le record garde aussi le verifier PKCE quand la politique le requiert.

Le writer réalise les prédicats de binding et un `SELECT … FOR UPDATE`, charge les secrets originaux, marque consommé et efface `authentication_nonce`/`pkce_verifier` dans **la même transaction**. Le commit précède le retour de capacité et l'I/O Connect. Une course concurrente a un seul gagnant ; un replica n'a pas besoin du cache mémoire d'un autre.

Un binding invalide ne brûle pas la transaction authentique ; une transaction expirée ne devient pas valide parce que son nonce est correct. Les expirations restent bornées, TTL maximal600s et cleanup borné des lignes selon le mécanisme existant.

## Après une erreur, on ne restaure pas les secrets

- Complete refusé, réponse partielle, erreur réseau : aucune mutation métier IAM, mais consommation déjà commitée ; replay refusé, nouveau START requis.
- Ligne legacy/corrompue avec bindings valides : le writer consomme/purge d'abord, puis le domaine refuse sans capacité utilisable. Aucun nonce n'est fabriqué pour la réparer.
- Échec writer/commit : pas de capacité, pas de complete. Une erreur d'environnement avant montage d'un test n'est pas un RED de ce comportement.

Les autorisations LINK/RELINK sont établies au START depuis le principal plateforme. Le callback transactionnel utilise la cible persistée ; il ne prend pas une nouvelle cible dans la query et n'exige pas de recréer l'autorité depuis un Bearer arbitraire. Voir [[projects/iamrusty/concepts/oauth-provider-linking]].

## Frontières qui ne sont pas couvertes par state

Le state ne remplace ni TLS, ni HMAC S2S, ni validation cryptographique OIDC. Le navigateur ne doit pas appeler directement les routes Connect HMAC ; le connecteur ne devient pas propriétaire des cookies, des UUID ou du rapprochement de comptes.

Le profil OAuth ne transmet pas un nonce OIDC à un vendor qui n'a pas ce protocole, mais IAM maintient le même contexte durable v2. Les règles exactes de profil et PKCE viennent du catalogue validé, pas d'une déduction sur le nom `github`/`gitlab`.

## Sources et navigation

- [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]] — chronologie complète START/consume/complete et portée de la capacité.
- [[projects/oauthconnect/concepts/tls-hmac-and-signatures]] — les différentes protections, sans interchangeabilité.
- [[projects/iamrusty/decisions/0413-oauthconnect-multi-instances]] — évolution minimale v2 et bornes.
- [ADR0412 courante](../../../../../docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md), [contrat IAM](../../../../../docs/contracts/iam-oauthconnect-v2.md), [preuves80IT](../../../../../docs/missions/20261108-iam-oauthconnect-v2-results.md).
