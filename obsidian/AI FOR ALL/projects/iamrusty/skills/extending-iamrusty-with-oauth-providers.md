---
title: >-
  Ajouter une instance fournisseur OAuth ou OIDC
category: skills
tags: [oauth, services, rust, security, visibility/internal]
sources:
  - services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md
  - docs/adr/0413-oauthconnect-multi-instances-profils.md
  - docs/contracts/oauthconnect-multi-instances.md
  - docs/contracts/iam-oauthconnect-v2.md
summary: >-
  Un fournisseur compatible devient une instance configurée d'OAuthConnect et une entrée IAM complète, sans copier un service ni ajouter un client vendor in-process.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-11-08
---

# Ajouter une instance fournisseur

> [!important] Supersession partielle, pas réécriture historique
> Les recettes anciennes «copier GitHubConnect en un nouveau service» correspondaient à0408/0411. ADR0413 **Accepted / Partial** change cette cible pour les profils compatibles : ajouter une instance au service [[projects/oauthconnect/oauthconnect|OAuthConnect]]. Les anciens services ne sont pas supprimés par le raccord local et la bascule reste à qualifier.

## Procédure de conception et configuration

1. Identifier l'autorité, son issuer pour OIDC et son profil OAuth ou OIDC. Un slug désigne une autorité logique stable, pas un pod ; ne pas modifier son issuer en place pour simuler une rotation.
2. Configurer dans OAuthConnect les endpoints/scopes/mapping et secrets vendor selon le profil accepté. Aucun script métier caché, nouvel interpréteur de claims ou fallback de profil n'est ajouté dans IAM.
3. Aligner le slug et la politique PKCE, les callbacks publics exacts, et une clé HMAC propre à l'instance. Client secret OAuth, HMAC S2S, clé privée fournisseur et clés JWT IAM ne sont pas la même matière.
4. Ajouter une entrée IAM **complète** `[[idp.connectors]]` avec base URL de l'instance, HMAC et allowlist callback/relink-callback. Le boot reste fail-closed ; aucun catalogue partiel ni hot-load opportuniste.
5. Configurer TLS vérifié/CA nécessaires pour les hops effectifs. Une exception claire locale/test ne constitue pas une politique production. Ne pas désactiver la validation des certificats pour faire passer un test.
6. Vérifier les preuves de protocole et de métier aux frontières existantes, puis traiter séparément migration additive IAM, drainage TTL, E2E fournisseur et déploiement. Aucun compte/lien/UUID n'est migré implicitement.

## Ce qui ne change pas dans IAM

`Provider` est un slug typé : lettres ASCII, case-fold minuscule,1–50caractères ; pas d'enum vendor. Le registry immuable fournit `Arc<dyn FederatedOAuthClient>` ; application et tests accèdent aux types v2 via la façade `iam_domain::port::service`. Aucun client vendor in-process, dépendance directe supplémentaire ou nest OAuthConnect dans le monolithe.

LOGIN/LINK/RELINK utilisent authorize/complete v2 et la transaction durable décrite dans [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]]. La clé d'identité est instance+ID opaque, pas l'email ni une conversion numérique.

## Vérification et limites de bascule

IAM simule son collaborateur Connect HTTP sortant ; Connect simule le vendor, avec jeux OIDC signés pour les tests cryptographiques. Un mock S2S IAM n'est pas cette validation de signature. Le gap de chaîne intégrée IAM→Connect réel→vendor ne doit pas conduire à créer un deuxième harness ou à inventer une preuve.

Pour GitLab, vérifier `sub` nouveau égal à l'ancien identifiant canonique du même compte réel **avant bascule**. En cas de différence, arrêter et cadrer l'autorité/linking ; ne pas réassocier automatiquement les comptes. L'ajout d'une instance dans les fichiers ne prouve ni fournisseur live ni rollout.

## Navigation

- [Guide versionné détaillé](../../../../../services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md).
- [[projects/iamrusty/decisions/0413-oauthconnect-multi-instances]] et [[projects/iamrusty/decisions/index]].
- [[projects/iamrusty/concepts/oauth-provider-linking]] — politiques des comptes et des emails.
- [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]] — clés publiques et coordination de rotation.
