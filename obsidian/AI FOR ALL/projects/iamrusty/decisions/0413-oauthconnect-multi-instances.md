---
title: >-
  ADR-0413 — OAuthConnect multi-instances et contexte IAM v2
category: decisions
tags: [architecture, iam, oauth, security, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0413-oauthconnect-multi-instances-profils.md
  - docs/contracts/oauthconnect-multi-instances.md
  - docs/contracts/oauthconnect-oidc-service-boundary.md
  - docs/contracts/iam-oauthconnect-v2.md
  - docs/missions/20261107-oauthconnect-oidc-service.md
  - docs/missions/20261108-iam-oauthconnect-v2-results.md
summary: >-
  Cible ratifiée : un service OAuthConnect, instances OAuth/OIDC et deux opérations v2 ; IAM garde comptes et nonce durable. Code local qualifié, cutover externe.
provenance:
  extracted: 0.98
  inferred: 0.02
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# ADR-0413 — OAuthConnect multi-instances

**Canon : ADR0413 Accepted / Partial.** Le wiki explique cette décision, il ne la ratifie pas et ne change pas son statut. L'accord utilisateur sur v2, nonce indépendant et différence GitHub «email primaire seul» est acquis le6novembre ; claims riches/actions déléguées restent différés.

## Choix structurants

- Un service HTTP stateless héberge un catalogue immuable d'instances. Le slug représente une autorité logique stable ; des replicas peuvent servir la même instance.
- Les profils `oauth` et `oidc` ne remplacent pas les slugs `github`/`gitlab`. Ils déclarent comment authentifier une identité, pas un nouveau compte IAM.
- Deux opérations S2S : `authorize` et `complete` v2. Complete retourne seulement tokens+profil normalisé ensemble ; pas d'ID token ou objet de claims riches exposé à IAM, pas de fallback v1.
- IAM conserve transactions persistantes, navigateur, comptes UUID, liens, politique email et stockage des tokens. Le nonce d'authentification est indépendant, durable avant START et effacé avec le verifier lors de la consommation commitée.
- Le connecteur ne devient pas une autorité JWT utilisateur : la couture AppState SDK HMAC-only utilise un JWKS inline vide, sans principal ou clé de secours.
- HMAC par instance, METHOD/PATH complet/TIMESTAMP/BODY, TLS vérifié et bornes restent les frontières de confiance ; le monolithe ne nest pas OAuthConnect.

## Supersession partielle, précisément bornée

La topologie «un nouveau fournisseur = nouveau service copié» de [[projects/iamrusty/decisions/0408-connecteurs-http]] et l'onboarding de [[projects/iamrusty/decisions/0411-slug-registry]] changent pour les profils compatibles. L'interface minimale d'authentification de [[projects/iamrusty/decisions/0407-contrat-federe]] évolue vers v2.

Le reste est conservé : frontière HTTP, slugs typés, fail-closed au boot, responsabilité IAM et absence de nest. Les statuts historiques Accepted/Implemented de ces ADR ne sont pas globalement changés en «obsolètes».

## Photographie vérifiée du8novembre

- Connect :295tests dans la campagne locale regroupant configuration/domaine/infra/HTTP/setup/service/contrat, jeux OIDC signés et vrai binaire Windows. Pas un fournisseur live.
- IAM :325unités+80IT sur11binaires, writer Postgres, upgrade réel, navigateur et TLS. Pas une qualification de toutes les suites IAM.
- Code/docs publiés sur master ; SDK/pin inchangé dans ces lots. La bascule de configuration réelle n'est pas effectuée.

**Pourquoi Partial persiste :** E2E GitHub/GitLab et égalité `sub` GitLab/ancien ID réels restent externes ; pas de déploiement/cutover ou retrait legacy. La preuve intégrée IAM→Connect réel→vendor signé n'est pas fermée par les mocks S2S IAM. Les limites de streaming/Clippy et de ressources retenues sont documentées, pas effacées par un total de tests.

## Navigation

- [[projects/oauthconnect/oauthconnect]] — le service et ses responsabilités.
- [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]] — mécanique transactionnelle.
- [[projects/aiforall/references/session-oauthconnect-iam-20261108]] — décisions, provenance de session et essais.
- [ADR canonique](../../../../../docs/adr/0413-oauthconnect-multi-instances-profils.md) et [contrat IAM](../../../../../docs/contracts/iam-oauthconnect-v2.md).
