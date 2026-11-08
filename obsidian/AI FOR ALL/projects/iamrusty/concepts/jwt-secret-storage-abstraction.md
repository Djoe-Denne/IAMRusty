---
title: JWT Secret Storage Abstraction
category: concepts
tags: [security, jwt, auth, visibility/internal]
sources:
  - services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - services/IAMRusty/configuration/src/lib.rs
  - services/IAMRusty/setup/src/app.rs
  - docs/guides/iam-oauthconnect-securite-sources.md
  - docs/adr/0311-crypto-privee-deleguee-aux-providers.md
summary: IAMRusty resolves JWT signing material from configurable secret backends before building token services, registration tokens, and JWKS output.
provenance:
  extracted: 0.78
  inferred: 0.14
  ambiguous: 0.08
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-11-08
---

# JWT Secret Storage Abstraction

[[projects/iamrusty/iamrusty]] sépare cryptographie JWT et stockage des secrets. La description d'avril ci-dessous concernait les premiers backends locaux ; elle n'est pas le lifecycle courant des clés d'accès déléguées.

## État ciblé vérifié le8novembre

`resolve_access_signing` choisit Transit configuré, sinon remote configuré, sinon chemin local soumis à ses conditions. Une erreur du provider choisi n'autorise pas de repli local. Le runtime délégué n'a pas à recevoir le privé : les `JwtKeyPair` de ce chemin portent un privé vide. Les branches historiques de `SecretStorage` ne décrivent pas à elles seules ces ports de signature.

Le public et son binding sont vérifiés à l'émission ; le publisher JWKS HTTP suit le registry writer, pas simplement un assemblage de la paire locale. La rotation organisation inspectée est Transit, via Hive→IAM ; le signer distant n'implique pas une capacité Rotate. Pending/Active/Retiring et TTL JWT IAM+60s sont détaillés dans [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]].

OpenBao **KVv2** des credentials Connect, OpenBao **Transit** de signature IAM, HMAC S2S, clés privées fournisseur OIDC et certificat TLS ne sont pas interchangeables. La lecture ne révèle ni credentials ni coffre réel, ne prouve pas un déploiement et n'autorise aucune rotation. Sources vérifiées : [S07 et S11–S14](../../../../../docs/guides/iam-oauthconnect-securite-sources.md).

## Photographie historique du23avril2026 — pas la liste des capacités actuelles

- `JwtConfig` resolves a `SecretStorage` value into a concrete `JwtSecret`, then converts that into either HS256 or RS256 runtime configuration.
- The current code supports plain-text HMAC secrets and PEM-backed RSA key pairs, with placeholder branches for Vault and GCP Secret Manager that are not yet implemented.
- `setup/src/app.rs` uses the resolved algorithm to create both `JwtTokenService` and `RegistrationTokenServiceImpl`, so the same cryptographic source shapes regular auth and registration-completion flows.
- Both constructors enforce RS256 in production via compile-time guards (`#[cfg(not(feature = "test-relaxed-jwt"))]` on the `if` in `RegistrationTokenServiceImpl::new` and an `assert!` in `JwtTokenService::with_refresh_expiration`); the `with_hmac` constructor on `JwtTokenService` is gated on the same feature so it does not exist in release binaries. The `iam-service` test build activates the feature via its dev-dep on `iam-infra` so the in-tree HS256 `test.toml` boots — see [[projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation]] and [[concepts/test-only-cargo-feature-relaxation]].
- The HTTP layer exposes a JWKS endpoint, which makes the public verification keys part of the service contract rather than an out-of-band deployment artifact.
- The docs and examples still use names such as `[jwt.secret_storage]`, `PlainText`, and `PemFile`, while the current code and TOML files use `[jwt.secret]`, lowercase serde tags like `pem_file`, and the field name `value`. ^[ambiguous]
- The code also defaults access tokens to 15 minutes and refresh tokens to 30 days unless config overrides them.

## Questions historiques, à recouper avec l'actualisation

- Vault and GCP secret backends are documented architecturally, but the current implementation returns not-implemented errors for those branches. ^[ambiguous]
- The docs often assume broader operational patterns for key rotation and zero-downtime migration than are directly visible in this repo alone. ^[ambiguous]

## Sources

- [[projects/iamrusty/iamrusty]] - Service that consumes the abstraction.
- [[projects/iamrusty/references/iamrusty-runtime-and-security]] - Runtime config, JWKS, TLS, and deployment context.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] - API surface that exposes token and JWKS behavior.
- [[projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation]] - Production RS256 guards and how tests relax them.
- [[concepts/test-only-cargo-feature-relaxation]] - Generic Cargo-feature pattern used to gate the constructors.
- <!-- [[concepts/structured-service-configuration]] --> - Config layer that resolves secret backends.
