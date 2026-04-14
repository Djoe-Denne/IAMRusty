---
title: >-
  Event-Driven Microservice Platform
category: concepts
tags: [architecture, microservices, events, visibility/internal]
sources:
  - README.md
  - docs/project/Archi.md
  - IAMRusty/README.md
  - rustycog/README.md
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-events/src/event.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
  - hive-events/README.md
summary: >-
  The platform uses decoupled services plus transport-neutral domain events and queue-backed coordination for non-blocking workflows and integrations.
provenance:
  extracted: 0.80
  inferred: 0.10
  ambiguous: 0.10
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Event-Driven Microservice Platform

AIForAll favors asynchronous coordination between bounded services instead of pushing all workflows through synchronous request chains. The clearest examples connect `[[projects/iamrusty/iamrusty]]`, `[[entities/telegraph]]`, `[[projects/hive-events/hive-events]]`, and the project-service work captured in `[[projects/manifesto/manifesto]]`.

## Key Ideas

- IAMRusty publishes user-signup style events and Telegraph consumes them to send notifications.
- Hive Events routes messages into purpose-specific queues such as `notification-events` and `sync-events`.
- RustyCog formalizes the event envelope through `DomainEvent`, which requires an event type, IDs, timestamp, version, JSON payload, and metadata independent of the transport.
- `QueueConfig` and the concrete publisher/consumer factories let services switch between Kafka, SQS, or disabled/no-op mode without rewriting the higher-level event API.
- The test harness shows both transports are active parts of the codebase: Kafka tests provision a KRaft container and consume messages back from the topic, while SQS tests provision LocalStack and exercise real queue URLs and message bodies.
- Asynchronous messaging lets services keep ownership over their own data and still participate in longer workflows. ^[inferred]
- The SDK now makes Kafka and SQS both first-class options in code, but the wiki still does not show which services standardize on which transport in production. ^[ambiguous]

## Open Questions

- The boundary between queue-backed domain events and any Kafka-based internal event tooling is not documented end to end.
- The event factories fall back to no-op publishers/consumers when transports are disabled or fail to initialize, so the desired production stance toward that fallback is not yet documented. ^[ambiguous]
- Retry, dead-letter, and observability strategies are only partially described in this ingest pass.

## Sources

- [[references/aiforall-platform]] — Repo-level workflow and service communication
- [[references/manifesto-service]] — Project-service orchestration and cascading ADR
- [[references/rustycog-crate-catalog]] — Code-backed inventory of the event crates
- [[references/platform-building-blocks]] — Event contracts and shared infrastructure primitives
