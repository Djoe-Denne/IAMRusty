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
updated: 2026-04-15T17:15:56.0808743Z
---

# EventPublisher

`EventPublisher<TError>` is the publishing abstraction behind RustyCog event transport factories.

## Key Ideas

- It defines `publish`, `publish_batch`, and `health_check` so call sites can treat transports uniformly.
- `ConcreteEventPublisher` switches between Kafka, SQS, and no-op implementations based on `QueueConfig`.
- Publisher factory functions include defensive fallback behavior when queue setup fails.
- `create_multi_queue_event_publisher()` introduces queue-name-targeted publishing semantics on top of the base publisher abstraction.

## Open Questions

- Multi-queue publishing behavior currently reuses one underlying publisher instance, so queue-specific guarantees are still evolving. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-events]]
- [[entities/domain-event]]
- [[entities/queue-config]]
