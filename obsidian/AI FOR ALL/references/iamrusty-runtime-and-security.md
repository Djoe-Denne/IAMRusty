---
title: IAMRusty Runtime and Security
category: references
tags: [reference, configuration, security, visibility/internal]
sources:
  - IAMRusty/docs/DATABASE_CONFIGURATION.md
  - IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - IAMRusty/docs/OAUTH_SECURITY_GUIDE.md
  - IAMRusty/docs/KAFKA_INTEGRATION.md
  - IAMRusty/docs/DEPLOYMENT_GUIDE.md
  - IAMRusty/configuration/src/lib.rs
  - IAMRusty/config/default.toml
  - IAMRusty/config/test.toml
  - IAMRusty/http/src/oauth_state.rs
summary: Source-backed summary of IAMRusty's config loader, JWT secret resolution, queue wiring, TLS guidance, and security-related doc-code drift.
provenance:
  extracted: 0.72
  inferred: 0.15
  ambiguous: 0.13
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T17:46:37.6929647Z
---

# IAMRusty Runtime and Security

These sources describe how `[[projects/iamrusty/iamrusty]]` is configured and secured at runtime: environment selection, database and queue setup, JWT signing, TLS deployment, and OAuth callback hardening.

## Key Ideas

- The real config loader uses the `IAM` prefix and one typed `AppConfig` that includes server, database, OAuth, JWT, logging, command, queue, and legacy Kafka sections.
- Structured database config supports nested credentials, read replicas, and cached random ports, which are especially important in the test environment.
- JWT behavior is driven by resolved secret storage, not by hardcoded algorithms: the runtime can build HS256 or RS256 token services and surface public verification data through JWKS.
- Current config files show PEM-backed JWT keys in both default and test TOMLs, while the code also keeps a plain-text HMAC branch for compatibility and future non-PEM secret backends.
- The runtime now builds queue-backed publishers from `config.queue`, but docs still discuss local Kafka configuration and legacy Kafka-specific entry points alongside that newer queue abstraction. ^[ambiguous]
- The OAuth security guide describes timestamped, expiring state and exact redirect validation, while the current `http/src/oauth_state.rs` only stores operation plus nonce and the callback handler hardcodes local redirect URIs. ^[ambiguous]

## Open Questions

- Operator-facing docs mix `APP_` and `IAM_` env prefixes, so deployment instructions and live config behavior are not fully aligned. ^[ambiguous]
- The docs often show `[jwt.secret_storage]` examples, but the actual config files and loader use `[jwt.secret]` plus lowercase serde tags like `pem_file`. ^[ambiguous]

## Sources

- [[projects/iamrusty/iamrusty]] - Service whose runtime is being configured.
- [[concepts/structured-service-configuration]] - Main concept distilled from these sources.
- [[concepts/jwt-secret-storage-abstraction]] - JWT-specific secret-resolution pattern.
- [[concepts/oauth-state-and-csrf-protection]] - OAuth hardening behavior and implementation drift.
