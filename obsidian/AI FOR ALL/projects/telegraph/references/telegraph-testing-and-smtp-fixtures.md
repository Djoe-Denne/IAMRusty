---
title: Telegraph Testing and SMTP Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - Telegraph/config/test.toml
  - Telegraph/tests/common.rs
  - Telegraph/tests/notification_http_endpoints_test.rs
  - Telegraph/tests/user_signup_event_test.rs
  - Telegraph/tests/user_email_verified_event_test.rs
summary: Telegraph-specific testing notes layered on top of RustyCog's shared harness, focusing on its SQS, SMTP, notification, and end-to-end delivery checks.
provenance:
  extracted: 0.8
  inferred: 0.12
  ambiguous: 0.08
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Telegraph Testing and SMTP Fixtures

This page narrows `[[projects/rustycog/references/rustycog-testing]]` to the way `[[projects/telegraph/telegraph]]` proves both its notification API and queue-driven delivery paths.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-testing]]` explains the shared test fixture model, migration hooks, JWT helpers, and boot path that Telegraph extends.
- `[[concepts/integration-testing-with-real-infrastructure]]` captures the broader real-infrastructure testing pattern that this service applies to queues and SMTP as well as HTTP.

## Service-Specific Differences

- `TelegraphTestDescriptor` extends the shared `rustycog_testing` model with real database and SQS support, while Telegraph-specific setup adds a dedicated SMTP container for email assertions.
- `setup_test_server()` creates a `TelegraphTestFixture`, clears prior SMTP state, then boots the app through the shared RustyCog test-server path so the service is exercised with real infrastructure rather than a mocked shell.
- HTTP integration tests use real JWTs, database fixture builders, and serialized execution to validate pagination, unread filtering, and ownership enforcement for notification endpoints.
- Queue-driven tests publish `iam_events` payloads through the SQS fixture and then poll either the SMTP container or the database until the expected email or notification record appears.
- When adding a new event type or delivery mode, the most reliable test shape is still end to end: publish the real queue payload, then assert the channel-specific side effect (SMTP state, persisted notification rows, or both) instead of unit-testing the processor in isolation.
- `config/test.toml` keeps the environment dynamic but realistic: DB and SQS use `port = 0`, SMTP runs locally on `1025`, and event routing stays enabled.
- Compared with the current IAMRusty pages, Telegraph's test suite leans more heavily on SQS plus SMTP verification than on provider-mock plus Kafka-style flows. ^[ambiguous]

## Open Questions

- The event tests rely on polling loops and second-long sleeps to wait for delivery, which is practical but slower and less explicit than an acknowledgment-oriented harness. ^[inferred]
- SMTP and SQS fixtures cover the currently wired channels, but the test suite does not yet show how future direct-send or SMS-style paths would be validated. ^[ambiguous]

## Sources

- [[projects/telegraph/telegraph]] - Service whose HTTP and event flows are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Cross-service concept view of these patterns.
- [[projects/rustycog/references/rustycog-testing]] - Shared test harness Telegraph extends with SQS and SMTP fixtures.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] - HTTP behaviors covered by the API tests.
- [[projects/telegraph/references/telegraph-event-processing]] - Queue behaviors covered by the event tests.
