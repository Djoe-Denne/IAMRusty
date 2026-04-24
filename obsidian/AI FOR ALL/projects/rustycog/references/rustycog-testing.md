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
  - rustycog/rustycog-testing/src/common/openfga_testcontainer.rs
  - rustycog/rustycog-testing/src/wiremock/mod.rs
  - rustycog/rustycog-testing/src/permission/mod.rs
summary: >-
  rustycog-testing bundles service test descriptors, app/bootstrap helpers, shared wiremock, and real infrastructure harnesses for DB, Kafka, SQS, and OpenFGA.
provenance:
  extracted: 0.88
  inferred: 0.09
  ambiguous: 0.03
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-24T19:05:00Z
---

# RustyCog Testing

`rustycog-testing` is the shared integration-test toolbox for services built on `[[projects/rustycog/rustycog]]`.

## Key Ideas

- The crate re-exports common test modules (DB, events, HTTP, wiremock) through one dependency.
- `ServiceTestDescriptor<T>` is the central service contract: it defines app build/run hooks, migration hooks, and capability flags (`has_db()`, `has_sqs()`, `has_openfga()`).
- Fixture builders branch off descriptor flags to provision only the infrastructure a service needs, keeping shared helpers portable across services.
- `get_test_server()` and `setup_test_server()` manage a reusable global test server lifecycle using `OnceLock` and async mutex guards.
- The `wiremock` module provides a shared mock server fixture with explicit reset behavior for test isolation; see [[projects/rustycog/references/wiremock-mock-server-fixture]] for the full API surface, port-3000 singleton model, and stub patterns.
- `MockServerFixture::new()` eagerly resets all previously mounted mocks, `MockServerFixture::reset()` is exposed for mid-test re-arrangement, and its `Drop` impl schedules another reset on the current Tokio runtime, so tests stay isolated even when sharing a single `wiremock::MockServer` across the whole process.
- The `permission` module is now a compatibility re-export for `TestOpenFga`; the old wiremock-backed `OpenFgaMockService` files were removed.
- Kafka, SQS, and OpenFGA testcontainer modules provide real transport/protocol fixtures for integration tests. They follow the singleton + defensive-Docker-cleanup recipe captured in [[skills/creating-testcontainer-fixtures]], which is the entry point for adding any new shared or per-service container fixture.
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]] runs `openfga/openfga`, creates a fresh store/model per test setup, and exposes `allow`/`deny` tuple helpers so permission-gated route tests exercise production OpenFGA semantics instead of stubbed HTTP responses.
- The package keeps service tests close to production wiring while still minimizing repeated bootstrapping code.

## Linked Entities

- [[entities/event-publisher]]
- [[entities/queue-config]]
- [[entities/route-builder]]

## Open Questions

- The balance between one global reusable server and strict test isolation is still service-dependent and can affect flaky-test posture. ^[inferred]

## Sources

- [[projects/rustycog/references/index]]
- [[projects/rustycog/references/wiremock-mock-server-fixture]]
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]]
- [[projects/rustycog/references/openfga-mock-service]]
- [[skills/creating-testcontainer-fixtures]]
- [[skills/stubbing-http-with-wiremock]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/rustycog/rustycog]]
