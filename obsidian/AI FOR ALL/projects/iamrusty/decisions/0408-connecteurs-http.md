---
title: "ADR-0408 — connecteurs IdP en services HTTP (Accepted)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0408-connecteurs-idp-services-http.md
summary: >-
  Canon : docs/adr/0408. Accepted / Implemented. GitHub Connect /
  GitLab Connect = standalones compose. Registry redirect_uris[]. Pas de
  factory in-process. Pas de nest monolith v1.
created: 2026-09-20T16:15:00Z
updated: 2026-09-22T12:49:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0408 — connecteurs HTTP indépendants

Canon : `docs/adr/0408-connecteurs-idp-services-http.md`. Hub : [[projects/iamrusty/decisions/index]].

## Statut : Accepted (2026-09-20)

Réalité **Implemented**. Remplace **comme cible** `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` et [[projects/iamrusty/skills/extending-iamrusty-with-oauth-providers]].

## Paquet figé (cible)

1. Services `GitHubConnect` / `GitLabConnect` + crate `idp-connect-contract` (feature `server` pour le HTTP S2S partagé).
2. Registry IAM `[[idp.connectors]]` : slug, base URL, HMAC, **`redirect_uris[]`** (callback **et** relink-callback — pas un `redirect_uri` unique).
3. Compose only (ports 8085/8446, 8086/8447). **Pas** de nest `oodhive-monolith` v1.
4. Checklist nouveau-service : **sans** OpenFGA, events, DB, bearer user.

Hive `ExternalProviderClient` (sync org) = futur consommateur d’un contrat étendu, hors slice.
