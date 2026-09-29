# Authentification JWT

Deux configurations distinctes. Les confondre est la principale source d’erreur.

## Contrat runtime (aujourd’hui)

Access JWT plateforme = **RS256 + `kid` opaque + JWKS**. Principal AuthN = `(iss, sub)`. `aud` = `aiforall`.

### Émetteur (IAMRusty seulement)

Section **`[jwt]`** : comment IAM **signe** les access tokens.

```toml
[jwt]
expiration_seconds = 900
refresh_token_expiration_seconds = 2592000
public_base_url = "http://127.0.0.1:8080"
jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
audience = "aiforall"
oauth_state_secret = "…"

[jwt.secret]
type = "pem_file"
private_key_path = "config/keys/test-platform.pem"
public_key_path = "config/keys/test-platform.pub"
key_id = "test-rs256-kid-01"
```

- Claims émis : `sub` (UUID), `iss` (`{public_base_url}/iam`), `aud`, `exp`, `iat`, `jti` ; header `typ=aiforall-access+jwt`, `alg=RS256`, `kid`.
- Signature via `SigningProvider` (PEM dev / Transit prod-capables). Registry `SigningKey` → JWKS.
- Rotation org = N+1 : clé Pending publiée dans le JWKS avant toute signature ; `can_sign` = Active seulement.
- Probe Transit = challenge `sign_digest` + verify PKCS1v15 (URL Transit = config IAM, refuse fermé sans URL).
- Identity org-managed persistée (RPC interne) sans mint de JWT org ; login/refresh restent platform.
- Hive s2s → IAM via `WorkloadIdentity` / `StaticCredential` (`iam-internal-token`).
- JWKS : `GET /iam/.well-known/jwks.json` construit depuis `SigningKeyRegistry::list_jwks_keys` (pending+active+retiring ; retiring hors fenêtre TTL+60s), fallback cache bootstrap si vide. Jamais HMAC dans le JWKS.
- Login / refresh appellent `ensure_platform_identity(user_id, platform_issuer)`.

Guide historique (non canon) : [`IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`](../../IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md). Recette consommateur : [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md).

### Consommateurs (Hive, Manifesto, Telegraph, extractor IAM)

Section **`[auth.jwt]`** :

```toml
[auth.jwt]
audience = "aiforall"
jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
```

`UserIdExtractor` ([`rustycog/rustycog-http/src/jwt_handler.rs`](../../rustycog/rustycog-http/src/jwt_handler.rs)) :

- Vérifie **RS256** via JWKS (`kid` + `iss` du JWK). Insert `JwtPrincipal { iss, sub, org }` dans les extensions.
- Runtime prod/dev : `allowed_algorithms=["RS256"]`. HS256 n'est pas le défaut — uniquement via le flag explicite `allowed_algorithms` (fenêtre IT `test.toml`).
- Claims : `sub` UUID, `exp`, `iat`, `jti` ; RS256 exige aussi `typ=aiforall-access+jwt` et `kid`.

En test HS256 fenêtre : `create_jwt_token(user_id)` pose encore `iss=iamrusty` (backfill membership Hive). Préférer le `iss` du token (`JwtPrincipal`) plutôt que hardcoder `iamrusty` dans les services.

### Tableau runtime

| Élément | Valeur attendue |
|---|---|
| `iss` plateforme | `{public_base_url}/iam` |
| `aud` | `aiforall` |
| Algorithme access | RS256 |
| JWKS | `/iam/.well-known/jwks.json` |

## Cible (ADR-0304)

Ratifiée : [ADR-0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md). Identity : [0305](../adr/0305-account-identity-trust-domain.md). Config signer Hive→IAM : [0306](../adr/0306-hive-iam-configuration-signature.md) (transport au composition root : InProcess monolithe / HTTP micro). WorkloadIdentity : [0307](../adr/0307-workload-identity-port.md) (Accepted / Implemented) — adapters OIDC WIF AWS/GCP/Azure sur le chemin HTTP (preuve wiremock `wif_exchanges`) avec fallback `StaticCredential` si provider absent. Mesh AuthN [0308](../adr/0308-mesh-authn-jwt.md) (`ext-authz/` Check HTTP) : profil opt-in (`docker compose --profile mesh`, overlay `kind-mesh/`) ; JWT rustycog in-process reste le défaut ; Accepted / Partial (pas Implemented). Remote signer HTTP [0309](../adr/0309-remote-signer.md) : Accepted / Partial (pas Implemented).

## Suite

- [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md)
