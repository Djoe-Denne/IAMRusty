---
title: Using RustyCog Testing
category: skills
tags: [rustycog, testing, skills, visibility/internal]
sources:
  - rustycog/rustycog-testing/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
  - rustycog/rustycog-testing/src/wiremock/mod.rs
  - IAMRusty/tests/common.rs
  - Telegraph/tests/common.rs
  - Hive/tests/common.rs
  - Manifesto/tests/common.rs
summary: Workflow for using rustycog-testing to bootstrap reusable integration tests, prefixed service URLs, infrastructure-backed event tests, and wiremock HTTP fakes.
provenance:
  extracted: 0.87
  inferred: 0.08
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-25T10:13:00Z
---

# Using RustyCog Testing

Use this guide when setting up integration tests with `<!-- [[projects/rustycog/references/rustycog-testing]] -->`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests.
- Return a **service-prefixed** base URL from each service's local `tests/common.rs`: `/iam` for IAMRusty, `/telegraph` for Telegraph, `/hive` for Hive, and `/manifesto` for Manifesto. Test bodies should append route paths such as `/api/...` to that prefixed base URL instead of repeating the prefix at every call site.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior.
- For outbound HTTP collaborators, wrap the shared [[projects/rustycog/references/wiremock-mock-server-fixture]] in a typed `MockService` per collaborator and arrange responses with `mock_*` helpers — see [[skills/stubbing-http-with-wiremock]] for the recipe.
- For permission-gated routes (`with_permission_on`), construct [[projects/rustycog/references/openfga-mock-service]] in `setup_test_server` and return its handle alongside the test fixture so individual tests can arrange `mock_check_allow` / `mock_check_deny` per tuple. Set `openfga.cache_ttl_seconds = 0` in the test config so re-arranged decisions actually fire.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Using the raw origin returned by `rustycog_testing::setup_test_server()` directly in service tests. Wrap it once in the service-local helper with the same `SERVICE_PREFIX` used by runtime routing, otherwise tests will pass against paths that do not match microservice or monolith mode.
- Hard-coding `/api/...` against a bare origin in new test helpers. Keep the prefix centralized in `tests/common.rs` so moving between standalone and monolith runtime modes does not change individual tests.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Forgetting to reset state between tests when reusing shared server instances.
- Skipping `#[serial]` on tests that touch the wiremock fixture — the singleton listens on a fixed port and mocks are process-wide, so parallel tests will clobber each other.
- Asserting on a `403` from a permission-gated route without resetting the wiremock fake first when `setup_test_server` mounted a permissive `mock_check_any(true)` default. wiremock matches in registration order, so the catch-all wins. Call `openfga.reset().await` then mount your `mock_check_deny(...)` for the exact tuple under test.
- Asserting on a state change after a revoke/grant API call without setting `openfga.cache_ttl_seconds = 0`. The production `CachedPermissionChecker` (15s TTL) will serve the pre-revoke decision and the second request never reaches the wiremock fake.

## Sources

- [[projects/rustycog/references/rustycog-testing]]
- [[projects/aiforall/skills/running-aiforall-runtime-modes]]
- [[projects/aiforall/references/modular-monolith-runtime]]
- [[projects/rustycog/references/wiremock-mock-server-fixture]]
- [[projects/rustycog/references/openfga-mock-service]]
- [[skills/stubbing-http-with-wiremock]]
- [[skills/using-rustycog-permission]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/rustycog/rustycog]]
- [[projects/manifesto/references/manifesto-testing-and-fixtures]]
