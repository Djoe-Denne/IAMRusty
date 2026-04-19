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
updated: 2026-04-15T22:10:00Z
---

# QueueConfig

`QueueConfig` is the queue transport pivot from `[[projects/rustycog/references/rustycog-config]]`.

## Key Ideas

- `QueueConfig` is the runtime transport selector for event infrastructure (`Kafka`, `Sqs`, or `Disabled`).
- It connects typed configuration loading to event adapter wiring, so services can swap transport mode without changing event-calling code.
- `KafkaConfig`/`SqsConfig` hold transport-specific endpoint and credential details behind one enum boundary.
- Publisher/consumer factories in `[[projects/rustycog/references/rustycog-events]]` consume this type directly.

## Sources

- [[projects/rustycog/references/rustycog-config]]
- [[projects/rustycog/references/rustycog-events]]
- [[concepts/structured-service-configuration]]
