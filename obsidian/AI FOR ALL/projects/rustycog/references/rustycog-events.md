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
  extracted: 0.9
  inferred: 0.06
  ambiguous: 0.04
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-19T11:38:52.5746779Z
---

# RustyCog Events

`rustycog-events` provides the event envelope and queue transport adapters used by `[[projects/rustycog/rustycog]]` services.

## Key Ideas

- `DomainEvent` standardizes event identity, aggregate linkage, timestamp, version, JSON serialization, and metadata.
- `EventPublisher<TError>` and `EventConsumer` define async contracts independent from transport choice.
- Factory helpers create Kafka, SQS, or no-op publishers/consumers from `QueueConfig`.
- SQS factory paths initialize rustls' AWS-LC crypto provider once before AWS SDK usage.
- In test mode (`cfg(test)` or `test-utils`), Kafka usage depends on both config enablement and test-container env vars (`RUSTYCOG_KAFKA__HOST`, `PORT`, `ENABLED`), while SQS uses enablement checks.
- In both test and production modes, failed transport setup can fall back to no-op behavior instead of hard failing startup.
- `create_multi_queue_event_publisher()` accepts queue-name sets and queue config, then builds a publisher adapter used by services that target multiple queues.
- The current multi-queue helper still wraps one underlying publisher instance, with explicit code comments that broader multi-publisher fanout is a future extension.

## Linked Entities

- [[entities/domain-event]]
- [[entities/event-publisher]]
- [[entities/queue-config]]

## Open Questions

- The current multi-queue helper tracks multiple queue names, but still builds one publisher instance underneath. Conflict to resolve. ^[ambiguous]
- Startup fallback to no-op publishers/consumers improves resilience but can hide broken messaging configuration if not observed closely. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[concepts/event-driven-microservice-platform]]
- [[projects/telegraph/references/telegraph-event-processing]] - Concrete SQS-backed consumer and descriptor-driven delivery flow built on these abstractions.
- [[projects/rustycog/rustycog]]
