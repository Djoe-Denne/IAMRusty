---
title: Using RustyCog Server
category: skills
tags: [rustycog, server, skills, visibility/internal]
sources:
  - rustycog/rustycog-server/src/health.rs
summary: Minimal usage pattern for rustycog-server health primitives and custom HealthChecker implementations.
provenance:
  extracted: 0.91
  inferred: 0.04
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Using RustyCog Server

Use this guide when you only need health-probe contracts from `[[projects/rustycog/references/rustycog-server]]`.

## Workflow

- Implement `HealthChecker` for each component that needs explicit liveness/readiness checks.
- Return `HealthStatus::Unhealthy(message)` with actionable context for diagnostics.
- Aggregate checker results in your service health endpoint handler or startup checks.
- Keep this crate scoped to health concerns and use `rustycog-http` for route/server assembly.

## Common Pitfalls

- Assuming this crate provides full server bootstrap APIs.
- Returning generic unhealthy messages that are not actionable in operations.

## Sources

- [[projects/rustycog/references/rustycog-server]]
- [[entities/health-checker]]
- [[projects/rustycog/references/rustycog-http]]
