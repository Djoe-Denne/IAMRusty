---
title: RustyCog Events
category: references
tags: [reference, rustycog, events, visibility/internal]
sources:
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-events/src/event.rs
  - rustycog/rustycog-config/src/lib.rs
summary: rustycog-events defines domain event contracts and transport factories for Kafka, SQS, and no-op publishers/consumers.
provenance:
  extracted: 0.87
  inferred: 0.06
  ambiguous: 0.07
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Events

`rustycog-events` provides the event envelope and queue transport adapters used by `[[projects/rustycog/rustycog]]` services.

## Key Ideas

- `DomainEvent` standardizes event identity, aggregate linkage, timestamp, version, JSON serialization, and metadata.
- `EventPublisher<TError>` and `EventConsumer` define async contracts independent from transport choice.
- Factory helpers create Kafka, SQS, or no-op publishers/consumers from `QueueConfig`.
- In both test and production modes, failed transport setup can fall back to no-op behavior instead of hard failing startup.
- `create_multi_queue_event_publisher()` accepts queue-name sets and queue config, then builds a publisher adapter used by services that target multiple queues.

## Linked Entities

- [[entities/domain-event]]
- [[entities/event-publisher]]
- [[entities/queue-config]]

## Open Questions

- The current multi-queue helper tracks multiple queue names, but still builds one publisher instance underneath. Conflict to resolve. ^[ambiguous]
- Startup fallback to no-op publishers/consumers improves resilience but can hide broken messaging configuration if not observed closely. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[concepts/event-driven-microservice-platform]]
- [[projects/rustycog/rustycog]]
