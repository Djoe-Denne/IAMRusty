# Using RustyCog Testing

Use this guide when setting up integration tests with `rustycog-testing`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests. For services that wire `rustycog-permission`, also return an `OpenFgaMockService` handle from the harness so individual tests can arrange `Check` decisions per tuple.
- Return a **service-prefixed** base URL from each service's local `tests/common.rs`: `/iam` for IAMRusty, `/telegraph` for Telegraph, `/hive` for Hive, and `/manifesto` for Manifesto. Test bodies should append route paths such as `/api/...` to that prefixed base URL instead of repeating the prefix at every call site.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior; keep shared `test.toml` queue settings `enabled = false` unless the whole suite genuinely needs transport.
- For SQS producer routing tests, configure every physical queue in `SqsConfig`, use the LocalStack fixture's named-queue helpers (`wait_for_messages_from_queue`, `get_all_messages_from_queue`), and assert both the positive destination and the negative fallback queue. `Hive/tests/sqs_event_routing_tests.rs`, `IAMRusty/tests/sqs_event_routing_tests.rs`, and `Manifesto/tests/sqs_event_routing_tests.rs` are the reference shapes.
- Prefer a dedicated routing-test descriptor with `has_sqs() == true` and a test-binary env override such as `HIVE_QUEUE__ENABLED=true`, `IAM_QUEUE__ENABLED=true`, or `MANIFESTO_QUEUE__ENABLED=true`. Default descriptors should keep `has_sqs() == false` so normal HTTP/API tests avoid LocalStack startup.
- For outbound HTTP collaborators, wrap `rustycog_testing::wiremock::MockServerFixture` in a typed per-collaborator service. See the `creating-wiremock-fixtures` skill at `.cursor/skills/creating-wiremock-fixtures/SKILL.md`.
- For permission-gated routes, use `rustycog_testing::permission::OpenFgaFixtures::service().await` and set `openfga.cache_ttl_seconds = 0` in the test config so re-arranged decisions actually fire mid-test.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Permission denial test pattern

When a test asserts a `403`/`deny` from a permission-gated route, do not rely on the harness's permissive `mock_check_any(true)` default. Reset and mount a per-tuple deny stub so the OpenFGA fake actually returns false for the exact `(subject, action, resource)` the route guard will Check:

```rust
openfga.reset().await;
openfga
    .mock_check_deny(
        Subject::new(member_id),
        Permission::Admin,
        ResourceRef::new("project", component.id()), // trailing UUID in the path
    )
    .await;
```

For grant ➜ revoke ➜ deny shapes, repeat the reset between phases and re-mount fresh stubs. Requires `openfga.cache_ttl_seconds = 0` in the test config; otherwise the `CachedPermissionChecker` (15s TTL by default) serves the stale allow from the first request.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Using the raw origin returned by `rustycog_testing::setup_test_server()` directly in service tests. Wrap it once in the service-local helper with the same `SERVICE_PREFIX` used by runtime routing, otherwise tests pass against paths that do not match microservice or monolith mode.
- Hard-coding `/api/...` against a bare origin in new test helpers. Keep the prefix centralized in `tests/common.rs` so moving between standalone and monolith runtime modes does not change individual tests.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Checking only the default queue in SQS routing tests. For mapped events, read the explicit destination queue by name and verify the fallback queue stayed empty.
- Forgetting to reset state between tests when reusing shared server instances.
- Skipping `#[serial]` on tests that touch the wiremock fixture — the singleton listens on a fixed port and mocks are process-wide.
- Asserting on a `403` from a permission-gated route without resetting the wiremock fake first when the harness mounted a permissive default. wiremock matches in registration order; the catch-all wins.
- Asserting on a state change after a revoke/grant API call without setting `openfga.cache_ttl_seconds = 0`. The production cache serves the pre-revoke decision and the second request never reaches the wiremock fake.

## Source files

- `rustycog/rustycog-testing/src/lib.rs`
- `rustycog/rustycog-testing/src/common/test_server.rs`
- `rustycog/rustycog-testing/src/common/kafka_testcontainer.rs`
- `rustycog/rustycog-testing/src/common/sqs_testcontainer.rs`
- `rustycog/rustycog-testing/src/wiremock/mod.rs` — `MockServerFixture` singleton.
- `rustycog/rustycog-testing/src/permission/service.rs` — `OpenFgaMockService` for permission-gated routes.
- `IAMRusty/tests/common.rs` — wraps the raw server origin with `/iam`.
- `IAMRusty/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for IAM producer events routed to Telegraph.
- `Telegraph/tests/common.rs` — wraps the raw server origin with `/telegraph`.
- `Hive/tests/common.rs` — wraps the raw server origin with `/hive`.
- `Hive/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for Hive producer events.
- `Manifesto/tests/common.rs` — wraps the raw server origin with `/manifesto` and returns the service test fixture, prefixed base URL, HTTP client, OpenFGA handle, and component-service mock.
- `Manifesto/tests/sqs_event_routing_tests.rs` — LocalStack SQS named-queue routing assertions for Manifesto producer events routed to SentinelSync.
- `Manifesto/tests/component_api_tests.rs` — see tests 4 / 5 / 6 for the deny / multi-tuple / phase-flip patterns.

## Key helpers

- `setup_test_server()` — reusable base URL + HTTP client for endpoint tests; should also surface the `OpenFgaMockService` handle when the service wires `rustycog-permission`.
- `MockServerFixture::reset()` — wipe every mounted stub for mid-test re-arrangement.
- `OpenFgaMockService::mock_check_allow` / `mock_check_deny` / `mock_check_any` / `reset()` / `client_config()` — per-tuple `Check` arrangement and an out-of-the-box `OpenFgaClientConfig` pointing at the fake.
- Kafka/SQS testcontainer helpers — opt-in real-transport coverage. For SQS routing, prefer `TestSqs::wait_for_messages_from_queue` and `TestSqs::get_all_messages_from_queue` over primary/default-queue helpers.
