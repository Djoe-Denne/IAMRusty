---
title: Using RustyCog Events
category: skills
tags: [rustycog, events, skills, visibility/internal]
sources:
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-events/src/event.rs
  - rustycog/rustycog-config/src/lib.rs
summary: Operational workflow for defining domain events and wiring RustyCog event publishers/consumers with QueueConfig.
provenance:
  extracted: 0.88
  inferred: 0.05
  ambiguous: 0.07
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Using RustyCog Events

Use this guide when integrating `<!-- [[projects/rustycog/references/rustycog-events]] -->` in service setup.

## Workflow

- Define event payload types that satisfy `DomainEvent` (type, IDs, timestamp, version, payload JSON, metadata).
- Load `QueueConfig` from service config and build publisher/consumer via factory helpers.
- Use `publish` for single events and `publish_batch` for transactional/event-burst cases.
- For queue-targeted scenarios, use `create_multi_queue_event_publisher()` with explicit queue-name sets.
- Add transport health checks to startup diagnostics to detect silent no-op fallbacks.

## Common Pitfalls

- Assuming queue setup failure always stops startup; factories can degrade to no-op mode.
- Mixing transport-specific event naming conventions without a shared event-type contract.
- Treating multi-queue publisher helpers as fully isolated per-queue publishers. ^[ambiguous]

## Sources

- <!-- [[projects/rustycog/references/rustycog-events]] -->
- <!-- [[entities/domain-event]] -->
- <!-- [[concepts/event-driven-microservice-platform]] -->
