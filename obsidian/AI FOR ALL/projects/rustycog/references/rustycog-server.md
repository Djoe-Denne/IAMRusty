---
title: RustyCog Server
category: references
tags: [reference, rustycog, server, visibility/internal]
sources:
  - rustycog/rustycog-server/src/health.rs
summary: rustycog-server currently exposes lightweight health-check abstractions rather than a full application bootstrap layer.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T22:10:00Z
---

# RustyCog Server

`rustycog-server` is a minimal crate that currently focuses on health-check contracts.

## Key Ideas

- `HealthStatus` encodes `Healthy` or `Unhealthy(message)` responses.
- `HealthChecker` defines one async `check()` contract for pluggable health probes.
- `BasicHealthChecker` is the default implementation and always returns healthy.
- The crate is re-exported through `rustycog-meta`, so consumers can still import it from umbrella dependencies.

## Usage Guidance

- This crate is intentionally narrow; most server bootstrapping belongs to `[[projects/rustycog/references/rustycog-http]]`.
- In practice, teams use `HealthChecker` as a shared probe contract and keep route/server assembly in HTTP/setup crates.

## Linked Entities

- [[entities/health-checker]]

## Open Questions

- The crate name suggests broader server-setup ownership, but the current surface is health-only. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[projects/rustycog/rustycog]]
- [[concepts/shared-rust-microservice-sdk]]
