# IAMRusty

Identité et accès : comptes, OAuth GitHub/GitLab via connecteurs HTTP HMAC, JWT, refresh, reset MDP.

- Préfixe : `/iam` — compose : **8080**
- JWT **émetteur** : `[jwt]` + `[jwt.secret]` (HS256 obligatoire aujourd’hui)
- JWT **consommateur** (ses routes `/api/me`, …) : `[auth.jwt]` recopié depuis le HMAC issuer
- OpenFGA : non (IAM n’écrit pas les tuples org/projet)
- Events : `user_signed_up`, `user_email_verified`, `password_reset_requested` → `telegraph-events`

## Docs

- Handbook : [../platform/authn-jwt.md](../platform/authn-jwt.md), [../functional/identite.md](../functional/identite.md)
- [`services/IAMRusty/README.md`](../../services/IAMRusty/README.md), [`services/IAMRusty/docs/`](../../services/IAMRusty/docs/), QA [`services/IAMRusty/qa/scenarii/`](../../services/IAMRusty/qa/scenarii/)
