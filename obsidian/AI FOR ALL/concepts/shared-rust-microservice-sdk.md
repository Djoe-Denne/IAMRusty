---
title: >-
  Shared Rust Microservice SDK
category: concepts
tags: [sdk, rust, platform, visibility/internal]
sources:
  - rustycog/README.md
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-core/src/error.rs
  - rustycog/rustycog-command/src/lib.rs
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-db/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-logger/src/lib.rs
  - rustycog/rustycog-testing/src/lib.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  RustyCog is the shared SDK stack for platform services, but its umbrella package, workspace membership, and builder-level ergonomics do not all line up perfectly.
provenance:
  extracted: 0.78
  inferred: 0.08
  ambiguous: 0.14
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Shared Rust Microservice SDK

`[[projects/rustycog/rustycog]]` packages the recurring mechanics of service development into reusable crates. That makes it the platform-level implementation of a shared internal SDK for AIForAll services such as `[[projects/iamrusty/iamrusty]]` and `[[projects/manifesto/manifesto]]`.

## Key Ideas

- The root workspace treats most `rustycog-*` crates as regular members, and `rustycog/Cargo.toml` also exposes a `rustycog-meta` package that groups the stack behind one umbrella dependency.
- The SDK is not one monolith. It is split across shared errors, command execution, typed config loading, DB pooling, HTTP wiring, permissions, events, logging, and testing so services can compose the pieces they need; each crate now has its own page under `[[projects/rustycog/references/index]]`.
- `GenericCommandService`, `RouteBuilder`, `DbConnectionPool`, `QueueConfig`, and the reusable test harness show that the crates are meant to be consumed as one coordinated service shell rather than as unrelated helpers.
- The crate-level entity vocabulary is now promoted to global pages (`[[entities/command-registry]]`, `[[entities/queue-config]]`, `[[entities/route-builder]]`, and peers) so service docs can reference shared SDK nouns instead of redefining them.
- The generic extension points stay narrow: services supply their own concrete config types and permission fetchers through traits such as `HasLoggingConfig`, `HasServerConfig`, and `PermissionsFetcher`.
- The package set is clearly trying to standardize service scaffolding and reduce repeated infrastructure code across the platform. ^[inferred]
- The workspace and the umbrella package are close but not identical: `rustycog-meta` includes `rustycog-logger`, while the root workspace members list does not expose every RustyCog crate symmetrically. Conflict to resolve. ^[ambiguous]
- `rustycog-server` currently exports health primitives only, which is narrower than its name suggests. Conflict to resolve. ^[ambiguous]
- `rustycog-events` advertises multi-queue routing capability, but the current helper builds one underlying publisher instance. Conflict to resolve. ^[ambiguous]
- Manifesto-authored build guides still treat the SDK as the canonical stack, but they also show that some higher-level ergonomics and wiring stories vary by service rather than being perfectly standardized. ^[ambiguous]

## Open Questions

- The wiki still does not catalog which RustyCog crates are used by each service in production.
- There is not yet a single compatibility matrix showing stable versus evolving SDK surfaces. ^[ambiguous]
- The current code makes the crate boundaries clear, but it does not define a formal public API policy for consumers outside this workspace. ^[ambiguous]
- Should the platform prefer `rustycog-meta` or explicit per-crate dependencies as the default guidance for new services? Conflict to resolve. ^[ambiguous]
- SQS/AWS/Scaleway endpoint semantics in config remain mixed and are still not clarified as one canonical operator story. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]] — Code-backed inventory of the current crate surfaces
- [[projects/rustycog/references/index]] — Per-crate reference pages
- [[references/platform-building-blocks]] — RustyCog capabilities and shared event utilities
- [[references/rustycog-service-construction]] — Service-build guidance around the stack
- [[projects/rustycog/rustycog]] — Concrete SDK project page
- [[skills/building-rustycog-services]] — Practical workflow using this stack