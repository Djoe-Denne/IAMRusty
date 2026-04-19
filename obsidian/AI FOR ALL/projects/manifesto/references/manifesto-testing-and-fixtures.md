---
title: >-
  Manifesto Testing and Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - Manifesto/tests/common.rs
  - Manifesto/tests/public_acl_api_tests.rs
  - Manifesto/tests/component_acl_consistency_tests.rs
  - Manifesto/tests/component_service_client_tests.rs
  - Manifesto/tests/event_runtime_tests.rs
  - Manifesto/tests/project_api_tests.rs
  - Manifesto/tests/component_api_tests.rs
  - Manifesto/tests/member_api_tests.rs
  - Manifesto/tests/fixtures/db/mod.rs
  - Manifesto/setup/src/app.rs
  - rustycog/rustycog-http/tests/permission_middleware_tests.rs
summary: >-
  Manifesto-specific testing notes on top of RustyCog's shared harness, covering DB-backed API
  suites plus focused tests for auth, ACL consistency, fail-closed integrations, and apparatus runtime behavior.
provenance:
  extracted: 0.89
  inferred: 0.07
  ambiguous: 0.04
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-04-19T18:00:00Z
---

# Manifesto Testing and Fixtures

This page narrows `[[projects/rustycog/references/rustycog-testing]]` to the way `[[projects/manifesto/manifesto]]` actually uses the shared harness, fixtures, and focused remediation-era tests.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-testing]]` explains the shared test server, migration hooks, JWT helpers, and fixture model that Manifesto builds on.
- `[[concepts/integration-testing-with-real-infrastructure]]` captures the broader pattern of using a real server plus real backing infrastructure instead of a mocked HTTP shell.

## Service-Specific Differences

- `ManifestoTestDescriptor` plugs into `rustycog-testing`, runs migrations up and down, reports `has_db() == true`, and keeps `has_sqs() == false` in the default harness.
- `setup_test_server()` boots the service through `build_and_run()`, then returns a real `TestFixture`, base URL, and `reqwest` client for DB-backed API tests.
- `project_api_tests.rs`, `component_api_tests.rs`, and `member_api_tests.rs` still cover the main HTTP CRUD and permission surfaces.
- `rustycog-http/tests/permission_middleware_tests.rs` now includes signed-token rejection coverage, so the shared auth middleware is tested against tampered bearer tokens instead of only happy paths.
- `tests/public_acl_api_tests.rs` covers anonymous public-read permission behavior plus project-list filter forwarding at the service boundary.
- `tests/component_acl_consistency_tests.rs` covers fail-hard component-instance ACL synchronization on add/remove flows.
- `tests/component_service_client_tests.rs` covers fail-closed component-service behavior and bearer API-key usage.
- `tests/event_runtime_tests.rs` covers disabled queue bootstrap, enabled-config no-op fallback when no broker fixture exists, and `ComponentStatusProcessor` duplicate-delivery/stale-event idempotency plus state updates.
- Tests use real signed JWTs from `rustycog_testing::http::jwt::create_jwt_token()`.
- `DbFixtures` still provides reusable builders for projects, components, and members when DB-backed scenarios are useful.

## Notes

- Checked-in configs keep queues disabled by default, so event-path confidence currently comes from focused runtime tests rather than queue-backed end-to-end API suites.
- `ComponentResponse.endpoint` and `access_token` remain unset in current API behavior, and tests treat that as the present product boundary rather than a missing fixture detail.

## Open Questions

- If queue-backed CI becomes standard later, which Manifesto event paths deserve full broker-backed integration coverage instead of today's unit-level runtime checks?

## Sources

- [[projects/manifesto/manifesto]] - Service hub and current MVP framing.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - HTTP entrypoints exercised by these tests.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Lifecycle behavior validated by the project suite.
- [[projects/manifesto/concepts/component-instance-permissions]] - Permission model exercised by the component and member suites.
- [[concepts/integration-testing-with-real-infrastructure]] - Shared real-server testing pattern that Manifesto follows.
