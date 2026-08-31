---
title: >-
  Queue readiness signaling
category: concepts
tags: [events, readiness, rustycog, visibility/internal]
sources:
  - readiness/src/factory.rs
  - docs/reviews/iam-architecture-comparison.md
  - cursor-conversation/ready-queue-factories-2026-08-29
summary: >-
  Shared readiness factories classify rustycog queue no-ops as disabled, live, or degraded and log that status for /ready.
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Queue readiness signaling

RustyCog queue factories can silently fall back to a no-op publisher/consumer when config is disabled or the broker is missing. That hid broken wiring behind a green process. The `readiness` crate wraps those factories and **signals** the outcome.

## Contract

- `create_signaled_multi_queue_event_publisher` and `create_signaled_event_consumer` return the rustycog transport plus a `ComponentStatus` (`disabled` / `live` / `degraded`).
- `signal_queue_status` logs the service name, role (publisher vs consumer), and status so `/ready` and operators see the same classification.
- Manifesto, Telegraph, Hive, and IAMRusty should share this path so a no-op is an explicit signal, not an accident. ^[inferred]

## Related

- [[entities/queue-config]]
- [[entities/health-checker]]
- [[concepts/event-driven-microservice-platform]]
- [[skills/using-rustycog-events]]
- [[projects/aiforall/skills/running-aiforall-runtime-modes]]
