---
title: JWKS et rotation des clés — fournisseur OIDC et IAM
category: concepts
tags: [oauth, authentication, security, architecture, visibility/internal]
sources:
  - docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md
  - docs/guides/iam-oauthconnect-securite-sources.md
  - services/OAuthConnect/infra/src/oidc/cache.rs
  - services/IAMRusty/infra/src/signing/rotate.rs
summary: >-
  JWKS publie le public pour vérifier les signatures. Le fournisseur tourne ses privés ; Connect renouvelle son cache. IAM possède un lifecycle indépendant via Transit.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# JWKS et rotation des clés

## Le problème : vérifier sans pouvoir signer

Dans le profil OIDC, le fournisseur signe un ID token avec sa **clé privée**. OAuthConnect vérifie cette signature avec la **clé publique** correspondante. Posséder cette dernière ne permet pas de produire une nouvelle signature valide. Le payload signé n'est pas chiffré : base64url est un encodage, pas de la confidentialité.

Une JWK représente une clé ; un JWKS contient un tableau `keys`. Le format JWK sait représenter du matériel privé, mais un endpoint **public de vérification** ne doit jamais le publier. Pour RSA, il expose notamment le module et l'exposant publics, pas les paramètres privés. `kid` est un identifiant de sélection, pas une preuve de confiance ni un secret.

Le JWKS ne décrit ni les utilisateurs ni leurs rôles. Ce n'est pas non plus une chaîne de certificats TLS : même si une JWK peut porter `x5c`, elle n'est pas le certificat du serveur HTTPS qui héberge le document.

## D'où vient la confiance ?

La configuration fixe l'issuer et les origines admises. Discovery fournit les métadonnées et le `jwks_uri` ; le client contrôle issuer, endpoints, transport et profils avant de les employer. Il ne suit pas une URL `jku`/`x5u` arbitraire trouvée dans un token.

Le résultat doit passer **signature et contexte** : algorithme RS256 admis localement, issuer exact, audience attendue, temps, nonce demandé et autres contrôles du profil. UserInfo éventuel doit rester lié au subject validé. Une signature mathématiquement valide pour un autre client ou issuer n'est pas une identité acceptée.

## Qui conserve et change quelle clé ?

- **Fournisseur OIDC** : garde le privé et signe les ID tokens ; son opérateur publie les publics et décide de leur rotation. GitLab.com relève du fournisseur SaaS ; une instance self-managed relève de ses opérateurs. Le coffre, calendrier et mécanisme réels n'ont pas été inspectés.
- **OAuthConnect** : garde seulement une vue publique en RAM, partagée entre clones d'un client d'instance mais distincte entre processus/replicas. Il ne génère pas le privé fournisseur et ne provoque pas sa rotation.
- **IAM** : signe ses propres access tokens plateforme au moyen d'un signer configuré. Son JWKS et son lifecycle ne sont pas le JWKS fournisseur consommé par Connect.
- **OAuth client secret / HMAC / TLS** : autres matières avec d'autres propriétaires ; voir [[projects/oauthconnect/concepts/tls-hmac-and-signatures]].

Le profil OAuth GitHub authentifie via les API configurées ; il n'invente pas un ID token ou un JWKS d'identité.

## Le cache Connect, concrètement

Fraîcheur du dernier succès **strictement inférieure à300s** ; tentative de refresh espacée d'au moins30s, enregistrée avant l'I/O. Un verrou/single-flight et un recontrôle du snapshot empêchent des requêtes concurrentes identiques d'engendrer une rafale de GET. Le client de validation est reconstruit avec le snapshot réellement retenu.

À l'intérieur de la fraîcheur, seul l'échec typé `NoMatchingKey` peut demander une acquisition réactive : **pas tout échec de signature ou de claim**. Réutiliser le même `kid` avec un nouveau matériau ne force pas un refresh anticipé ; il faut distinguer ce cas d'un nouveau `kid` absent. La nouvelle validation porte sur le même token, sans second échange du code.

Un refresh raté ne rajeunit pas le dernier succès. Une ancienne clé encore fraîche peut donc rester utilisable, tandis que le nouveau `kid` refusé ne devient pas accepté. À expiration, échec d'acquisition = refus, pas service stale illimité. Ces règles n'offrent ni révocation instantanée ni rotation garantie sans interruption.

**300s est un TTL de cache ;30s un cooldown réseau. Aucun n'est une fréquence de rotation de clé privée.** Redémarrer Connect vide une vue publique, pas le coffre du fournisseur. Changer d'issuer sous le même slug est un changement d'autorité, pas une rotation.

## Rotation OIDC habituelle : séquence à expliquer, pas opération exécutée

1. L'opérateur fournisseur prépare un nouveau privé et publie son public avec un nouveau `kid`.
2. Après la propagation jugée nécessaire, le fournisseur signe avec ce nouveau `kid`.
3. Les validateurs acquièrent le nouveau public selon leurs politiques de cache.
4. L'ancien public chevauche le nouveau suffisamment longtemps pour les tokens encore valides et les caches concernés.
5. Le fournisseur retire l'ancien public selon sa politique ; retrait public et destruction du privé ne sont pas synonymes.

Le code Connect ne prouve ni l'ordre réel ni le délai choisis par un fournisseur live.

## Rotation IAM : un autre circuit

Le chemin organisation inspecté est **Admin organisation → Hive → IAM interne → rotator → Transit**. Hive vérifie l'autorisation Admin et appelle le port de signer ; IAM orchestre le matériel et le registry writer. Cela établit un chemin produit, pas l'identité d'un opérateur ou une cadence en production.

La rotation livrée inspectée supporte Transit : nouvelle version épinglée, public récupéré et preuve du matériau avant admission. La capacité de **signer à distance** ne signifie pas capacité de **Rotate** : RemoteHttp/cloud retournent `ProviderNotSupported` dans cette primitive ; PEM ne fabrique pas de successeur.

Le registry prépublie durablement Pending ; la promotion vérifie révision/binding/ancien Active puis commite atomiquement nouveau Active et ancien Retiring avec la publication. Le helper n'impose pas à lui seul un délai de propagation entre prépublication et promotion. À l'émission, le codec vérifie le candidat et une fence writer finale d'époque : ne pas lui attribuer une lease distribuée gelant le provider pendant tous les awaits.

Retiring reste publiable tant que son horodatage admissible + **TTL JWT IAM configuré +60s** dépasse l'instant d'évaluation. Ce TTL n'est ni300s de Connect ni une constante universelle900s. Revoked est exclu. Le publisher HTTP utilise une publication writer matérialisée ; un succès vide valide est autoritatif, une panne ne devient pas artificiellement un succès vide.

La prescription de cache consommateur IAM (ADR0304/0308) est encore une troisième politique : ne pas lui appliquer automatiquement300/30. Aucun déploiement, calendrier Ops, coffre effectif ou endpoint plateforme de rotation public n'est qualifié par cette lecture de code.

## Lire plus bas

- [Guide détaillé](../../../../../docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md) et [preuves S01–S06/S11–S14](../../../../../docs/guides/iam-oauthconnect-securite-sources.md).
- [Atlas versionné](../../../../../docs/diagrams/README.md).
- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]] ; [[projects/iamrusty/concepts/jwt-secret-storage-abstraction]].
- [[projects/aiforall/references/session-oauthconnect-iam-20261108]] — provenance et limites historiques.
