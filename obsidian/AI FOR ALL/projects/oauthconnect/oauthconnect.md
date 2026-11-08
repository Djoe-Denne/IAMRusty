---
title: >-
  OAuthConnect — authenticator OAuth et OIDC multi-instances
category: project
tags: [oauth, authentication, architecture, security, visibility/internal]
sources:
  - docs/adr/0413-oauthconnect-multi-instances-profils.md
  - docs/contracts/oauthconnect-multi-instances.md
  - docs/contracts/oauthconnect-oidc-service-boundary.md
  - docs/missions/20261107-oauthconnect-oidc-service.md
  - docs/missions/20261108-iam-oauthconnect-v2-results.md
summary: >-
  Connect authentifie l'identité externe et rend tokens+profil ; IAM conserve les comptes. Profils OAuth/OIDC, confiance HMAC/TLS et clés publiques sont séparés.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# OAuthConnect

OAuthConnect est le service HTTP d'authentification fédérée configuré par instances de la plateforme AIForAll. Son travail est de fournir à [[projects/iamrusty/iamrusty|IAM]] un résultat normalisé **tokens+identité**, sans devenir le propriétaire d'un compte humain, d'un lien IAM ou d'une session plateforme.

## Le modèle mental

Un **service** est un processus/binaire pouvant avoir plusieurs replicas. Une **instance** est une configuration d'autorité logique stable : slug, endpoints/issuer, credentials et politique de confiance. Un **profil** décrit le protocole de cette instance : `oauth` ou `oidc`. Ces mots ne sont pas interchangeables : `github` est un exemple de slug, pas un troisième profil ; un pod n'est pas une nouvelle autorité.

Un fournisseur compatible devient une entrée de configuration, pas un nouveau client vendor dans IAM ou une copie de service. Un besoin hors des deux profils acceptés exige un nouveau cadrage ; le mapping déclaratif n'est pas un langage pour scripts métier arbitraires.

## Deux opérations et deux frontières

Les routes S2S sont sous `/oauth-connect/{instance}/v2/authorize` et `/oauth-connect/{instance}/v2/complete`. Le PATH complet et le corps exact sont authentifiés par le HMAC de l'instance ; le navigateur ne les appelle pas directement.

- **Authorize** produit l'URL d'autorisation vendor avec le contexte demandé par IAM : state, callback, nonce v2 et challenge PKCE selon le profil.
- **Complete** échange le code, établit l'identité selon le profil puis retourne uniquement `ProviderAuthentication {tokens, profile}`. Il ne retourne ni ID token, ni JWKS, ni contexte opaque permettant d'appeler plus tard un service d'actions déléguées.

Le résultat groupé évite qu'IAM traite un succès token comme un succès d'identité avant validation. ^[inferred] Les appels HTTP nécessaires à cette validation restent internes au profil ; «deux opérations S2S» ne veut pas dire «deux requêtes HTTP vendor au total».

## OAuth et OIDC ne fournissent pas la même preuve

Le profil **OAuth** échange le code et interroge les API d'identité configurées, puis projette un profil minimal. L'instance GitHub utilise notamment la sélection d'email primaire, sans ancien fallback public implicite. Ce profil n'invente pas d'ID token/JWKS d'identité.

Le profil **OIDC** est un client/Relying Party. Il utilise Discovery, l'issuer configuré et le JWKS public pour valider l'ID token, puis UserInfo selon la politique. La clé privée de signature reste chez le fournisseur. Voir [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]].

Dans les deux profils, TLS protège les échanges et les endpoints/configurations doivent rester admis. Un token signé valide ne suffit pas si issuer, audience, temps ou nonce ne correspondent pas. Voir [[projects/oauthconnect/concepts/tls-hmac-and-signatures]].

## Stateless ne veut pas dire «aucun octet en RAM»

Connect n'a pas de cache métier authorize→complete ; le nonce attendu vient de la transaction IAM durable. Un replica reconstruit avec le même catalogue peut traiter la complétion sans avoir reçu le START.

Il possède cependant un cache technique de **clés publiques JWKS** en RAM, par client d'instance et par processus, pour la vérification OIDC. Ce cache n'est ni la clé privée fournisseur, ni la base des comptes, ni une session navigateur. Le renouveler n'est pas faire tourner une clé privée.

## Composition et secrets

Le catalogue est validé et construit complètement avant écoute : pas d'instance manquante silencieusement, pas de configuration partielle acceptée parce qu'une autre instance fonctionne. Secrets client OAuth et HMAC sont résolus au boot par les mécanismes SDK approuvés, OpenBao/direct selon configuration explicite ; aucune clé manquante ne provoque un fallback implicite.

La couture AppState SDK HMAC-only emploie un JWKS inline vide : elle ne crée pas d'autorité JWT utilisateur ni de refresh réseau IAM. Un Bearer RS256 bien formé ne remplace pas l'authentification HMAC des routes Connect.

## État qualifié et gates restants

Le lot Connect a exécuté295tests regroupés, dont jeux OIDC signés, routage HMAC, catalogue mixte et vrai binaire natif Windows. Le raccord IAM a ensuite exécuté325unités+80IT sur son périmètre retenu. Ces nombres ne sont pas additionnés pour prétendre qualifier une chaîne de bout en bout non exécutée.

ADR0413 reste Accepted/Partial. La chaîne intégrée IAM→Connect réel→vendor signé est un gap explicite, les E2E GitHub/GitLab et la comparaison réelle `sub` GitLab/ancien ID sont externes, aucune bascule ou suppression legacy n'est faite. Les schémas pédagogiques décrivent des responsabilités et preuves datées, pas un inventaire live de certificats ou pods.

## Parcours de lecture

- [[projects/iamrusty/decisions/0413-oauthconnect-multi-instances]] — cible et supersession partielle.
- [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]] — nonce, PKCE, commit et capacité IAM.
- [[projects/iamrusty/concepts/oauth-provider-linking]] — décisions de compte après résultat groupé.
- [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]] — propriétaire des clés et refresh.
- [[projects/oauthconnect/concepts/tls-hmac-and-signatures]] — ce que protège chaque couche.
- [[projects/aiforall/references/session-oauthconnect-iam-20261108]] — provenance de la session et contributions des agents.
- [Guide d'ajout d'instance](../../../../services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md) et [preuve du service](../../../../docs/missions/20261107-oauthconnect-oidc-service.md).
