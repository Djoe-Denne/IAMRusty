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
  - docs/adr/0411-idp-provider-slug-registry-fail-closed.md
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
> Canon : [[projects/iamrusty/decisions/index]] (ADR-0407–**0411**, Réalité **Implemented**). Ajouter un IdP = **nouveau service connecteur** + ligne `[[idp.connectors]]` **complète**, pas un client in-process dans IAM. Détail slug/registry : [[projects/iamrusty/decisions/0411-slug-registry]].

Adding a provider to `[[projects/iamrusty/iamrusty]]` is no longer a cross-cutting IAM enum + factory change. GitHub and GitLab already live in `GitHubConnect/` and `GitLabConnect/`. IAM talks to them only through `HttpIdpConnector` (HMAC S2S).

## Key Ideas

- IAM owns browser callbacks, CSRF `OAuthState`, linking, JWT, and `provider_tokens`. The connector owns vendor `client_id` / `client_secret` and the GitHub/GitLab HTTP calls.
- Register the connector in IAM TOML: `id`, `base_url` (service prefix included), `hmac_secret` (≥ 16 chars), `redirect_uris` (callback **and** relink-callback). Compose IAM is port **8080**.
- Boot fails closed if `idp.connectors` is empty, any line is incomplete (empty `base_url`, missing Callback or Relink `redirect_uris`, illegal `id`, HMAC shorter than 16 bytes). **Hot-load is not supported** — restart IAM after a registry change ([0411](docs/adr/0411-idp-provider-slug-registry-fail-closed.md)).
- `Provider` is a typed **slug** (letters-only, ASCII case-fold, length 1–50), not an enum. Parse: `Provider::parse_slug`. Illegal syntax → **400** `invalid_provider`. Well-formed slug absent from the registry → **422** `connector_not_configured`. Catalogue = shared `Arc<HashMap<Provider, Arc<dyn FederatedOAuthClient>>>` injected at `OAuthService::new`.
- Integration tests: IAM mocks the **connector**; the connector mocks the **vendor**. Do not remount vendor APIs on IAM `:3000`. Do not nest Connect in `oodhive-monolith` (v1).

## Workflow

- Scaffold a Connect service (hexagon mince, no Postgres, no OpenFGA, no user JWT) implementing `FederatedOAuthClient` from `idp-connect-contract` feature `server`.
- Allowlist the public IAM callback URIs; keep HMAC secrets aligned between IAM `[[idp.connectors]]` and the connector config.
- Add a **complete** IAM registry line. Boot builds `HashMap<Provider, Arc<dyn FederatedOAuthClient>>` and injects it at `OAuthService::new` (no `register_provider_client`, no skip of extra slugs).
- Cover login/callback/link/relink with the existing IAM WireMock connector fixture; cover token+profile in the connector crate against WireMock vendor.

## Leftover (accurate)

- HMAC S2S Connect remains ADR-0409 (unchanged). Wiki is not more canonical than [ADR-0411](docs/adr/0411-idp-provider-slug-registry-fail-closed.md). ^[extracted]

## Sources

- [[projects/iamrusty/iamrusty]] - Platform IdP that consumes Connect.
- [[projects/iamrusty/concepts/oauth-provider-linking]] - Linking stays on IAM.
- [[projects/iamrusty/decisions/0408-connecteurs-http]] - Connect = HTTP services.
- [[projects/iamrusty/decisions/0411-slug-registry]] - Typed slug + fail-closed registry.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] - `/iam/api/auth/{provider_name}/login|callback|link|relink-*` stays.
