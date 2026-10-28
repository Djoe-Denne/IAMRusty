---
title: Telegraph Testing and SMTP Fixtures
category: references
tags: [reference, testing, fixtures, visibility/internal]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - services/Telegraph/config/test.toml
  - services/Telegraph/tests/common.rs
  - services/Telegraph/tests/notification_http_endpoints_test.rs
  - services/Telegraph/tests/user_signup_event_test.rs
  - services/Telegraph/tests/user_email_verified_event_test.rs
  - services/Telegraph/tests/fixtures/smtp/mod.rs
  - services/Telegraph/tests/fixtures/smtp/service.rs
  - services/Telegraph/tests/fixtures/smtp/resources.rs
  - services/Telegraph/tests/fixtures/smtp/testcontainer.rs
summary: Telegraph real-infrastructure tests, SMTP fixture distinctions, durable recovery timing, SQS/OpenFGA/HTTP oracles and cleanup by owned IDs.
provenance:
  extracted: 0.86
  inferred: 0.08
  ambiguous: 0.06
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-10-27T19:27:11Z
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

## SMTP Fixture Stack

Telegraph keeps **two parallel SMTP fixtures** and chooses between them per test:

- **`SmtpService`** (`services/Telegraph/tests/fixtures/smtp/service.rs`) wraps the shared [[projects/rustycog/references/wiremock-mock-server-fixture]] to fake SMTP as HTTP. Each SMTP verb is modeled as `POST /smtp/<verb>` against the wiremock server: `mock_greeting`, `mock_ehlo`, `mock_auth`, `mock_mail_from`, `mock_rcpt_to`, `mock_data`, `mock_quit`. ^[inferred]
- High-level scenario helpers compose those primitives: `mock_successful_email_send(expected_email)`, `mock_authenticated_email_send(auth, email)`, `mock_auth_failure(auth)`, `mock_recipient_rejection(to_email)`. A `SmtpScenarioBuilder` (returned from `mock_custom_scenario()`) lets tests assemble ad-hoc multi-step flows fluently.
- `mock_data` matches body content adaptively: it always asserts the `subject` and every recipient address are present in the request body, then adds one `body_string_contains` matcher per word from the first three significant words (>2 chars) of the expected text body so template variation does not break the stub.
- Inspection helpers `received_requests`, `verify_email_sent(subject, recipient)`, and `email_count` read from `MockServer::received_requests` so tests can assert on what the service actually sent.
- Typed `SmtpResponse`/`SmtpAuthRequest`/`SmtpEmail`/`SmtpCapabilities` DTOs in `resources.rs` provide ready-made SMTP response codes (`service_ready`, `ok`, `auth_success`, `auth_failed`, `mailbox_unavailable`, `closing`) and pre-built emails (`user_signup_welcome`, `password_reset_request`, `email_verification`).
- **`TestSmtp`** (`services/Telegraph/tests/fixtures/smtp/testcontainer.rs`) is the heavier path: a real MailHog container started via `testcontainers` with the test-config SMTP port mapped through to MailHog's 1025, plus the API port pinned at 8025. It exposes `get_emails`, `email_count`, `has_email(subject, recipient)`, and `clear_emails` by calling MailHog's HTTP API at `/api/v1/messages`, parsing the MIME structure into `TestEmail` values.
- The April snapshot described a named-container cleanup fallback. It is not an operator authorization or the current cleanup contract: the October mission tracks exact IDs from fresh fixture ledgers, checks leases and preserves preexisting identities. Do not infer ownership from a name or image, or generalize cleanup to unowned containers.
- The HTTP-modeled `SmtpService` does not prove real SMTP framing. Use the real MailHog/testcontainers path for protocol-level evidence; [[skills/stubbing-http-with-wiremock]] concerns outbound HTTP collaborators, not replacement of SQL/SQS/SMTP protocol fidelity.

## Durable notification proof — October 2026

The component and full vertical use real PostgreSQL, SQS and OpenFGA fixtures with explicitly enabled transport. They are native Windows tests, one Cargo at a time with **24 jobs** configured and target E:, not application builds/tests in Docker or Kind.

[[projects/aiforall/skills/proving-leased-outbox-recovery]] separates SQL lease30s, SQS inbound visibility5s and local stop6s. Observe a future retry date at first FAILED, not after the independent12s sampler; recovery uses one persisted-eligibility deadline with a strict check after its final await. The reconstructed owner must retain join handles on timeout, not only request abort.

The final vertical passes nominal and outage/rebuild with real bytes, public Sentinel handler/OpenFGA, fresh-ledger replay, unique tuple,403 without mutation and200 recipient. The component's1/0/1 row counts are not the vertical's1/1/1. Static review and `--no-run` remain distinct from execution. Results and hashes: [[projects/aiforall/references/mission-telegraph-outbox-20261027]].

Each campaign ends with zero owned containers running. The final run preserved a SMTP Exited(2) fixture and two opaque volumes as ASK; this is not a clean-exit SMTP claim or permission to delete them. The15 preexisting container IDs/states were unchanged. No global Desktop/WSL shutdown was performed because a foreign workload remained active; RAM was not measured.

## Open Questions

- The event tests rely on polling loops and second-long sleeps to wait for delivery, which is practical but slower and less explicit than an acknowledgment-oriented harness. ^[inferred]
- SMTP and SQS fixtures cover the currently wired channels, but the test suite does not yet show how future direct-send or SMS-style paths would be validated. ^[ambiguous]

## Sources

- [[projects/telegraph/telegraph]] - Service whose HTTP and event flows are under test.
- [[concepts/integration-testing-with-real-infrastructure]] - Cross-service concept view of these patterns.
- [[projects/rustycog/references/rustycog-testing]] - Shared test harness Telegraph extends with SQS and SMTP fixtures.
- [[projects/rustycog/references/wiremock-mock-server-fixture]] - Underlying mock server `SmtpService` wraps.
- [[skills/stubbing-http-with-wiremock]] - Recipe behind the `mock_*` helper convention used here.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] - HTTP behaviors covered by the API tests.
- [[projects/telegraph/references/telegraph-event-processing]] - Queue behaviors covered by the event tests.
