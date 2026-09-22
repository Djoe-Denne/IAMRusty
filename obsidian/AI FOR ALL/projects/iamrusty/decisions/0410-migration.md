---
title: "ADR-0410 — migration incrémentale vers les connecteurs (Accepted)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0410-migration-iam-connecteurs-idp.md
summary: >-
  Canon : docs/adr/0410. Accepted / Implemented. IAM HTTP HMAC only.
  Routes /api/auth/{provider} inchangées. IT 0200/0201. Pas de nest, pas Google.
created: 2026-09-20T16:15:00Z
updated: 2026-09-22T12:49:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0410 — migration IAM → connecteurs

Canon : `docs/adr/0410-migration-iam-connecteurs-idp.md`. Hub : [[projects/iamrusty/decisions/index]].

## Statut : Accepted (2026-09-20)

Réalité **Implemented** (S1–S6). IAM ne parle à GitHub/GitLab que via connecteurs HTTP HMAC. Volant in-process retiré.

IT : IAM WireMock **connecteur** ; connecteur WireMock **vendor**. Interdit de mocker le use-case IAM.
