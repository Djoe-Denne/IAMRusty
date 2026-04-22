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
summary: Workflow for using rustycog-testing to bootstrap reusable integration tests, infrastructure-backed event tests, and wiremock-backed HTTP collaborator fakes.
provenance:
  extracted: 0.86
  inferred: 0.08
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-22T16:20:59Z
---

# Using RustyCog Testing

Use this guide when setting up integration tests with `<!-- [[projects/rustycog/references/rustycog-testing]] -->`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior.
- For outbound HTTP collaborators, wrap the shared [[projects/rustycog/references/wiremock-mock-server-fixture]] in a typed `MockService` per collaborator and arrange responses with `mock_*` helpers — see [[skills/stubbing-http-with-wiremock]] for the recipe.
- For permission-gated routes (`with_permission_on`), construct [[projects/rustycog/references/openfga-mock-service]] in `setup_test_server` and return its handle alongside the test fixture so individual tests can arrange `mock_check_allow` / `mock_check_deny` per tuple. Set `openfga.cache_ttl_seconds = 0` in the test config so re-arranged decisions actually fire.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Forgetting to reset state between tests when reusing shared server instances.
- Skipping `#[serial]` on tests that touch the wiremock fixture — the singleton listens on a fixed port and mocks are process-wide, so parallel tests will clobber each other.
- Asserting on a `403` from a permission-gated route without resetting the wiremock fake first when `setup_test_server` mounted a permissive `mock_check_any(true)` default. wiremock matches in registration order, so the catch-all wins. Call `openfga.reset().await` then mount your `mock_check_deny(...)` for the exact tuple under test.
- Asserting on a state change after a revoke/grant API call without setting `openfga.cache_ttl_seconds = 0`. The production `CachedPermissionChecker` (15s TTL) will serve the pre-revoke decision and the second request never reaches the wiremock fake.

## Sources

- [[projects/rustycog/references/rustycog-testing]]
- [[projects/rustycog/references/wiremock-mock-server-fixture]]
- [[projects/rustycog/references/openfga-mock-service]]
- [[skills/stubbing-http-with-wiremock]]
- [[skills/using-rustycog-permission]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/rustycog/rustycog]]
- [[projects/manifesto/references/manifesto-testing-and-fixtures]]
