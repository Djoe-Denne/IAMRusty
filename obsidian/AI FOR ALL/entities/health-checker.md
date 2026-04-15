---
title: HealthChecker
category: entities
tags: [rustycog, server, health, visibility/internal]
sources:
  - rustycog/rustycog-server/src/health.rs
summary: HealthChecker is the RustyCog trait for asynchronous component health checks, with HealthStatus as the common result shape.
provenance:
  extracted: 0.91
  inferred: 0.04
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# HealthChecker

`HealthChecker` is the health-probe abstraction exported by `[[projects/rustycog/references/rustycog-server]]`.

## Key Ideas

- The trait defines one async `check()` method returning `HealthStatus`.
- `HealthStatus` provides `Healthy` or `Unhealthy(message)` as shared status outcomes.
- `BasicHealthChecker` is the minimal always-healthy implementation used as a default.
- The type gives service code one common health-probe contract even though crate scope is currently small.

## Open Questions

- The current crate scope is limited to health primitives, so broader server concerns remain in other crates. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-server]]
- [[entities/route-builder]]
- [[concepts/shared-rust-microservice-sdk]]
