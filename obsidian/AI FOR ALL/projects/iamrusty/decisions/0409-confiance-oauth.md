---
title: "ADR-0409 — callback IAM, secrets connecteur, HMAC (Accepted)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0409-confiance-callback-oauth-idp-connect.md
summary: >-
  Canon : docs/adr/0409. Accepted / Implemented. Navigateur → IAM.
  CSRF IAM (exp+HMAC). client_secret sur le connecteur. HMAC S2S PATH préfixé.
created: 2026-09-20T16:15:00Z
updated: 2026-09-22T12:49:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0409 — frontières de confiance OAuth

Canon : `docs/adr/0409-confiance-callback-oauth-idp-connect.md`. Hub : [[projects/iamrusty/decisions/index]].

## Statut : Accepted (2026-09-20)

Réalité **Implemented**. CSRF IAM (`oauth_state.rs` : `exp`, HMAC, replay) + registry `redirect_uris[]` + HMAC S2S vers les connecteurs.

Le concept [[projects/iamrusty/concepts/oauth-state-and-csrf-protection]] décrit `exp`+HMAC et `redirect_uris` registry (plus de 8081 hardcodé dans le handler).

## Paquet figé (cible)

1. Callbacks OAuth App = IAM public. Registry `redirect_uris[]` (callback + relink-callback). Connecteur S2S only.
2. `client_secret` vendor hors IAM. Linking reste IAM.
3. Trust v1 = HMAC (`METHOD\nPATH\nTIMESTAMP\nBODY`, PATH **avec** préfixe, fenêtre 30 s, compare `hmac`+`subtle`) + HTTPS mesh ; pas de bearer JWT utilisateur sur le connecteur.
4. IT IAM : WireMock connecteur `:3000` (pas vendor). Development compose IAM :8080.
