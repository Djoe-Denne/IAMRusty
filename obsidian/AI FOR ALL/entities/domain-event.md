---
title: DomainEvent
category: entities
tags: [rustycog, events, messaging, visibility/internal]
sources:
  - rustycog/rustycog-events/src/event.rs
summary: DomainEvent is RustyCog's transport-neutral event contract covering event identity, aggregate linkage, versioning, payload serialization, and metadata.
provenance:
  extracted: 0.91
  inferred: 0.04
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# DomainEvent

`DomainEvent` is the core event contract in `[[projects/rustycog/references/rustycog-events]]`.

## Key Ideas

- It requires event type, event ID, aggregate ID, occurrence timestamp, schema version, metadata, and JSON serialization capability.
- The trait is transport-neutral, which lets Kafka, SQS, and no-op implementations share one event envelope.
- `BaseEvent` provides a reusable struct for common event fields and metadata/version helpers.
- Services can define custom event payload types while preserving one shared event contract.

## Open Questions

- The wiki still needs a cross-service schema-evolution playbook for `version()` strategy over time. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-events]]
- [[entities/event-publisher]]
- [[concepts/event-driven-microservice-platform]]
