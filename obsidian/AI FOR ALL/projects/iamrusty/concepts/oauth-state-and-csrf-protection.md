---
title: OAuth State and CSRF Protection
category: concepts
tags: [security, oauth, csrf, visibility/internal]
sources:
  - IAMRusty/docs/OAUTH_SECURITY_GUIDE.md
  - IAMRusty/http/src/oauth_state.rs
  - IAMRusty/http/src/handlers/auth.rs
  - docs/adr/0409-confiance-callback-oauth-idp-connect.md
summary: >-
  OAuthState carries operation, nonce, provider, exp (TTL 600 s) and HMAC.
  Path slug ≠ state.provider → 400 invalid_state. redirect_uri comes from
  [[idp.connectors]] redirect_uris, not a hardcoded 8081 handler.
provenance:
  extracted: 0.84
  inferred: 0.12
  ambiguous: 0.04
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-09-22T12:49:00Z
---

# OAuth State and CSRF Protection

> [!note] Code (2026-09-22)
> `OAuthState` a `operation`, `nonce`, `provider`, `exp` (TTL 600 s), HMAC et anti-rejeu. `redirect_uri` login/relink vient du registry `[[idp.connectors]]` (last-segment `callback` vs `relink-callback`). Canon CSRF HMAC : [[projects/iamrusty/decisions/0409-confiance-oauth]]. Binding slug : [[projects/iamrusty/decisions/0411-slug-registry]] / [ADR-0411](docs/adr/0411-idp-provider-slug-registry-fail-closed.md).

IAM treats OAuth `state` as a security control, not a pass-through cookie. The connector **propagates** `state` to the vendor; it does not mint CSRF.

## Key Ideas

- Login and link create an `OAuthState` before redirecting (`new_login(provider)`, `new_link(user_id, provider)`). The callback decodes it before any auth command runs.
- `http/src/oauth_state.rs` stores operation, nonce, **provider**, and `exp`. Encode is HMAC-signed. Link carries the authenticated user ID.
- Path slug ≠ `state.provider` → `400` `invalid_state`.
- TTL is **600 s**. HMAC/TTL remain ADR-0409. ^[extracted]
- `redirect_uri` is chosen from the connector’s `redirect_uris[]` (exact public IAM URLs). Development compose uses **8080**; IAM **tests** still use **8081** because that is the rustycog-testing IAM port, not Telegraph. ^[extracted]
- The callback rejects missing code/state, provider errors, illegal slugs (400), unregistered slugs (422), and invalid state before login or link.
- Authenticated link also runs `GetUserCommand`, so link is bound to bearer auth and the state payload.

## Leftover (accurate)

- Vendor `client_secret` is on GitHub Connect / GitLab Connect. IAM’s remaining HMAC secret is the IdP-connect S2S key plus `jwt.oauth_state_secret`. ^[extracted]

## Sources

- [[projects/iamrusty/iamrusty]] - Service that mints and checks state.
- [[projects/iamrusty/concepts/oauth-provider-linking]] - Flow that depends on operation-aware state.
- [[projects/iamrusty/decisions/0409-confiance-oauth]] - Callback/CSRF ownership.
- [[projects/iamrusty/decisions/0411-slug-registry]] - Provider slug bound in state.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] - Handler-level callback validation.
