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
summary: Telegraph exercises both its read-model API and queue-driven delivery flows with rustycog-testing, real SQS, SMTP containers, and direct DB checks.
provenance:
  extracted: 0.8
  inferred: 0.12
  ambiguous: 0.08
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-04-14T18:18:24.0602572Z
---

# Telegraph Testing and SMTP Fixtures

These sources show how `[[projects/telegraph/telegraph]]` validates both halves of the service: the authenticated notification API and the queue-driven communication pipeline that sends emails or creates persisted notifications.

## Key Ideas

- `TelegraphTestDescriptor` extends the shared `rustycog_testing` model with real database and SQS support, while Telegraph-specific setup adds a dedicated SMTP container for email assertions.
- `setup_test_server()` creates a `TelegraphTestFixture`, clears prior SMTP state, then boots the app through the shared RustyCog test-server path so the service is exercised with real infrastructure rather than a mocked shell.
- HTTP integration tests use real JWTs, database fixture builders, and serialized execution to validate pagination, unread filtering, and ownership enforcement for notification endpoints.
- Queue-driven tests publish `iam_events` payloads through the SQS fixture and then poll either the SMTP container or the database until the expected email or notification record appears.
- `config/test.toml` keeps the environment dynamic but realistic: DB and SQS use `port = 0`, SMTP runs locally on `1025`, and event routing stays enabled.
- Compared with the current IAMRusty pages, Telegraph's test suite leans more heavily on SQS plus SMTP verification than on provider-mock plus Kafka-style flows. Conflict to resolve only if the repo wants one unified event-testing story. ^[ambiguous]

## Open Questions

- The event tests rely on polling loops and second-long sleeps to wait for delivery, which is practical but slower and less explicit than an acknowledgment-oriented harness. ^[inferred]
- SMTP and SQS fixtures cover the currently wired channels, but the test suite does not yet show how future direct-send or SMS-style paths would be validated. ^[ambiguous]

## Sources

- [[projects/telegraph/telegraph]] - Service whose HTTP and event flows are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Cross-service concept view of these patterns.
- [[references/telegraph-http-and-notification-api]] - HTTP behaviors covered by the API tests.
- [[references/telegraph-event-processing]] - Queue behaviors covered by the event tests.
