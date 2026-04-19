---
title: >-
  Manifesto Testing and Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - Manifesto/tests/common.rs
  - Manifesto/tests/project_api_tests.rs
  - Manifesto/tests/component_api_tests.rs
  - Manifesto/tests/member_api_tests.rs
  - Manifesto/tests/fixtures/db/mod.rs
  - Manifesto/setup/src/app.rs
summary: >-
  Manifesto-specific testing notes on top of RustyCog's shared harness, covering its DB-first fixture model, API suites, and the default no-SQS posture.
provenance:
  extracted: 0.84
  inferred: 0.09
  ambiguous: 0.07
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Manifesto Testing and Fixtures

This page narrows `[[projects/rustycog/references/rustycog-testing]]` to the way `[[projects/manifesto/manifesto]]` actually uses the shared harness, fixtures, and HTTP-level coverage.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-testing]]` explains the shared test server, migration hooks, JWT helpers, and fixture model that Manifesto builds on.
- `[[concepts/integration-testing-with-real-infrastructure]]` captures the broader pattern of using a real server plus real backing infrastructure instead of a mocked HTTP shell.

## Service-Specific Differences

- `ManifestoTestDescriptor` plugs into `rustycog-testing`, runs migrations up and down, reports `has_db() == true`, and keeps `has_sqs() == false` in the default harness.
- `setup_test_server()` boots the service through `build_and_run()`, then returns a real `TestFixture`, base URL, and `reqwest` client for end-to-end API tests.
- `project_api_tests.rs` covers project creation, read/detail/list, update/delete, lifecycle transitions, and the immediate owner-permission bootstrap that project creation is expected to grant.
- `component_api_tests.rs` covers add/get/list/update/remove flows plus the distinction between generic component permissions and component-instance UUID permissions.
- `member_api_tests.rs` covers member CRUD plus grant/revoke behavior for both generic resources and component-specific permission paths.
- Tests use real JWTs generated through `rustycog_testing::http::jwt::create_jwt_token()` and `#[serial]` execution so shared server and database setup remain deterministic.
- `DbFixtures` exposes reusable builders for projects, components, and members, plus helper combinations such as project-with-owner and project-with-component, so tests can focus on API intent rather than raw insert boilerplate.
- The project-detail tests assert that components are returned, but the live response still leaves `endpoint` and `access_token` unset, which matches the current API limitation rather than a richer provisioning contract. ^[ambiguous]

## Open Questions

- The default test harness does not exercise queue-backed publication even though the runtime can wire an event publisher, so event-path verification is still lighter than the HTTP and permission coverage. ^[ambiguous]
- Should the external component-service fallback path gain its own dedicated doubles or adapter-level tests, rather than being covered mainly through API scenarios and code inspection? ^[inferred]

## Sources

- [[projects/manifesto/manifesto]] - Service hub and current MVP framing.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - HTTP entrypoints exercised by these tests.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Lifecycle behavior validated by the project suite.
- [[projects/manifesto/concepts/component-instance-permissions]] - Permission model exercised by the component and member suites.
- [[concepts/integration-testing-with-real-infrastructure]] - Shared real-server testing pattern that Manifesto follows.
