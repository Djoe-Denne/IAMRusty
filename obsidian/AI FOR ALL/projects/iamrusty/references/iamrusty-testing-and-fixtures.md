---
title: IAMRusty Testing and Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - IAMRusty/docs/TESTING_GUIDE.md
  - IAMRusty/docs/FIXTURES_GUIDE.md
  - IAMRusty/docs/KAFKA_EVENT_TESTING_GUIDE.md
  - IAMRusty/tests/fixtures/db/mod.rs
  - IAMRusty/tests/signup_kafka.rs
summary: IAMRusty-specific testing notes layered on top of RustyCog's shared harness, focusing on auth fixtures, provider mocks, and optional queue or Kafka-backed validation.
provenance:
  extracted: 0.84
  inferred: 0.1
  ambiguous: 0.06
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-19T12:08:26.9393504Z
---

# IAMRusty Testing and Fixtures

This page narrows `[[projects/rustycog/references/rustycog-testing]]` to the way `[[projects/iamrusty/iamrusty]]` actually validates auth behavior, provider flows, and optional event publication.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-testing]]` explains the shared test server, migration hooks, JWT helpers, and base fixture model that IAMRusty reuses.
- `[[concepts/integration-testing-with-real-infrastructure]]` captures the broader real-infrastructure testing pattern that IAMRusty applies to auth flows.

## Service-Specific Differences

- Integration tests are built around a shared server/database fixture plus `#[serial]` execution so cleanup, state, and runtime setup stay deterministic.
- `DbFixtures` exposes fluent builders for users, emails, provider tokens, refresh tokens, verification records, and password-reset tokens, along with higher-level helpers for common auth scenarios.
- GitHub and GitLab service fixtures mock external OAuth APIs while still letting the service execute real HTTP handlers and persistence logic.
- Kafka validation exists as a real container-backed test that consumes published events, but it is intentionally `#[ignore]` because of Docker, startup time, and environment requirements.
- The Kafka test also confirms that queue and event behavior are wired through the same config-driven runtime used by the service instead of through a special test-only code path.
- Shared harness behavior now largely follows `[[projects/rustycog/references/rustycog-testing]]`, while IAMRusty keeps service-specific fixtures and auth-flow assertions.
- The testing docs still reference some local utilities that have since moved into `rustycog-testing`, so parts of the published guide lag behind the current fixture module layout. ^[ambiguous]

## Open Questions

- Queue-backed coverage is present but incomplete, because Kafka tests are optional and the default test queue config is disabled. ^[ambiguous]
- The missing `docs/TEST_DATABASE_GUIDE.md` means part of the intended testing narrative is absent from the current repo snapshot. ^[ambiguous]

## Sources

- [[projects/iamrusty/iamrusty]] - Service whose flows are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Distilled testing concept from these sources.
- [[projects/iamrusty/skills/testing-rust-services-with-fixtures]] - Actionable workflow built from the same patterns.
- [[projects/iamrusty/references/iamrusty-runtime-and-security]] - Config and queue context behind test setup.
- [[projects/rustycog/references/rustycog-testing]] - Shared test-runtime layer IAMRusty now builds on.
