---
title: EventPublisher
category: entities
tags: [rustycog, events, messaging, visibility/internal]
sources:
  - rustycog/rustycog-events/src/event.rs
  - rustycog/rustycog-events/src/lib.rs
summary: EventPublisher is the async RustyCog interface for single/batch event publication and health checks across Kafka, SQS, or no-op backends.
provenance:
  extracted: 0.89
  inferred: 0.05
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T22:10:00Z
---

# EventPublisher

`EventPublisher<TError>` is the publishing abstraction behind RustyCog event transport factories.

## Key Ideas

- `EventPublisher` is the async publication interface (`publish`, `publish_batch`, `health_check`) used by services and adapters.
- Factory wiring selects Kafka, SQS, or no-op implementations from `QueueConfig`.
- The abstraction keeps call sites transport-agnostic while leaving transport-specific setup in one place.
- Queue-targeted variants build on top of this base publisher contract.

## Sources

- [[projects/rustycog/references/rustycog-events]]
- [[entities/domain-event]]
- [[entities/queue-config]]
