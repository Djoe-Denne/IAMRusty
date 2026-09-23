---
title: "ADR-0411 — slug typé et registry fail-closed (Accepted)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0411-idp-provider-slug-registry-fail-closed.md
summary: >-
  Canon : docs/adr/0411. Accepted / Implemented. Provider = slug lettres-only.
  Catalogue = [[idp.connectors]] au boot. 400 syntaxe / 422 hors registry.
  Pas de hot-load. Pas de HuggingFaceConnect.
created: 2026-09-22T15:10:00Z
updated: 2026-09-22T15:10:00Z
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
---

# ADR-0411 — slug registry fail-closed

Canon : `docs/adr/0411-idp-provider-slug-registry-fail-closed.md`. Hub : [[projects/iamrusty/decisions/index]].

## Statut : Accepted (2026-09-22)

Réalité **Implemented** (GitHub + GitLab existants). `Provider` n’est plus un enum. Onboarding N+1 = copie [[projects/iamrusty/skills/extending-iamrusty-with-oauth-providers|gabarit GitHubConnect]] + ligne registry + restart IAM. HMAC 0409 inchangé ([[projects/iamrusty/decisions/0409-confiance-oauth]]).
