# Adding a federated IdP (GitHub/GitLab Connect)

IAM does **not** take a new vendor OAuth client in-process. A new identity provider is a **connector HTTP service** plus one `[[idp.connectors]]` line. Routes `/iam/api/auth/{provider}/*` stay on IAM.

Canon: [ADR-0407](../../docs/adr/0407-contrat-authn-federee-vendor-neutral.md), [0408](../../docs/adr/0408-connecteurs-idp-services-http.md), [0409](../../docs/adr/0409-confiance-callback-oauth-idp-connect.md), [0410](../../docs/adr/0410-migration-iam-connecteurs-idp.md).

## What IAM owns vs what the connector owns

| Surface | Owner |
|---|---|
| Browser callback, CSRF `OAuthState` (HMAC + `exp`), account linking, JWT, `provider_tokens` | IAM |
| Vendor `client_id` / `client_secret`, authorize/token/profile HTTP to GitHub/GitLab | Connector |
| IAM → connector hop | HMAC S2S (`idp-connect-contract`), not a user JWT |

v1 route adapter: domain `Provider` remains `GitHub | GitLab` ([0407 §8](../../docs/adr/0407-contrat-authn-federee-vendor-neutral.md)). Do not add Google (or N+1) as an IAM enum variant; add a **new service**.

## Architecture

```
Browser  →  IAM /iam/api/auth/{slug}/login|callback|link|relink
                 │  OAuthState CSRF, redirect_uri from registry
                 ▼
            HttpIdpConnector  (HMAC, 10s, no redirect follow)
                 │  HTTP JSON /v1/authorize|token|profile
                 ▼
            GitHubConnect / GitLabConnect  (vendor OAuth client)
```

`OAuthProviderFactory` is a `HashMap<Provider, Arc<dyn FederatedOAuthClient>>` filled at boot from `[[idp.connectors]]`. There is no generic `OAuthProviderFactory<GH, GL>` and no in-process vendor client in IAM.

## Add a connector (checklist)

1. **Scaffold a Connect service** (hexagon mince, no Postgres, no OpenFGA, no user JWT). Pattern: `GitHubConnect/`, `GitLabConnect/`. Implement `FederatedOAuthClient` from `idp-connect-contract`. Serve `/v1/*` via the crate’s `server` feature (`connect_router`).
2. **Allowlist** the public IAM callback **and** relink-callback (exact string match). Compose IAM is **8080** (Telegraph is 8081).
3. **Register IAM** in `IAMRusty/config/{development,default,production,test}.toml`:

```toml
[[idp.connectors]]
id = "github"
base_url = "http://github-connect-service:8080/github-connect"
hmac_secret = "change-me-github-connect-hmac"   # ≥ 16 chars; same value as the connector
redirect_uris = [
  "http://127.0.0.1:8080/iam/api/auth/github/callback",
  "http://127.0.0.1:8080/iam/api/auth/github/relink-callback",
]
```

4. **Boot is fail-closed**: empty `idp.connectors`, HMAC shorter than 16 bytes, or a registry that wires only unknown slugs → IAM does not start.
5. **IT**: IAM tests mock the **connector** (WireMock HMAC), never `api.github.com`. Connector tests mock the **vendor**. Do not nest the connector in `oodhive-monolith` (v1).

Unknown slugs in the registry are skipped; a slug still needs a `Provider` v1 adapter (GitHub/GitLab) to be reachable on `/api/auth/{provider}`. Extra IdPs wait on a later route/registry slice — do not expand the enum “just because”.

## What not to do

- Put `client_secret` in IAM TOML.
- Implement a vendor OAuth2 client inside `IAMRusty/infra`.
- Remount vendor APIs on IAM test port `:3000`.
- Treat Telegraph `:8081` as the IAM public callback in development compose.
