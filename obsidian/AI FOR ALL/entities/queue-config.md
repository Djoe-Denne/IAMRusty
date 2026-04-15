---
title: QueueConfig
category: entities
tags: [rustycog, configuration, events, visibility/internal]
sources:
  - rustycog/rustycog-config/src/lib.rs
summary: QueueConfig is the transport selector and config model that drives Kafka, SQS, or disabled event behavior in RustyCog.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# QueueConfig

`QueueConfig` is the queue transport pivot from `[[projects/rustycog/references/rustycog-config]]`.

## Key Ideas

- It is an enum with `Kafka`, `Sqs`, and `Disabled` variants that provides one runtime branch point for event infrastructure.
- `KafkaConfig` and `SqsConfig` include broker/queue details, credentials, enable flags, timeout/retry controls, and helpers for endpoint/broker construction.
- Queue config objects include random-port resolution/cache helpers used heavily in test scenarios.
- Event publisher and consumer factories in `[[projects/rustycog/references/rustycog-events]]` consume this type directly.

## Open Questions

- Naming and endpoint semantics currently mix AWS and Scaleway conventions in SQS-oriented code paths. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-config]]
- [[projects/rustycog/references/rustycog-events]]
- [[concepts/structured-service-configuration]]
