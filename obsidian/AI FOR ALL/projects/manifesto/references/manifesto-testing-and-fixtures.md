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
  - Manifesto/config/test.toml
  - rustycog/rustycog-testing/src/permission/service.rs
summary: >-
  Manifesto HTTP ITs use TestOpenFga, not shared WireMock Check. Component-catalog stubs use MockServerFixture::isolated(). Leftover FGA without a DB member is 403.
provenance:
  extracted: 0.85
  inferred: 0.10
  ambiguous: 0.05
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-09-09T16:45:00Z
---

# Manifesto Testing and Fixtures

This page narrows `[[projects/rustycog/references/rustycog-testing]]` to the way `[[projects/manifesto/manifesto]]` actually uses the shared harness, fixtures, and focused remediation-era tests.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-testing]]` explains the shared test server, migration hooks, JWT helpers, and fixture model that Manifesto builds on.
- `[[concepts/integration-testing-with-real-infrastructure]]` captures the broader pattern of using a real server plus real backing infrastructure instead of a mocked HTTP shell.

## Service-Specific Differences

- `ManifestoTestDescriptor` plugs into `rustycog-testing`, runs migrations up and down, reports `has_db() == true`, and keeps `has_sqs() == false` in the default harness.
- `setup_test_server()` boots the service through `build_and_run()`. OpenFGA **Check** in HTTP ITs uses [[projects/rustycog/references/openfga-real-testcontainer-fixture]] (`TestOpenFga`, default deny). Do not stub Check on the shared WireMock listener.
- Component-catalog HTTP uses [[projects/rustycog/references/isolated-wiremock-fixture]] so parallel tests do not collide on `:3000`.
- `project_api_tests.rs`, `component_api_tests.rs`, `member_api_tests.rs`, `project_visibility_acl_api_tests.rs`, and `correction_regression_tests.rs` cover CRUD, join, Suspended, restore, and leftover-FGA 403.
- Arranging only `grant_write` without an active member is **403** — [[projects/manifesto/concepts/immediate-membership-acl]].
- There is no Manifesto suite that boots sentinel-sync against a real broker for AuthZ e2e. Transport/reconcile tests live in `sentinel-sync`.
- `rustycog-http/tests/permission_middleware_tests.rs` now includes signed-token rejection coverage, so the shared auth middleware is tested against tampered bearer tokens instead of only happy paths.
- `tests/public_acl_api_tests.rs` covers anonymous public-read permission behavior plus project-list filter forwarding at the service boundary.
- `tests/component_acl_consistency_tests.rs` covers fail-hard component-instance ACL synchronization on add/remove flows.
- `tests/component_service_client_tests.rs` covers fail-closed component-service behavior and bearer API-key usage.
- `tests/event_runtime_tests.rs` covers disabled queue bootstrap, enabled-config no-op fallback when no broker fixture exists, and `ComponentStatusProcessor` duplicate-delivery/stale-event idempotency plus state updates.
- Tests use real signed JWTs from `rustycog_testing::http::jwt::create_jwt_token()`.
- `DbFixtures` still provides reusable builders for projects, components, and members when DB-backed scenarios are useful.

## OpenFGA in HTTP ITs

Use `TestOpenFga` (`allow` / `deny`). `OpenFgaMockService` remains crate-level only. `cache_ttl_seconds = 0` in `Manifesto/config/test.toml` still applies if a cached checker is ever wired.

Component-catalog stubs: `MockServerFixture::isolated()` — [[projects/rustycog/references/isolated-wiremock-fixture]].

The trailing-UUID `ResourceRef` quirk still applies when middleware binds the last path UUID.

## Notes

- Checked-in configs keep queues disabled by default.
- `ComponentResponse.endpoint` and `access_token` remain unset.

## Open Questions

- If queue-backed CI becomes standard later, which Manifesto event paths deserve full broker-backed integration coverage?

## Sources

- [[projects/manifesto/manifesto]]
- [[projects/manifesto/references/manifesto-api-and-permission-flows]]
- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]]
- [[projects/rustycog/references/isolated-wiremock-fixture]]
- [[skills/stubbing-http-with-wiremock]]
