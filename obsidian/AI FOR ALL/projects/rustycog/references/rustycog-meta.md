---
title: RustyCog Meta
category: references
tags: [reference, rustycog, packaging, visibility/internal]
sources:
  - rustycog/Cargo.toml
  - Cargo.toml
summary: rustycog-meta is the umbrella package that re-exports all RustyCog crates as one dependency surface for services.
provenance:
  extracted: 0.85
  inferred: 0.06
  ambiguous: 0.09
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Meta

`rustycog-meta` is the packaging umbrella for `[[projects/rustycog/rustycog]]`: one dependency that includes core, command, config, DB, events, HTTP, permission, logger, server, and testing crates.

## Key Ideas

- The package lives in `rustycog/Cargo.toml` and depends on all first-party `rustycog-*` crates via local paths.
- It is useful for fast bootstrapping when a service wants the full stack without listing each crate explicitly.
- The root workspace (`Cargo.toml`) still tracks crate membership independently from this umbrella package, so packaging and workspace membership are related but not identical.
- `rustycog-meta` keeps dependency selection simple for consumers, while direct per-crate dependencies provide tighter dependency control.

## Linked Entities

- [[entities/command-registry]]
- [[entities/route-builder]]
- [[entities/domain-event]]

## Open Questions

- The repo does not yet define one canonical recommendation for new services: meta-package convenience versus explicit crate-by-crate dependencies. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[projects/rustycog/rustycog]]
- [[concepts/shared-rust-microservice-sdk]]
