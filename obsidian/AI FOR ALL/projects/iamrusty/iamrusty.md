---
title: IAMRusty
category: project
tags: [iam, oauth, security, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
  - IAMRusty/docs/API_REFERENCE.md
  - IAMRusty/domain/src/entity/events.rs
  - IAMRusty/setup/src/app.rs
  - IAMRusty/http/src/lib.rs
summary: IAMRusty is a Rust IAM service that combines OAuth, email/password auth, JWTs, provider linking, and queue-backed identity events behind a hexagonal architecture.
provenance:
  extracted: 0.74
  inferred: 0.18
  ambiguous: 0.08
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-19T11:13:11Z
---

# IAMRusty

## Indexes

- [[projects/iamrusty/concepts/index]] — concepts
- [[projects/iamrusty/skills/index]] — skills
- [[projects/iamrusty/references/index]] — references

`IAMRusty` is the identity service in the AIForAll workspace. It combines OAuth login, email/password authentication, registration completion, JWT and refresh-token issuance, provider linking, and queue-backed auth events behind `[[projects/iamrusty/concepts/hexagonal-architecture]]` and a shared `[[projects/rustycog/rustycog]]` runtime stack.

## Key Ideas

- The service is split across domain, application, infrastructure, HTTP, configuration, and setup crates, with `setup/src/app.rs` acting as the composition root for repositories, token services, provider clients, and the command registry.
- Authentication is intentionally dual-mode: unauthenticated users enter OAuth login, while authenticated users can attach extra providers through `[[projects/iamrusty/concepts/oauth-provider-linking]]`.
- Runtime behavior depends on `[[concepts/structured-service-configuration]]`, including environment-specific TOML files, cached random ports in tests, queue settings, and JWT secret resolution.
- Command orchestration is centralized through `[[concepts/command-registry-and-retry-policies]]`, so handlers delegate business work instead of embedding retries, logging, and error mapping directly.
- The service relies on `[[concepts/integration-testing-with-real-infrastructure]]` for end-to-end confidence, using real databases, HTTP servers, fixtures, and optional queue-backed checks.
- IAMRusty uses `iam-events` as its domain-event contract surface, while `[[projects/rustycog/references/rustycog-events]]` provides the queue transport and publisher runtime.
- The published API and the current implementation are close but not identical: the docs still describe some older route names and payload shapes, while the live route table in `http/src/lib.rs` exposes separate login, link, and relink endpoints. ^[ambiguous]

## Related

- [[projects/iamrusty/references/iamrusty-service]] - Code-backed overview of the crate layout, route surface, and runtime wiring.
- [[projects/iamrusty/references/iamrusty-entity-model]] - Identity-side entities such as users, emails, provider links, and token artifacts.
- [[projects/iamrusty/references/iamrusty-runtime-and-security]] - Configuration, JWT, queue, TLS, and OAuth hardening details.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] - Public and authenticated HTTP flows, including registration completion and password reset.
- [[projects/iamrusty/references/iamrusty-command-execution]] - How the command registry wraps the service's use cases.
- [[projects/iamrusty/references/iamrusty-testing-and-fixtures]] - Test server, database fixture, and Kafka-backed validation patterns.
- [[projects/iamrusty/skills/testing-rust-services-with-fixtures]] - Preferred workflow for building IAM-style integration tests.
- [[projects/iamrusty/skills/extending-iamrusty-with-oauth-providers]] - End-to-end checklist for adding another provider safely.
- [[projects/rustycog/references/index]] - Shared RustyCog crate map backing the runtime assumptions used across IAMRusty pages.

## Open Questions

- The docs describe some security and deployment behavior more strongly than the current code exposes, especially around OAuth state expiry and callback URI handling. ^[ambiguous]
- `README.md` and testing docs still reference `docs/TEST_DATABASE_GUIDE.md`, but that file is not present in the current source set. ^[ambiguous]

## Sources

- [[projects/iamrusty/references/iamrusty-service]]
- [[projects/iamrusty/references/iamrusty-runtime-and-security]]
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]]
- [[projects/iamrusty/references/iamrusty-command-execution]]
- [[projects/iamrusty/references/iamrusty-testing-and-fixtures]]
