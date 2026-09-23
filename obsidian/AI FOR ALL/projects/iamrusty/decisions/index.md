---
title: "IAMRusty — décisions vivantes (IdP fédérés)"
category: decisions
tags: [architecture, iam, oauth, visibility/internal]
status: accepted
feature_status: implemented
summary: >-
  Hub des ADR Accepted 0407–0411 (IAM-IdP). Réalité Implemented :
  connecteurs HTTP HMAC, Provider = slug registry, plus d’enum GH/GL.
  Canon git : docs/adr/. Distinct de 0100–0502 et Apparatus 0001–0008.
created: 2026-09-20T16:15:00Z
updated: 2026-09-22T15:10:00Z
sources:
  - docs/adr/README.md
  - docs/adr/0407-contrat-authn-federee-vendor-neutral.md
  - docs/adr/0408-connecteurs-idp-services-http.md
  - docs/adr/0409-confiance-callback-oauth-idp-connect.md
  - docs/adr/0410-migration-iam-connecteurs-idp.md
  - docs/adr/0411-idp-provider-slug-registry-fail-closed.md
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# IAMRusty — décisions vivantes

Canon : `docs/adr/0407`–`0411`. Jalon **IAM-IdP** (hors Apparatus). Statut **Accepted**. Réalité **Implemented** : IAM parle aux vendors via Connect HMAC S2S ; `Provider` = slug registry (0411). Pas de HuggingFaceConnect, pas de nest monolith.

Ne pas confondre avec [[projects/aiforall/decisions/index|vague 2 rétroactive]] (`0100`–`0502`) ni [[projects/manifesto/decisions/index|vague 1 Apparatus]] (`0001`–`0008`).

| ADR | Décision | Note |
|---|---|---|
| [[projects/iamrusty/decisions/0407-contrat-federe]] | Contrat vendor-neutral | complète le trou catalogue de [[projects/aiforall/decisions/0400-services-runtime]] / ADR-0400 |
| [[projects/iamrusty/decisions/0408-connecteurs-http]] | Services Connect, pas factory in-process | superède la *cible* du skill d’extension IAM |
| [[projects/iamrusty/decisions/0409-confiance-oauth]] | Callback/CSRF IAM ; secrets vendor sur le connecteur | |
| [[projects/iamrusty/decisions/0410-migration]] | Extraction incrémentale, UX inchangée | |
| [[projects/iamrusty/decisions/0411-slug-registry]] | Slug typé + registry boot fail-closed | plus d’enum GH/GL ; hot-load non |

## Related

- [[projects/iamrusty/iamrusty]]
- [[projects/iamrusty/concepts/oauth-provider-linking]]
- [[projects/iamrusty/concepts/oauth-state-and-csrf-protection]]
- [[projects/iamrusty/skills/extending-iamrusty-with-oauth-providers]]
- [[journal/2026-09-20]]
- [[journal/2026-09-22]]
