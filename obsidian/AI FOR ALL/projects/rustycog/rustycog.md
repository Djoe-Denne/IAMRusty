---
title: >-
  RustyCog
category: project
tags: [sdk, rust, platform, visibility/internal]
sources:
  - rustycog/README.md
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-testing/src/common/test_server.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  RustyCog is the shared Rust SDK/workspace that standardizes commands, config, HTTP, permissions, events, logging, DB access, and test infrastructure across services.
provenance:
  extracted: 0.77
  inferred: 0.09
  ambiguous: 0.14
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog

## Indexes

- [[projects/rustycog/references/index]] — references

RustyCog is the shared platform SDK for AIForAll. It lives as a set of `rustycog-*` crates inside the main workspace, and `rustycog/Cargo.toml` also exposes a `rustycog-meta` package that bundles the stack for consumers that want one umbrella dependency. It is the concrete implementation of `[[concepts/shared-rust-microservice-sdk]]` for services such as `[[projects/iamrusty/iamrusty]]` and `[[projects/manifesto/manifesto]]`.

## Crate References

- [[projects/rustycog/references/rustycog-core]]
- [[projects/rustycog/references/rustycog-config]]
- [[projects/rustycog/references/rustycog-db]]
- [[projects/rustycog/references/rustycog-command]]
- [[projects/rustycog/references/rustycog-events]]
- [[projects/rustycog/references/rustycog-http]]
- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-testing]]
- [[projects/rustycog/references/rustycog-server]]
- [[projects/rustycog/references/rustycog-logger]]
- [[projects/rustycog/references/rustycog-meta]]

## Key Ideas

- RustyCog is organized as focused crates so services can compose only what they need while keeping one shared mental model for errors, commands, config, HTTP, permissions, events, logging, DB, and tests.
- The per-crate pages in `[[projects/rustycog/references/index]]` now document each crate separately, rather than keeping the details only in one catalog page.
- Shared technical vocabulary from these crates is promoted into global entity pages under `[[entities/index]]` to reduce duplicate explanations across service docs.
- `rustycog-meta` provides umbrella packaging while direct crate dependencies remain a viable path for explicit dependency control.

## Open Questions

- The wiki still does not catalog a service-by-service crate-adoption matrix for production deployments.
- `rustycog-logger` is included in `rustycog-meta`, but it is not listed in root workspace members. Conflict to resolve. ^[ambiguous]
- `rustycog-server` currently exposes health primitives only, despite a broader crate name. Conflict to resolve. ^[ambiguous]
- `create_multi_queue_event_publisher()` currently tracks multiple queue names but builds one publisher instance. Conflict to resolve. ^[ambiguous]
- The README still advertises macros/examples not present in this tree. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]] — Code-backed map of the crate surfaces
- [[references/platform-building-blocks]] — Shared SDK and event-contract foundation
- [[concepts/shared-rust-microservice-sdk]] — Cross-project abstraction implemented here
- [[skills/building-rustycog-services]] — Practical workflow derived from that guidance