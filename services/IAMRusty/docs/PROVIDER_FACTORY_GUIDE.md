# Adding a federated identity-provider instance

Target: [ADR-0413](../../../docs/adr/0413-oauthconnect-multi-instances-profils.md), **Accepted / Partial**, and the [IAM v2 implementation contract](../../../docs/contracts/iam-oauthconnect-v2.md). The [multi-instance contract](../../../docs/contracts/oauthconnect-multi-instances.md) defines the Connect profiles, secrets and cutover conditions.

IAM does **not** host a vendor OAuth/OIDC client. Add an instance to the existing **OAuthConnect** service and register its base URL in IAM. Do not copy GitHubConnect into a new service for each provider. Legacy services remain present until the separately qualified cutover; this guide does not activate a deployment.

`Provider` remains a typed slug: ASCII letters only, case-folded to lowercase, length 1–50. `huggingface` is compatible; `hugging-face` and digits are not. No enum variant or recompilation of `iam-domain` is required for another compatible registered instance. Preserve existing `github`/`gitlab` slugs and their account links.

## Responsibilities and protocol

- **IAM** owns signed state, browser-cookie binding, the durable authentication transaction, account/link rules, IAM UUIDs, sessions and JWT issuance.
- **IAM BEGIN** generates an independent 32-byte base64url authentication nonce, persists it before START, and exposes only nonce and optional S256 challenge. The verifier remains private.
- **OAuthConnect** owns vendor credentials, endpoint/scopes/profile configuration, OAuth/OIDC completion and normalized identity. Private OIDC signing keys remain with the provider, not IAM or Connect.
- **The S2S hop** uses the instance's HMAC key and the complete received PATH/body, not a user Bearer token. TLS trust and the HMAC key are separate from vendor credentials and IAM JWT keys.

```text
Browser → IAM /iam/api/auth/{slug}/login|callback|link|relink
                   │ signed state + cookie + durable writer transaction
                   ▼
              HttpIdpConnector (verified TLS, HMAC, bounded response)
                   │ POST <instance-base>/v2/authorize
                   │ POST <instance-base>/v2/complete
                   ▼
              OAuthConnect instance → configured OAuth or OIDC provider
```

LOGIN, LINK and RELINK use the same v2 completion result containing **tokens and identity together**. There is no IAM fallback to v1 token/profile calls or profile lookup from the access token alone. The transaction writer consumes and erases nonce/verifier in one commit before completion; replay or legacy/corrupt context requires a new START.

`OAuthProviderFactory` remains a boot-filled `HashMap<Provider, Arc<dyn FederatedOAuthClient>>`, shared with the usecases. Application consumers import the port's existing v2 types through `iam_domain::port::service`; do not add a vendor client, duplicate DTO or new SDK dependency. The monolith keeps this HTTP adapter; it does not nest OAuthConnect or introduce an in-process vendor bridge.

## Register an instance

1. Configure an instance in OAuthConnect using its existing OAuth or OIDC profile. Keep vendor credentials and mappings there, not in IAM. Align instance slug and PKCE policy; do not infer profile kind from `github` or `gitlab`.
2. Provision the instance's HMAC key through the approved secret mechanism. Both ends must use the same value, at least 16 bytes after normalization; never reuse a vendor client secret or an IAM JWT key for this purpose.
3. Register a **complete** `[[idp.connectors]]` entry in IAM. The example below is illustrative, not a deployed endpoint or production secret:

```toml
[[idp.connectors]]
id = "github"
base_url = "https://connect.example.test/oauth-connect/github"
hmac_secret = "illustrative-instance-key-change-me"
redirect_uris = [
  "http://127.0.0.1:8080/iam/api/auth/github/callback",
  "http://127.0.0.1:8080/iam/api/auth/github/relink-callback",
]
```

4. Configure verified TLS trust and the existing public IAM callback allowlist. S2S Connect routes must not become browser routes. No wildcard alias or shortened signed PATH; plaintext needs an existing explicit local/test exception, not an implicit fallback.
5. Boot validation stays fail-closed: empty registry, incomplete entry, illegal/duplicate slug, invalid redirect/transport or missing/short HMAC does not silently disable an instance. A valid but unregistered slug remains a request-time `connector_not_configured` error. Registry hot-load is not supported.

## Migration, identity and cutover

Apply the additive IAM migration before the new binary: `oauth_transactions.authentication_nonce TEXT NULL`, without backfill or account/token migration. Drain old STARTs for their TTL (at most 600 seconds), or expect their callbacks to be refused with a new START required. Rollback keeps the nullable column and restores the prior binary/config; do not reset the database or run a destructive down migration.

Identity remains **instance + opaque external ID**, linked to the same IAM UUID. Do not normalize IDs numerically, force email matching, relink in bulk or rewrite an issuer in place. LOGIN still requires an email even for an existing link; LINK/RELINK retain their distinct existing rules. See the contract for the exact business ordering and unresolved rare-case arbitration.

Pointing deployed IAM URLs at OAuthConnect and retiring legacy services are separate operations. Real GitHub/GitLab E2E evidence is owned elsewhere. In particular, GitLab's new `sub` must equal the existing canonical GitLab ID on the same real account; a mock cannot prove this. Stop the cutover if it differs—do not improvise an account migration.

## Verification boundaries

Native Windows units exercise the real IAM adapter against outbound HTTP collaborators. PostgreSQL guarantees use the existing RustyCog testcontainer fixtures; TLS uses the existing local verified-CA/SAN fixture. Neither mocks nor a successful compile qualify a real provider or deployment. Keep codes, tokens, nonce and verifier out of diagnostics; preserve the existing trace-redaction checks.
