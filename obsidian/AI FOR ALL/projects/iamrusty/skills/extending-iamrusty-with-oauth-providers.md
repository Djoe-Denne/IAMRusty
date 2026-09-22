---
title: Extending IAMRusty with OAuth Providers
category: skills
tags: [oauth, services, rust, visibility/internal]
sources:
  - IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md
  - IAMRusty/setup/src/app.rs
  - docs/adr/0407-contrat-authn-federee-vendor-neutral.md
  - docs/adr/0408-connecteurs-idp-services-http.md
  - docs/adr/0410-migration-iam-connecteurs-idp.md
summary: >-
  Add an IdP by shipping a Connect HMAC service and one [[idp.connectors]]
  line. IAM keeps /api/auth/{provider} routes; vendor clients stay out of IAM.
provenance:
  extracted: 0.82
  inferred: 0.14
  ambiguous: 0.04
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-09-22T12:49:00Z
---

# Extending IAMRusty with OAuth Providers

> [!note] Cible vivante (Implemented)
> Canon : [[projects/iamrusty/decisions/index]] (ADR-0407–0410, Réalité **Implemented**). Ajouter un IdP = **nouveau service connecteur** + ligne `[[idp.connectors]]`, pas un client in-process dans IAM.

Adding a provider to `[[projects/iamrusty/iamrusty]]` is no longer a cross-cutting IAM enum + factory change. GitHub and GitLab already live in `GitHubConnect/` and `GitLabConnect/`. IAM talks to them only through `HttpIdpConnector` (HMAC S2S).

## Key Ideas

- IAM owns browser callbacks, CSRF `OAuthState`, linking, JWT, and `provider_tokens`. The connector owns vendor `client_id` / `client_secret` and the GitHub/GitLab HTTP calls.
- Register the connector in IAM TOML: `id`, `base_url` (service prefix included), `hmac_secret` (≥ 16 chars), `redirect_uris` (callback **and** relink-callback). Compose IAM is port **8080**.
- Boot fails closed if `idp.connectors` is empty, any HMAC is shorter than 16 bytes, or no known v1 slug (`github` / `gitlab`) is wired.
- Domain `Provider` GitHub|GitLab remains the v1 **route** adapter ([0407 §8](docs/adr/0407-contrat-authn-federee-vendor-neutral.md)). Do not add Google to that enum; add a new Connect service instead.
- Integration tests: IAM mocks the **connector**; the connector mocks the **vendor**. Do not remount vendor APIs on IAM `:3000`. Do not nest Connect in `oodhive-monolith` (v1).

## Workflow

- Scaffold a Connect service (hexagon mince, no Postgres, no OpenFGA, no user JWT) implementing `FederatedOAuthClient` from `idp-connect-contract` feature `server`.
- Allowlist the public IAM callback URIs; keep HMAC secrets aligned between IAM `[[idp.connectors]]` and the connector config.
- Add the IAM registry line. `setup_http_idp_clients` maps known slugs onto `OAuthProviderFactory` (`HashMap<Provider, Arc<dyn FederatedOAuthClient>>`).
- Cover login/callback/link/relink with the existing IAM WireMock connector fixture; cover token+profile in the connector crate against WireMock vendor.

## Leftover (accurate)

- v1 routes still parse `{provider}` as GitHub|GitLab. Extra slugs in the registry are skipped until a later route slice. ^[extracted]

## Sources

- [[projects/iamrusty/iamrusty]] - Platform IdP that consumes Connect.
- [[projects/iamrusty/concepts/oauth-provider-linking]] - Linking stays on IAM.
- [[projects/iamrusty/decisions/0408-connecteurs-http]] - Connect = HTTP services.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] - `/iam/api/auth/{provider}` stays.
