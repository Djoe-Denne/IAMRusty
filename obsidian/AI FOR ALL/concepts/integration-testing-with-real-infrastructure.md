---
title: Integration Testing with Real Infrastructure
category: concepts
tags: [testing, integration, fixtures, visibility/internal]
sources:
  - IAMRusty/docs/TESTING_GUIDE.md
  - IAMRusty/docs/FIXTURES_GUIDE.md
  - IAMRusty/docs/KAFKA_EVENT_TESTING_GUIDE.md
  - IAMRusty/tests/fixtures/db/mod.rs
  - IAMRusty/tests/signup_kafka.rs
  - Telegraph/config/test.toml
  - Telegraph/tests/common.rs
  - Telegraph/tests/notification_http_endpoints_test.rs
  - Telegraph/tests/user_signup_event_test.rs
  - Telegraph/tests/user_email_verified_event_test.rs
  - Hive/config/test.toml
  - Hive/tests/common.rs
  - Hive/tests/organization_api_tests.rs
  - Hive/tests/members_api_tests.rs
  - Hive/tests/external_link_api_tests.rs
  - Hive/tests/fixtures/external_provider/service.rs
summary: Repo services favor real DB, queue, and HTTP fixtures, with Hive adding DB-plus-mock-provider API tests alongside IAMRusty’s Kafka and Telegraph’s SQS/SMTP coverage.
provenance:
  extracted: 0.76
  inferred: 0.13
  ambiguous: 0.11
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Integration Testing with Real Infrastructure

`[[projects/iamrusty/iamrusty]]`, `[[projects/telegraph/telegraph]]`, and `[[projects/hive/hive]]` all lean on integration tests that exercise real transport, database, and application state instead of treating orchestration code as something to mock away. The concrete stacks differ, but the repo-wide testing instinct is the same.

## Key Ideas

- Tests are designed around a shared test-server bootstrap, real database state, and `#[serial]` execution so runtime setup and cleanup stay deterministic across services.
- IAMRusty's suite focuses on HTTP, DB fixtures, provider mocks, and optional Kafka-backed checks, while Telegraph extends the same general pattern with real SQS plus a dedicated SMTP container for delivery assertions.
- Hive follows the same test-server pattern but keeps queue publishing disabled in the default test runtime, emphasizing real DB state, JWT-backed API calls, and mock external-provider HTTP fixtures instead of real queue consumers.
- Telegraph's `TelegraphTestDescriptor` explicitly declares DB and SQS support, and `setup_test_server()` clears prior SMTP state before booting the service through shared `rustycog_testing` infrastructure.
- Hive's `HiveTestDescriptor` explicitly keeps `has_db()` true and `has_sqs()` false, then boots the service through the same shared `rustycog_testing` infrastructure used elsewhere.
- Telegraph's HTTP tests use real JWTs, DB fixtures, and the live route table to verify pagination, unread filtering, and ownership semantics for the notification read model.
- Hive's org/member/external-link tests use real JWTs, DB fixtures, and a Wiremock-backed external-provider service to verify authorization, persistence, and integration behavior through the live HTTP server.
- Telegraph's queue-driven tests publish `iam_events` payloads through the SQS fixture, then poll SMTP or the database until the expected email or notification record appears.
- Conflict to resolve: IAMRusty's current live wiki emphasizes optional Kafka-backed verification, Telegraph's live event coverage is SQS plus SMTP centric, and Hive's default tests are HTTP-plus-DB with a mocked external provider and queue disabled. All three real-infrastructure variants are first-class in this repo. ^[ambiguous]

## Open Questions

- The repo still does not present one unified rule for when services should prefer Kafka fixtures versus SQS and SMTP fixture stacks for event-heavy tests. ^[ambiguous]
- Hive's current test harness treats queue publishing as optional even though production config enables queue output, so event verification depth still varies by service. ^[ambiguous]
- Telegraph's polling loops and second-long sleeps are practical for async delivery verification, but the suite would be faster if the shared harness exposed stronger event-completion signals. ^[inferred]

## Sources

- [[projects/iamrusty/iamrusty]] - Service whose auth and queue flows exemplify the IAM side of the pattern.
- [[projects/telegraph/telegraph]] - Service adding SQS and SMTP-backed delivery verification.
- [[projects/hive/hive]] - Service adding DB-backed API tests and mocked external-provider integration.
- [[references/iamrusty-testing-and-fixtures]] - Concrete IAMRusty examples behind the original page.
- [[references/telegraph-testing-and-smtp-fixtures]] - Concrete Telegraph examples for HTTP, SQS, and SMTP.
- [[references/hive-testing-and-api-fixtures]] - Concrete Hive examples for HTTP, DB, and external-provider fixtures.
- [[concepts/structured-service-configuration]] - Random ports and typed config matter in both suites.