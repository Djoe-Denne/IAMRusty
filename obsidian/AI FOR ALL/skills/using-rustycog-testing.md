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
  - IAMRusty/tests/sqs_event_routing_tests.rs
  - Telegraph/tests/common.rs
  - Hive/tests/common.rs
  - Hive/tests/sqs_event_routing_tests.rs
  - Manifesto/tests/common.rs
  - Manifesto/tests/sqs_event_routing_tests.rs
summary: Workflow for using rustycog-testing to bootstrap service tests, prefixed URLs, SQS fanout fixtures, real infrastructure, and wiremock fakes.
provenance:
  extracted: 0.88
  inferred: 0.08
  ambiguous: 0.04
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-08-31T09:45:00Z
---

# Using RustyCog Testing

Use this guide when setting up integration tests with `<!-- [[projects/rustycog/references/rustycog-testing]] -->`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client. Hive, Telegraph, and Manifesto also return [[projects/rustycog/references/openfga-real-testcontainer-fixture|`TestOpenFga`]]. Default is **deny** — each test calls `allow` / `allow_all` / `allow_wildcard`. IAM keeps `has_openfga() == false`.
- Return a **service-prefixed** base URL from each service's local `tests/common.rs`: `/iam` for IAMRusty, `/telegraph` for Telegraph, `/hive` for Hive, and `/manifesto` for Manifesto. Test bodies should append route paths such as `/api/...` to that prefixed base URL instead of repeating the prefix at every call site.
- One `#[path = "fixtures/mod.rs"]` in `tests/common.rs` only. Other files do `mod common`.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior; keep shared `test.toml` queue settings `enabled = false` unless the whole suite genuinely needs transport.
- For SQS fanout tests, configure all destination queues in `SqsConfig`; the LocalStack fixture creates every configured physical queue and named-queue helpers let tests assert each destination independently.
- For producer-side SQS routing tests, use a distinct `default_queues` fallback plus explicit `[queue.queues]` mappings. Drain every relevant queue before the action, then assert the event appears via `wait_for_messages_from_queue(mapped_queue, ...)` and does **not** appear via `get_all_messages_from_queue(default_queue, ...)`.
- Prefer a dedicated routing-test descriptor with `has_sqs() == true` and a test-binary env override such as `HIVE_QUEUE__ENABLED=true`, `IAM_QUEUE__ENABLED=true`, or `MANIFESTO_QUEUE__ENABLED=true`. The default descriptor should keep `has_sqs() == false` so normal HTTP/API tests do not pay LocalStack startup cost.
- Keep named-queue routing tests transport-heavy and `#[serial]`. `Hive/tests/sqs_event_routing_tests.rs`, `IAMRusty/tests/sqs_event_routing_tests.rs`, and `Manifesto/tests/sqs_event_routing_tests.rs` are the reference shapes for HTTP action -> domain event -> mapped LocalStack queue assertions.
- For outbound HTTP collaborators, wrap the shared [[projects/rustycog/references/wiremock-mock-server-fixture]] in a typed `MockService` per collaborator and arrange responses with `mock_*` helpers — see [[skills/stubbing-http-with-wiremock]] for the recipe.
- Opt in with `has_openfga() == true` and `openfga_authorization_model_json()`. Keep `openfga.cache_ttl_seconds = 0` so grant→revoke is not served from `CachedPermissionChecker`. `OpenFgaMockService` is crate-level only — not for Hive / Telegraph / Manifesto HTTP ITs.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Using the raw origin returned by `rustycog_testing::setup_test_server()` directly in service tests. Wrap it once in the service-local helper with the same `SERVICE_PREFIX` used by runtime routing, otherwise tests will pass against paths that do not match microservice or monolith mode.
- Hard-coding `/api/...` against a bare origin in new test helpers. Keep the prefix centralized in `tests/common.rs` so moving between standalone and monolith runtime modes does not change individual tests.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Checking only the fallback queue in fanout or routing tests. Use named-queue reads when one event should land in multiple destination queues, and assert the fallback queue is empty when a per-event mapping should bypass it.
- Forgetting to reset state between tests when reusing shared server instances.
- Skipping `#[serial]` on tests that touch wiremock **or** `TestOpenFga`.
- Expecting a permissive OpenFGA default. A missing `allow` is a 403.
- Using `OpenFgaMockService` in a service IT that already sets `has_openfga() == true`.
- Asserting on a state change after revoke/grant without `openfga.cache_ttl_seconds = 0`.
- Re-declaring `#[path = "fixtures/mod.rs"]` in each `*_test.rs`.

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
