---
title: >-
  Integration Testing with Real Infrastructure
category: concepts
tags: [testing, integration, fixtures, visibility/internal]
sources:
  - IAMRusty/docs/TESTING_GUIDE.md
  - IAMRusty/docs/FIXTURES_GUIDE.md
  - Manifesto/docs/rustycog-service-build-guide.md
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-testing/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
summary: >-
  Integration tests reuse shared servers, real containers, typed fixtures, and optional Kafka/LocalStack checks instead of mocking every dependency by default.
provenance:
  extracted: 0.82
  inferred: 0.10
  ambiguous: 0.08
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Integration Testing with Real Infrastructure

A recurring repo pattern is to test behavior with real infrastructure components wherever practical. `[[projects/iamrusty/iamrusty]]` documents this most fully, while `[[skills/building-rustycog-services]]` generalizes it for new services.

## Key Ideas

- Tests reuse a global server and shared containers for performance, then regain isolation through table truncation and fixture cleanup.
- Serial execution is treated as the default for stateful integration tests.
- Database state is created with typed fixture builders, while external OAuth services are mocked with focused service fixtures.
- `setup_test_server()` keeps one OnceLock-backed async server alive per process and returns a shared HTTP client setup so tests do not repeatedly rebuild the app shell.
- Kafka-backed tests start a real KRaft container, inject `RUSTYCOG_KAFKA__*` env vars, wait for broker readiness, and can read messages back with a dedicated test consumer.
- SQS-backed tests start LocalStack, inject queue env vars, create the configured queue, and support send/receive/delete/purge flows against real queue URLs.
- Port `0` handling plus explicit cache clearing in the config layer makes ephemeral container ports predictable within a test run.
- Queue-backed tests exist for Kafka and SQS, but the docs and code still recommend starting with mock/no-op publishers and escalating only for critical end-to-end paths.
- JWT helpers, HTTP client conventions, and test descriptors turn test wiring into reusable platform practice rather than ad hoc boilerplate.
- Kafka test config is RustyCog-prefixed while the SQS fixture currently injects service-specific `IAM_QUEUE__...` variables, so the cross-service test-env convention is not fully uniform. ^[ambiguous]

## Open Questions

- The docs describe both mock-based and queue-backed strategies, but they do not define one universal threshold for when to switch from one to the other.
- Some queue-backed paths are Docker-dependent or ignored, so full-suite expectations can vary by environment. ^[ambiguous]
- The wiki still does not show which services run Kafka/SQS fixtures in their normal CI path versus only locally. ^[ambiguous]

## Sources

- [[references/iamrusty-testing-and-fixtures]] — Main source summary for these practices
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the test harness and queue fixtures
- [[concepts/shared-rust-microservice-sdk]] — Platform context that makes these patterns reusable
- [[skills/testing-rust-services-with-fixtures]] — Concrete workflow distilled from the guides
