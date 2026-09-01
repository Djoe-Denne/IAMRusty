# Recette : consommer un JWT IAM

Pour Hive, Manifesto, Telegraph, ou un nouveau service HTTP. L’émetteur reste IAM ([../platform/authn-jwt.md](../platform/authn-jwt.md)).

## 1. TOML

Dans `config/default.toml`, `development.toml`, `test.toml` :

```toml
[auth.jwt]
hs256_secret = "rustycog-dev-hs256-secret"    # test.toml : rustycog-test-hs256-secret
issuer = "iamrusty"
audience = "aiforall"
```

Prod : `hs256_secret = ""` et override env (`<PREFIX>_AUTH__JWT__HS256_SECRET`). Le secret **doit** matcher `[jwt.secret].value` d’IAM.

## 2. Extractor

Dans le setup / composition root :

```rust
let extractor = rustycog::http::UserIdExtractor::new(auth_config)?;
// AppState::new(command_service, extractor, permission_checker)
```

`AuthConfig` vient de la section `auth` (rustycog-config). Secret vide → erreur au boot, pas un extracteur « ouvert ».

## 3. Routes

```rust
.get("/api/me", get_me)
.authenticated()
```

Handler : extraire `AuthUser` (ou `OptionalAuthUser` après `.might_be_authenticated()`). Le middleware lit `Authorization: Bearer <token>`.

## 4. Tests

```rust
use rustycog::testing::http::jwt::create_jwt_token;

let token = create_jwt_token(owner_id);
let res = client
    .get(format!("{server_url}/api/..."))
    .bearer_auth(&token)
    .send()
    .await?;
```

`create_jwt_token` pose `iss=iamrusty`, `aud=aiforall`, secret `rustycog-test-hs256-secret`. Si le `test.toml` diverge (secret ou iss/aud), 401.

Pour un secret custom : `create_jwt_token_with_secret(user_id, secret)` — les claims iss/aud restent les constantes de test.

## 5. Ne pas faire

- Vérifier RS256 / JWKS dans le service — l’extractor ne sait pas.
- Omettre `iss`/`aud` dans le TOML « pour simplifier » si IAM les émet : dès qu’ils sont set, ils sont **requis**.
- Signer des tokens de test à la main sans `jti` / `iat` / `sub` UUID.
- Partager un HMAC de prod dans Git.

## IAM côté émetteur (rappel)

`[jwt]` + `[jwt.secret]`, `issuer` / `audience` identiques. Le composition root refuse RSA tant que rustycog-http est HS256-only.
