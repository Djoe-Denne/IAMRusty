---
title: >-
  IAMRusty
category: project
tags: [iam, oauth, security, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
  - IAMRusty/docs/DATABASE_CONFIGURATION.md
  - IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - IAMRusty/docs/OAUTH_SECURITY_GUIDE.md
  - IAMRusty/docs/TESTING_GUIDE.md
  - IAMRusty/docs/FIXTURES_GUIDE.md
  - IAMRusty/docs/COMMAND_PATTERN.md
  - IAMRusty/docs/COMMAND_RETRY_CONFIGURATION.md
summary: >-
  IAMRusty is the identity service handling OAuth login, provider linking, JWTs, typed config, and real-infrastructure testing in a hexagonal Rust codebase.
provenance:
  extracted: 0.82
  inferred: 0.16
  ambiguous: 0.02
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:03:47.5107188Z
---

# IAMRusty

IAMRusty is the platform's identity and access service. It combines OAuth-based authentication, JWT issuance, provider linking, typed runtime configuration, and a strongly layered Rust architecture built around `[[concepts/hexagonal-architecture]]`.

## Key Ideas

- The service uses a provider-agnostic user model and documents both `[[concepts/oauth-provider-linking]]` and `[[concepts/oauth-state-and-csrf-protection]]` for safe multi-provider flows.
- Its token story is no longer just "JWTs exist"; the docs outline `[[concepts/jwt-secret-storage-abstraction]]` with HMAC/RSA support, key rotation, and JWKS exposure.
- Runtime wiring follows `[[concepts/structured-service-configuration]]`, including environment profiles, nested env overrides, structured DB config, random test ports, and read/write split support.
- Request orchestration relies on `[[concepts/command-registry-and-retry-policies]]` so retries, timeouts, logging, metrics, and error mapping stay centralized.
- Testing leans on `[[concepts/integration-testing-with-real-infrastructure]]` and `[[skills/testing-rust-services-with-fixtures]]` rather than unit-only mocking.
- The service still fits into the wider `[[concepts/event-driven-microservice-platform]]` by publishing user-related events that downstream services such as `[[entities/telegraph]]` can consume.

## Open Questions

- The exact contents of the `iam-events` contract crate are still outside the wiki.
- The docs describe several secret-storage and retry-policy options, but the exact production standard across environments is only partially visible. ^[ambiguous]

## Sources

- [[references/iamrusty-service]] — Capabilities, configuration, and architecture summary
- [[references/iamrusty-runtime-and-security]] — Runtime config, JWT, and OAuth hardening guides
- [[references/iamrusty-testing-and-fixtures]] — Integration-test and fixture-system guides
- [[references/iamrusty-command-execution]] — Command registry and retry-policy guides
