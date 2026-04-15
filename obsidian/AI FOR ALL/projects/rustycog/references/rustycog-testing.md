---
title: RustyCog Testing
category: references
tags: [reference, rustycog, testing, visibility/internal]
sources:
  - rustycog/rustycog-testing/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
summary: rustycog-testing bundles reusable test utilities for app bootstrapping, HTTP clients, and real infrastructure fixtures (Kafka and SQS).
provenance:
  extracted: 0.88
  inferred: 0.06
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Testing

`rustycog-testing` is the shared integration-test toolbox for services built on `[[projects/rustycog/rustycog]]`.

## Key Ideas

- The crate re-exports common test modules (DB, events, HTTP, wiremock) through one dependency.
- `get_test_server()` and `setup_test_server()` manage a reusable global test server lifecycle using `OnceLock` and async mutex guards.
- Test server setup relies on shared descriptors and fixtures so each service can wire migrations, app state, and infra consistently.
- Kafka and SQS testcontainer modules provide real transport fixtures for event-path integration tests.
- The package keeps service tests close to production wiring while still minimizing repeated bootstrapping code.

## Linked Entities

- [[entities/event-publisher]]
- [[entities/queue-config]]
- [[entities/route-builder]]

## Open Questions

- The balance between one global reusable server and strict test isolation is still service-dependent and can affect flaky-test posture. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/rustycog/rustycog]]
