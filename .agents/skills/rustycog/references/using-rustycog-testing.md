# Using RustyCog Testing

Use this guide when setting up integration tests with `rustycog-testing`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests. Hive, Telegraph, and Manifesto return a `TestOpenFga` handle (real `openfga/openfga` testcontainer). The harness writes **no** permissive default — default is deny. Each test calls `openfga.allow(subject, action, resource)` (or `allow_all` / `allow_wildcard`) for every tuple the route guard will Check. IAM keeps `has_openfga() == false`.
- Return a **service-prefixed** base URL from each service's local `tests/common.rs`: `/iam` for IAMRusty, `/telegraph` for Telegraph, `/hive` for Hive, and `/manifesto` for Manifesto. Test bodies should append route paths such as `/api/...` to that prefixed base URL instead of repeating the prefix at every call site.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior; keep shared `test.toml` queue settings `enabled = false` unless the whole suite genuinely needs transport.
- For SQS producer routing tests, configure every physical queue in `SqsConfig`, use the LocalStack fixture's named-queue helpers (`wait_for_messages_from_queue`, `get_all_messages_from_queue`), and assert both the positive destination and the negative fallback queue. `Hive/tests/sqs_event_routing_tests.rs`, `IAMRusty/tests/sqs_event_routing_tests.rs`, and `Manifesto/tests/sqs_event_routing_tests.rs` are the reference shapes.
- Prefer a dedicated routing-test descriptor with `has_sqs() == true` and a test-binary env override such as `HIVE_QUEUE__ENABLED=true`, `IAM_QUEUE__ENABLED=true`, or `MANIFESTO_QUEUE__ENABLED=true`. Default descriptors should keep `has_sqs() == false` so normal HTTP/API tests avoid LocalStack startup.
- For outbound HTTP collaborators, wrap `rustycog_testing::wiremock::MockServerFixture` in a typed per-collaborator service. See the `creating-wiremock-fixtures` skill at `.cursor/skills/creating-wiremock-fixtures/SKILL.md`.
- Opt in with `has_openfga() == true` plus `openfga_authorization_model_json()` (`include_str!("../../openfga/model.json")`). `TestFixture::new` starts the singleton container, creates a store, uploads the model, and publishes `_OPENFGA__*` env vars before the app boots.
- One `#[path = "fixtures/mod.rs"]` lives in `tests/common.rs` only (Telegraph `duplicate_mod`). Other test files do `mod common`.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Permission denial test pattern

Service ITs talk to a **real** OpenFGA. Default is deny. For a happy path, write the tuple; for a `403`, write nothing (or `deny` after a prior `allow`):

```rust
openfga
    .allow(
        Subject::new(member_id),
        Permission::Admin,
        ResourceRef::new("project", component.id()), // trailing UUID in the path
    )
    .await?;
// later, to flip:
openfga
    .deny(Subject::new(member_id), Permission::Admin, resource)
    .await?;
```

`deny` tolerates a missing tuple. Keep `openfga.cache_ttl_seconds = 0` in test config and skip `CachedPermissionChecker` when 0, otherwise grant→revoke serves a stale allow. Tests that touch `TestOpenFga` stay `#[serial]` — the container is process-global.

`OpenFgaMockService` / `mock_check_*` remain in `rustycog-testing` for crate-level fakes. Do **not** use them for Hive / Telegraph / Manifesto HTTP ITs.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Using the raw origin returned by `rustycog_testing::setup_test_server()` directly in service tests. Wrap it once in the service-local helper with the same `SERVICE_PREFIX` used by runtime routing, otherwise tests pass against paths that do not match microservice or monolith mode.
- Hard-coding `/api/...` against a bare origin in new test helpers. Keep the prefix centralized in `tests/common.rs` so moving between standalone and monolith runtime modes does not change individual tests.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Checking only the default queue in SQS routing tests. For mapped events, read the explicit destination queue by name and verify the fallback queue stayed empty.
- Forgetting to reset state between tests when reusing shared server instances.
- Skipping `#[serial]` on tests that touch wiremock **or** `TestOpenFga` — both are process-wide singletons.
- Expecting a permissive OpenFGA default. The harness writes no tuples; a missing `allow` is a 403.
- Using `OpenFgaMockService` in a service IT that already sets `has_openfga() == true` — two authorization paths will disagree.
- Asserting on a state change after revoke/grant without `openfga.cache_ttl_seconds = 0`. The production cache serves the pre-revoke decision.
- Re-declaring `#[path = "fixtures/mod.rs"]` in each `*_test.rs` (Clippy `duplicate_mod`).

## Source files

- `rustycog/rustycog-testing/src/lib.rs`
- `rustycog/rustycog-testing/src/common/test_server.rs`
- `rustycog/rustycog-testing/src/common/kafka_testcontainer.rs`
- `rustycog/rustycog-testing/src/common/sqs_testcontainer.rs`
- `rustycog/rustycog-testing/src/wiremock/mod.rs` — `MockServerFixture` singleton.
- `rustycog/rustycog-testing/src/common/openfga_testcontainer.rs` — `TestOpenFga` (`allow` / `deny` / `allow_all` / `allow_wildcard`).
- `rustycog/rustycog-testing/src/permission/service.rs` — `OpenFgaMockService` (crate-level fake only).
- `IAMRusty/tests/common.rs` — wraps the raw server origin with `/iam`.
- `IAMRusty/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for IAM producer events routed to Telegraph.
- `Telegraph/tests/common.rs` — wraps the raw server origin with `/telegraph`.
- `Hive/tests/common.rs` — wraps the raw server origin with `/hive`.
- `Hive/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for Hive producer events.
- `Manifesto/tests/common.rs` — wraps the raw server origin with `/manifesto` and returns the service test fixture, prefixed base URL, HTTP client, OpenFGA handle, and component-service mock.
- `Manifesto/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for Manifesto producer events routed to SentinelSync.
- `Manifesto/tests/component_api_tests.rs` — see tests 4 / 5 / 6 for the deny / multi-tuple / phase-flip patterns.

## Key helpers

- `setup_test_server()` — reusable prefixed base URL + HTTP client; Hive/Telegraph/Manifesto also return `TestOpenFga`.
- `TestOpenFga::allow` / `deny` / `allow_all` / `allow_wildcard` / `write_tuple` — real relationship tuples for route guards.
- `MockServerFixture::reset()` — wipe HTTP stubs for mid-test re-arrangement (outbound collaborators, not OpenFGA Check).
- Kafka/SQS testcontainer helpers — opt-in real-transport coverage. For SQS routing, prefer `TestSqs::wait_for_messages_from_queue` and `TestSqs::get_all_messages_from_queue` over primary/default-queue helpers.
