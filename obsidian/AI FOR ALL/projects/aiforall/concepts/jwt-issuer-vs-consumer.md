---
title: >-
  JWT issuer versus consumer
category: concepts
tags: [iam, jwt, rustycog, security, visibility/internal]
sources:
  - docs/reviews/iam-architecture-comparison.md
  - projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation.md
  - cursor-conversation/jwt-jwks-unification-2026-08-29
summary: >-
  Service extractors verify HS256 via rustycog-http; IAMRusty still issues RS256 outside test-relaxed-jwt. That split is the open JWT unification gap.
provenance:
  extracted: 0.8
  inferred: 0.15
  ambiguous: 0.05
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# JWT issuer versus consumer

All four services verify inbound bearer tokens through rustycog-http `UserIdExtractor`, which is **HS256-only**. Shared `[auth.jwt]` consumer config points at the same HS256 secret.

IAMRusty as issuer is different: production constructors still enforce RS256 for access and registration tokens, gated by the `test-relaxed-jwt` Cargo feature so `test.toml` can boot HS256. See [[projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation]].

## Why this is a platform gap

- Consumers cannot validate an RS256 token IAM would issue with default/production config. ^[inferred]
- JWKS / `.well-known` exists on the IAM router (`/iam/.well-known/jwks.json` in monolith mode) but the shared extractor does not use it yet.
- Unification work started 2026-08-29: one verification story across rustycog-http, IAMRusty, and the three consumers. Resolution is still in progress. ^[ambiguous]

## Related

- [[concepts/architecture-coherence-across-services]]
- [[projects/iamrusty/iamrusty]]
- [[skills/using-rustycog-http]]
- [[projects/iamrusty/references/iamrusty-runtime-and-security]]
