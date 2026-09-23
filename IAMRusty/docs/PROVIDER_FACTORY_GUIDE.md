# Adding a federated IdP (GitHub Connect gabarit)

IAM does **not** take a new vendor OAuth client in-process. A new identity provider is a **copy of the GitHubConnect gabarit**, a compose service, and one `[[idp.connectors]]` line. Routes `/iam/api/auth/{provider}/*` stay on IAM.

Canon: [ADR-0407](../../docs/adr/0407-contrat-authn-federee-vendor-neutral.md), [0408](../../docs/adr/0408-connecteurs-idp-services-http.md), [0409](../../docs/adr/0409-confiance-callback-oauth-idp-connect.md), [0410](../../docs/adr/0410-migration-iam-connecteurs-idp.md), [0411](../../docs/adr/0411-idp-provider-slug-registry-fail-closed.md).

`Provider` is a typed **slug** (letters-only, ASCII case-fold, length 1–50). Do **not** add an enum variant. IdP N+1 does not recompile `iam-domain`.

## What IAM owns vs what the connector owns

| Surface | Owner |
|---|---|
| Browser callback, CSRF `OAuthState` (HMAC + `exp`), account linking, JWT, `provider_tokens` | IAM |
| Vendor `client_id` / `client_secret`, authorize/token/profile HTTP to the vendor | Connector |
| IAM → connector hop | HMAC S2S (`idp-connect-contract`), not a user JWT |

## Architecture

```
Browser  →  IAM /iam/api/auth/{slug}/login|callback|link|relink
                 │  OAuthState CSRF, redirect_uri from registry
                 ▼
            HttpIdpConnector  (HMAC, 10s, no redirect follow)
                 │  HTTP JSON /v1/authorize|token/profile
                 ▼
            GitHubConnect / GitLabConnect / (copied gabarit)
```

`OAuthProviderFactory` is a `HashMap<Provider, Arc<dyn FederatedOAuthClient>>` filled at boot from every **complete** `[[idp.connectors]]` line and injected as a shared `Arc` at `OAuthService::new` (there is no `register_provider_client`). There is no generic `OAuthProviderFactory<GH, GL>` and no in-process vendor client in IAM.

## Add a connector (checklist)

1. **Copy the gabarit** `GitHubConnect/` (hexagon mince, no Postgres, no OpenFGA, no user JWT). Implement `FederatedOAuthClient` from `idp-connect-contract`. Serve `/v1/*` via the crate’s `server` feature (`connect_router`).
2. **Compose**: add a service (GitHub Connect is 8085/8446, GitLab Connect 8086/8447; next host ports follow that sequence). Do **not** nest the connector in `oodhive-monolith` (0408).
3. **Allowlist** the public IAM callback **and** relink-callback (exact last-segment match). Compose IAM is **8080** (Telegraph is 8081).
4. **Register IAM** in `IAMRusty/config/{development,default,production,test}.toml`:

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

`id` is letters-only (`huggingface` is fine; `hugging-face` is not). Restart **Connect** then **IAM**. Hot-load is not supported ([0411](../../docs/adr/0411-idp-provider-slug-registry-fail-closed.md)).

5. **Boot is fail-closed**: empty `idp.connectors`, incomplete line (empty `base_url`, missing Callback or Relink `redirect_uris`), illegal `id`, or HMAC shorter than 16 bytes after trim → IAM does not start. A well-formed slug **absent from the registry** is a **422** `connector_not_configured` at request time, not a skipped boot line.
6. **IT**: IAM tests mock the **connector** (WireMock HMAC), never `api.github.com`. Connector tests mock the **vendor**.

## What not to do

- Put `client_secret` in IAM TOML.
- Implement a vendor OAuth2 client inside `IAMRusty/infra`.
- Remount vendor APIs on IAM test port `:3000`.
- Treat Telegraph `:8081` as the IAM public callback in development compose.
- Skip unknown slugs at boot, add a flag `enabled`, or key the client map by `String`.
