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
summary: >-
  IAM mints HS256 with iss/aud; all HTTP extractors verify the same HMAC. RS256/JWKS remains unused. Telegraph aligned [auth.jwt] on 2026-09-01.
provenance:
  extracted: 0.85
  inferred: 0.12
  ambiguous: 0.03
created: 2026-08-31T13:30:00Z
updated: 2026-09-01T18:00:00Z
---

# JWT issuer versus consumer

How-to (GitHub handbook, not this page): `docs/platform/authn-jwt.md` and `docs/guides/jwt-consommateur.md`.

## Current contract (2026-09-01)

- **Issuer** is IAMRusty `[jwt]` + `[jwt.secret]`. Checked-in TOMLs mint **HS256** with `issuer = "iamrusty"` and `audience = "aiforall"`. Claims: `sub`, `iss`, `aud`, `exp`, `iat`, `jti`.
- **Consumers** (Hive, Manifesto, Telegraph, IAM's own extractor) use `[auth.jwt]` (`hs256_secret`, `issuer`, `audience`). rustycog-http `UserIdExtractor` is **HS256-only**.
- Telegraph gained `issuer` / `audience` on 2026-09-01 (`68ac628`) — all four services now share the same consumer block.
- IAM `JwtConfig::http_verifier_auth` **refuses RS256**: an RSA issuer would 401 every `.authenticated()` route. Feature `test-relaxed-jwt` is a no-op leftover.
- Test helper `create_jwt_token` emits the same `iss`/`aud` and `rustycog-test-hs256-secret`.

## Still a platform gap

- JWKS (`GET /iam/.well-known/jwks.json`) exists on the IAM router. The shared extractor does **not** use it.
- Unification toward one RS256/JWKS story (started 2026-08-29) is unfinished. Do not flip production `[jwt.secret]` to PEM until rustycog-http verifies JWKS. ^[ambiguous]

## Related

- [[concepts/architecture-coherence-across-services]]
- [[projects/iamrusty/iamrusty]]
- [[skills/using-rustycog-http]]
- [[projects/iamrusty/references/iamrusty-runtime-and-security]]
