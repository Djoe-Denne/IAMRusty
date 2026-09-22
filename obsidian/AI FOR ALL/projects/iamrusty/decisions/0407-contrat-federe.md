---
title: "ADR-0407 — contrat d’authentification fédérée vendor-neutral (Accepted)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0407-contrat-authn-federee-vendor-neutral.md
summary: >-
  Canon : docs/adr/0407. Accepted / Implemented. IAM IdP plateforme ;
  Connect = federated authenticators. Crate idp-connect-contract (feature
  server) + HTTP JSON. Enum GH/GL = adapter v1 (0407 §8).
created: 2026-09-20T16:15:00Z
updated: 2026-09-22T12:49:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0407 — contrat fédéré vendor-neutral

Canon : `docs/adr/0407-contrat-authn-federee-vendor-neutral.md`. Hub : [[projects/iamrusty/decisions/index]].

## Statut : Accepted (2026-09-20)

Réalité **Implemented**. Décideur : Djoé Denne. Ne réécrit pas ADR-0400 ; en **remplit** le « Non décidé » catalogue providers. Leftover : enum `Provider` GH/GL = adapter v1 de route (0407 §8).

## Paquet figé (cible)

1. Crate `idp-connect-contract` : trait `FederatedOAuthClient`, DTOs `ProviderTokens` / `ProviderUserProfile`, helpers HMAC. Feature optionnelle `server` = middleware Axum + 3 handlers (`Arc<dyn FederatedOAuthClient>`).
2. Transport IAM↔connecteur = HTTP JSON (`/v1/authorize|token|profile`), pas gRPC, pas events.
3. `OAuthService` reste IAM. Subject = `(provider_id, provider_user_id)`.
4. Enum `Provider` GH/GL = adapter v1, pas le contrat.

Suite : [[projects/iamrusty/decisions/0408-connecteurs-http]], [[projects/iamrusty/decisions/0409-confiance-oauth]], [[projects/iamrusty/decisions/0410-migration]].
