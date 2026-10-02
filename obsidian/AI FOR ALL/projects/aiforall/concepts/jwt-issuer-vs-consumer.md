---
title: >-
  JWT issuer versus consumer
category: concepts
tags: [iam, jwt, rustycog, security, visibility/internal]
sources:
  - docs/platform/authn-jwt.md
  - docs/guides/jwt-consommateur.md
  - IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - rustycog/rustycog-http/src/jwt_handler.rs
  - projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation.md
  - cursor-conversation/jwt-jwks-unification-2026-08-29
  - docs/adr/0302-authn-jwt-authz-openfga.md
  - docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md
  - docs/adr/0308-mesh-authn-jwt.md
summary: >-
  Cible 0304 : access RS256, kid opaque, JWKS, issuer par trust domain.
  Réalité Partial. HS256 = fenêtre de migration, plus le contrat cible.
provenance:
  extracted: 0.84
  inferred: 0.12
  ambiguous: 0.04
created: 2026-08-31T13:30:00Z
updated: 2026-10-01T16:45:00Z
---

# JWT issuer versus consumer

How-to (GitHub handbook, not this page): `docs/platform/authn-jwt.md` and `docs/guides/jwt-consommateur.md`.

## Current contract (2026-09-27)

Cible : [[projects/aiforall/decisions/0304-access-jwt-trust]] (ADR-0304 Accepted / Réalité Partial).

- Access token = RS256 + `kid` opaque. JWKS canonique `GET /iam/.well-known/jwks.json`. Principal = `(iss, sub)`. `aud` reste `aiforall`.
- L’émetteur logique est IAMRusty. L’opération crypto est un `SigningProvider` (OpenBao Transit autorisé ; PEM = dev). Les consommateurs ne connaissent pas le KMS.
- Issuer cible : `https://{host}/iam` (platform) ou `https://{host}/iam/orgs/{slug}` (org). Cela remplace, pour la cible seulement, `iss=iamrusty`.
- Réalité Partial : mint RS256, JWKS, extracteur rustycog RS256, rotate N+1, probe Transit. Pas de BYOKMS cloud, pas de remote signer (0309).
- HS256 et `iss=iamrusty` restent la fenêtre de migration / les tokens historiques, plus le contrat à écrire. La photo du 1er sept. (extracteur HS256-only, refus RS256) est caduque comme cible.

## Still a platform gap

- Adapters cloud BYOKMS et remote signer ne sont pas dans le dépôt.
- Le mesh qui valide puis émet `(iss, sub)` est ADR-0308 **Accepted / Partial** : opt-in ext_authz. Sur Compose, mTLS de hop + mode passerelle §7 sont livrés ; le défaut reste JWT in-process. [[projects/aiforall/concepts/mesh-ext-authz-opt-in]] [[projects/aiforall/concepts/mesh-gateway-principal-trust]]

## Related

- [[projects/aiforall/decisions/0300-events-authz]] — ADR 0302 (AuthZ inchangé)
- [[projects/aiforall/decisions/0304-access-jwt-trust]] — cible 0304–0309
- [[projects/aiforall/decisions/0308-mesh-authn-jwt]] — overlay mesh §7
- [[concepts/architecture-coherence-across-services]]
- [[projects/iamrusty/iamrusty]]
- [[skills/using-rustycog-http]]
- [[projects/iamrusty/references/iamrusty-runtime-and-security]]
