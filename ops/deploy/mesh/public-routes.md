# IAM public ingress contract

Reconfirmed against owner A's `services/IAMRusty/http/src/public_routes.rs` (2026-10-04). Both Envoy configurations exempt only these **method + path** combinations from bearer ext-authz. The five GET routes also permit **HEAD**, explicitly matching Axum's implicit HEAD behavior:

- GET `/iam/.well-known/jwks.json`
- POST `/iam/api/auth/signup`, `/iam/api/auth/login`
- GET `/iam/api/auth/verify`, `/iam/api/auth/username/check`
- POST `/iam/api/auth/resend-verification`, `/iam/api/auth/complete-registration`
- POST `/iam/api/auth/password/reset-request`, `reset-validate`, `reset-confirm`
- GET `/iam/api/auth/{provider_name}/login`, `/iam/api/auth/{provider_name}/callback` (one path segment only)
- POST `/iam/api/token/refresh`
- GET/HEAD `/iam/api/auth/github/relink-callback`
- GET/HEAD `/iam/api/auth/gitlab/relink-callback`

Section 9 of `20261003-auth-fixes-interface-contract.md` supersedes the old relink-callback Bearer requirement **only on these two exact paths and GET/HEAD**. They are **state/browser-DB authenticated callbacks**, not unguarded OAuth: consume the signed state + browser-bound Relink transaction atomically before any exchange/link/credentials. Envoy does not validate this transaction. Do not deploy these exemptions before compliant A2 handlers and the callback-only SDK authentication change are integrated.

Compose/Kind served development registry explicitly lists `github` and `gitlab` with direct IAM `redirect_uris` ending in the exact hyphenated `/relink-callback`. New provider slugs require explicit configuration and exact rules, not a provider wildcard. Local-full's current `connectors=[]` remains unchanged: its renderer removes both new exemptions, retaining them only for an explicitly configured github/gitlab connector with its matching redirect path. Generated config identity is computed after filtering.

Counts: **22 symbolic pairs** = original 13 + five implicit HEAD + four concrete relink pairs. Expanding the original two provider-parametrized GET routes and their HEAD counterparts over github/gitlab yields **26 concrete pairs** in Compose/Kind, not 22. Existing generic login/callback regex behavior is unchanged by this delta. Local-full enables **zero new relink pairs**; disabled providers are not represented as live OAuth flows.

All other paths/methods retain ext-authz, including `/api/me`, START link/relink, authenticated password reset, echo and `/internal/` S2S/signer operations. No `/iam/` prefix bypass. Principal-header removal runs on callback routes as well.

E final assertions: parity + four added concrete pairs; START requires platform JWT; callback without Authorization succeeds only with valid transaction/cookie; invalid/replayed/wrong-intention/provider/browser/redirect state fails before effects. POST, suffix/subpath, encoded-path variants and unregistered providers must not gain the exemption. Health/readiness probes use direct service ports, not a generic public mesh exemption. A2/SDK agreement remains a deployment gate, not a source-only closure claim.
