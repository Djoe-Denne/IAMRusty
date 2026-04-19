---
title: RustyCog Testing
category: references
tags: [reference, rustycog, testing, visibility/internal]
sources:
  - rustycog/rustycog-testing/src/lib.rs
  - rustycog/rustycog-testing/src/common/service_test_descriptor.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
  - rustycog/rustycog-testing/src/wiremock/mod.rs
summary: rustycog-testing bundles reusable service test descriptors, app/bootstrap helpers, shared wiremock fixtures, and real infrastructure harnesses for Kafka and SQS.
provenance:
  extracted: 0.9
  inferred: 0.06
  ambiguous: 0.04
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-19T10:59:36Z
---

# RustyCog Testing

`rustycog-testing` is the shared integration-test toolbox for services built on `[[projects/rustycog/rustycog]]`.

## Key Ideas

- The crate re-exports common test modules (DB, events, HTTP, wiremock) through one dependency.
- `ServiceTestDescriptor<T>` is the central service contract: it defines app build/run hooks, migration hooks, and capability flags (`has_db()`, `has_sqs()`).
- Fixture builders branch off descriptor flags to provision only the infrastructure a service needs, keeping shared helpers portable across services.
- `get_test_server()` and `setup_test_server()` manage a reusable global test server lifecycle using `OnceLock` and async mutex guards.
- The `wiremock` module provides a shared mock server fixture with explicit reset behavior for test isolation.
- Kafka and SQS testcontainer modules provide real transport fixtures for event-path integration tests.
- The package keeps service tests close to production wiring while still minimizing repeated bootstrapping code.

## Linked Entities

- [[entities/event-publisher]]
- [[entities/queue-config]]
- [[entities/route-builder]]

## Open Questions

- The balance between one global reusable server and strict test isolation is still service-dependent and can affect flaky-test posture. ^[inferred]

## Sources

- [[projects/rustycog/references/index]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/rustycog/rustycog]]
