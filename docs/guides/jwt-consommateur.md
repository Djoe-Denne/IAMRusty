# Recette : consommer un JWT IAM

Pour Hive, Manifesto, Telegraph, ou un nouveau service HTTP. L’émetteur reste IAM ([../platform/authn-jwt.md](../platform/authn-jwt.md)).

## 1. TOML

```toml
[auth.jwt]
audience = "aiforall"
jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
```

Runtime : `allowed_algorithms=["RS256"]` — HS256 n'est pas le défaut. La fenêtre HS256 n'existe que si `allowed_algorithms` liste **explicitement** HS256 (`test.toml`).

## 2. Extractor

```rust
let extractor = rustycog::http::UserIdExtractor::new(auth_config)?;
// AppState::new(command_service, extractor, permission_checker)
```

Le middleware insère `Uuid` (`sub`) **et** `JwtPrincipal { iss, sub, org }` dans les extensions.

## 3. Routes

```rust
.get("/api/me", get_me)
.authenticated()
```

Handler : `AuthUser` et/ou `Extension<JwtPrincipal>` pour lire `principal.iss` (membership Hive, etc.).

## 4. Tests

```rust
use rustycog::testing::http::jwt::create_jwt_token; // HS256 fenêtre IT, iss=iamrusty

let token = create_jwt_token(owner_id);
```

Pour RS256 inline : `create_rs256_jwt_token` / `UserIdExtractor::from_inline_jwks`.

## 5. Ne pas faire

- Mint HS256 pour un consommateur runtime `allowed_algorithms=[RS256]`.
- Hardcoder `iss=iamrusty` dans les services métier — utiliser `JwtPrincipal.iss`.
- Publier un HMAC dans le JWKS.
- Omettre `kid` / `typ=aiforall-access+jwt` sur un access RS256.

## Rotation N+1 (consommateurs)

Pendant une rotation org, le JWKS peut contenir `pending` + `active` + `retiring`. Les validateurs doivent accepter tout `kid` présent dans le JWKS (RS256). IAM ne signe les access tokens qu’avec la clé plateforme Active au boot — pas avec une clé org Pending.
Identity org-managed peut exister en base sans JWT org minté. Hive appelle IAM en s2s via WorkloadIdentity.
