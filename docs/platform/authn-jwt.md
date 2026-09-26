# Authentification JWT

Deux configurations distinctes. Les confondre est la principale source d’erreur.

## Contrat runtime (aujourd’hui)

Ce que le dépôt **fait**. Ne pas présenter RS256, Transit JWT, ou issuer par trust domain comme livrés.

### Émetteur (IAMRusty seulement)

Section **`[jwt]`** : comment IAM **signe** les access tokens (et registration tokens).

```toml
[jwt]
expiration_seconds = 900
refresh_token_expiration_seconds = 2592000
issuer = "iamrusty"
audience = "aiforall"
oauth_state_secret = "…"

[jwt.secret]
type = "plain"
value = "rustycog-dev-hs256-secret"
```

- Claims émis : `sub` (UUID user), `iss`, `aud`, `exp`, `iat`, `jti` (`TokenClaims` — pas `typ`, pas `org`).
- Algorithmes : le type connaît HS256 **ou** RS256, mais le composition root **refuse RS256** : `JwtConfig::http_verifier_auth` échoue, parce que `UserIdExtractor` (rustycog-http) ne vérifie que HS256. Un issuer RS256 ferait 401 tous les `.authenticated()` des consommateurs.
- La feature Cargo `test-relaxed-jwt` est un no-op conservé pour les wires existants. HS256 est first-class.
- JWKS : `GET /iam/.well-known/jwks.json`. **Aucun consommateur actuel ne l’utilise.**
- IAM recopie le HMAC de `[jwt.secret]` vers `AuthConfig` pour **ses propres** routes authentifiées (`/api/me`, …).
- SigningProvider / OpenBao Transit pour JWT user : **ABSENT**. Transit OpenBao = Cosign Apparatus seulement.

Guide historique (non canon) : [`IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`](../../IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md). Recette consommateur : [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md).

### Consommateurs (Hive, Manifesto, Telegraph, et extractor IAM)

Section **`[auth.jwt]`** : comment rustycog-http **vérifie** un `Authorization: Bearer`.

```toml
[auth.jwt]
hs256_secret = "rustycog-dev-hs256-secret"   # test : rustycog-test-hs256-secret
issuer = "iamrusty"
audience = "aiforall"
```

`UserIdExtractor` ([`rustycog/rustycog-http/src/jwt_handler.rs`](../../rustycog/rustycog-http/src/jwt_handler.rs)) :

- Algorithme **HS256 uniquement** (pas JWKS, pas RS256).
- Secret obligatoire (trim non vide).
- Si `issuer` / `audience` sont non vides : ils deviennent des *required claims* et doivent matcher.
- Claims toujours exigés côté decode applicatif : `sub` (UUID), `exp`, `iat`, `jti` (non vide).
- `nbf` non validé.

En test, `create_jwt_token(user_id)` ([`rustycog/rustycog-testing/src/http/jwt.rs`](../../rustycog/rustycog-testing/src/http/jwt.rs)) émet `iss=iamrusty`, `aud=aiforall`, secret `rustycog-test-hs256-secret`. Un token sans `iss`/`aud` contre un TOML qui les définit → 401.

### Tableau runtime

| Élément | Valeur attendue (runtime) |
|---|---|
| `iss` | `iamrusty` |
| `aud` | `aiforall` |
| Secret local / compose | `rustycog-dev-hs256-secret` |
| Secret tests | `rustycog-test-hs256-secret` |
| Prod | secrets **vides** dans les TOML ; `IAM_JWT__SECRET__VALUE` et `IAM_AUTH__JWT__HS256_SECRET` (ou équivalent service) |

Les quatre services doivent partager **le même HMAC** tant que l’extractor est HS256-only. Rotation = changer IAM `[jwt.secret]` **et** tous les `[auth.jwt].hs256_secret`.

## Cible (ADR-0304 révisée — Unimplemented)

Cible ratifiée : [ADR-0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md) (RS256 + `kid` opaque + JWKS ; SigningProvider Transit|PEM ; issuer par trust domain ; principal `(iss, sub)`). Modèle Account/Identity : [0305](../adr/0305-account-identity-trust-domain.md). Config signer Hive→IAM : [0306](../adr/0306-hive-iam-configuration-signature.md).

**Ne pas** dire que RS256 ou Transit JWT sont en prod. `aud` reste `aiforall`. `iss=iamrusty` = platform historique jusqu’au cutoff issuer.

## Suite

- [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md)
- Wiki : `projects/aiforall/concepts/jwt-issuer-vs-consumer`
