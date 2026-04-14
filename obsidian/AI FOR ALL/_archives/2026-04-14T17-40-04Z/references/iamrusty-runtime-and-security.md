---
title: >-
  IAMRusty Runtime and Security Guides
category: references
tags: [reference, configuration, security, visibility/internal]
sources:
  - IAMRusty/docs/DATABASE_CONFIGURATION.md
  - IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - IAMRusty/docs/OAUTH_SECURITY_GUIDE.md
summary: >-
  Source summary for IAMRusty configuration, JWT key management, and OAuth hardening practices.
provenance:
  extracted: 0.92
  inferred: 0.05
  ambiguous: 0.03
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:03:47.5107188Z
---

# IAMRusty Runtime and Security Guides

These sources expand the operational side of `[[projects/iamrusty/iamrusty]]`, especially around `[[concepts/structured-service-configuration]]`, `[[concepts/jwt-secret-storage-abstraction]]`, and `[[concepts/oauth-state-and-csrf-protection]]`.

## Key Ideas

- Database configuration is typed, environment-aware, and supports random test ports plus per-process caching.
- JWT handling separates secret storage from token encoding so the runtime can switch between HMAC, RSA PEM files, and future secret managers.
- OAuth hardening relies on encoded state with nonce, timestamp, and operation context, plus exact redirect URI validation.
- Link-provider flows require authenticated context and explicit conflict detection to avoid cross-account takeover.
- The docs also emphasize operational security such as HTTPS, secret rotation, JWKS exposure, and security-event monitoring.

## Open Questions

- Vault, GCP, and AWS secret backends are described as future extensions rather than documented active integrations. ^[ambiguous]
- Downstream public-key distribution is described architecturally, but the exact production rollout across services is not captured in this batch.

## Sources

- [[concepts/structured-service-configuration]] — Typed config and env override model
- [[concepts/jwt-secret-storage-abstraction]] — Secret backend abstraction for tokens
- [[concepts/oauth-state-and-csrf-protection]] — OAuth state, CSRF, and callback hardening
