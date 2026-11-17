# OAuthConnect

OAuthConnect is the sole standalone federated-authenticator service.
GitHub is an OAuth profile; GitLab is an OIDC profile. IAM retains platform
accounts, browser transactions and token authority. No Connect is nested in the
monolith.

## Listener and IAM bases

One listener hosts all configured instances. The IAM adapter appends
`/v2/authorize` and `/v2/complete` to each instance base:

- Compose: `http://oauth-connect-service:8080/oauth-connect/github` and
  `http://oauth-connect-service:8080/oauth-connect/gitlab`.
- Host access to that Compose listener: port **8085**, with the same paths.
- TLS host mapping: **8446 → 8443**; production examples use verified HTTPS
  on `oauth-connect-service:8443`, not the host mapping.
- A native process must explicitly configure its listener (for the host IAM
  examples, `127.0.0.1:8085`); the Docker mapping does not configure native ports.

No provider-specific listener, old route alias, v1 fallback or capability
negotiation is introduced. Per-instance HMAC signs the complete v2 path.
HTTP on the development Compose network is the existing local exception,
not production guidance.

## Operator-owned configuration

There is no `services/OAuthConnect/config` directory to mount. Compose requires
existing directories, mounted read-only, and refuses to create missing sources:

- `OAUTH_CONNECT_CONFIG_DIR` (default `../aiforall-oauth-connect/config`) supplies
  `/app/config`, with the strict SDK loader's `default.toml` and
  `development.toml` configuration (`RUN_ENV=development`).
- `OAUTH_CONNECT_RUNTIME_DIR` (default `../aiforall-oauth-connect/runtime`) supplies
  `/run/oauth-connect` for operator-provisioned token files and trust material.
- Existing platform mesh certificates are mounted at `/app/certs`.

The default operator directories are outside the repository and build context.
The runtime Dockerfile excludes the entire context (artifacts come from
`BUILD_IMAGE`); the artifact builder also excludes `.local/oauth-connect` if an
operator explicitly chooses that ignored in-repository location. No private
directory is created or populated by this retirement.

Configure the Compose listener at `0.0.0.0:8080` and, if enabled, TLS at 8443
with the `oauth-connect-service` certificate. Declare the `github` and `gitlab`
instances, exact IAM callback allowlists, and each profile's actual PKCE
capability. Configure verified vendor/OpenBao transport and explicit secret
references. Provider OAuth secrets belong here, not in IAM; each HMAC key must
match its IAM instance. Keep credentials and operator inputs outside Git.

The retained declarative [catalogue fixture](../../services/OAuthConnect/configuration/tests/fixtures/ops-connect.toml)
illustrates the schema, not a ready-to-launch Compose configuration: it contains
historical cluster addresses and template substitutions, and its second profile
is a simulator, not GitLab. The configuration tests include OIDC examples.
Do not copy its cluster paths or local transport exception into production.
This retirement does not provision tokens, seed secrets, or start Compose.

Existing OpenBao paths **`github-connect/oauth`** and
**`github-connect/iam`** remain valid credential references. No credential
rename, rotation, migration or live resource deletion follows from removal of
the old code.

## Scope and evidence

See [ADR 0413](../adr/0413-oauthconnect-multi-instances-profils.md),
the [IAM v2 contract](../contracts/iam-oauthconnect-v2.md), and the
[retirement contract](../contracts/legacy-connect-retirement.md).
Local build/tests do not establish real-provider E2E, deployed GitLab subject
equivalence, account continuity or live cutover.
