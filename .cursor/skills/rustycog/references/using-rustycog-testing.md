# Using RustyCog Testing

Use this guide when setting up integration tests with `rustycog-testing`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests. For services that wire `rustycog-permission`, also return an `OpenFgaMockService` handle from the harness so individual tests can arrange `Check` decisions per tuple.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior.
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
- Leaving queue tests enabled by default when suites do not need transport behavior.
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
- `Manifesto/tests/common.rs` — canonical 4-tuple `setup_test_server` returning `(TestFixture, String, Client, OpenFgaMockService)`.
- `Manifesto/tests/component_api_tests.rs` — see tests 4 / 5 / 6 for the deny / multi-tuple / phase-flip patterns.

## Key helpers

- `setup_test_server()` — reusable base URL + HTTP client for endpoint tests; should also surface the `OpenFgaMockService` handle when the service wires `rustycog-permission`.
- `MockServerFixture::reset()` — wipe every mounted stub for mid-test re-arrangement.
- `OpenFgaMockService::mock_check_allow` / `mock_check_deny` / `mock_check_any` / `reset()` / `client_config()` — per-tuple `Check` arrangement and an out-of-the-box `OpenFgaClientConfig` pointing at the fake.
- Kafka/SQS testcontainer helpers — opt-in real-transport coverage.
