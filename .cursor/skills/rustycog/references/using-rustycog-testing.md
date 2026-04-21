# Using RustyCog Testing

Use this guide when setting up integration tests with `rustycog-testing`.

## Workflow

- Create one service test descriptor that builds app fixtures, test DB setup, and HTTP app wiring.
- Use `setup_test_server()` to obtain reusable base URL and HTTP client for endpoint tests.
- Add DB fixtures and migration setup in shared test initialization so each test starts from explicit state.
- Enable Kafka/SQS testcontainer helpers only for tests that need real queue behavior.
- Keep transport-heavy tests separate from fast unit tests to preserve local iteration speed.

## Common Pitfalls

- Recreating server/process setup in each test instead of reusing descriptor-based helpers.
- Leaving queue tests enabled by default when suites do not need transport behavior.
- Forgetting to reset state between tests when reusing shared server instances.

## Source files

- `rustycog/rustycog-testing/src/lib.rs`
- `rustycog/rustycog-testing/src/common/test_server.rs`
- `rustycog/rustycog-testing/src/common/kafka_testcontainer.rs`
- `rustycog/rustycog-testing/src/common/sqs_testcontainer.rs`

## Key helpers

- `setup_test_server()` — reusable base URL + HTTP client for endpoint tests
- Kafka/SQS testcontainer helpers — opt-in real-transport coverage
