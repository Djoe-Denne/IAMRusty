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
summary: >-
  RustyCog is a coordinated internal SDK/workspace whose crates provide errors, config, commands, HTTP, permissions, events, logging, DB access, and testing for platform services.
provenance:
  extracted: 0.84
  inferred: 0.10
  ambiguous: 0.06
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Shared Rust Microservice SDK

`[[projects/rustycog/rustycog]]` packages the recurring mechanics of service development into reusable crates. That makes it the platform-level implementation of a shared internal SDK for AIForAll services such as `[[projects/iamrusty/iamrusty]]` and `[[projects/manifesto/manifesto]]`.

## Key Ideas

- The root workspace treats most `rustycog-*` crates as regular members, and `rustycog/Cargo.toml` also exposes a `rustycog-meta` package that groups the stack behind one umbrella dependency.
- The SDK is not one monolith. It is split across shared errors, command execution, typed config loading, DB pooling, HTTP wiring, permissions, events, logging, and testing so services can compose the pieces they need.
- `GenericCommandService`, `RouteBuilder`, `DbConnectionPool`, `QueueConfig`, and the reusable test harness show that the crates are meant to be consumed as one coordinated service shell rather than as unrelated helpers.
- The generic extension points stay narrow: services supply their own concrete config types and permission fetchers through traits such as `HasLoggingConfig`, `HasServerConfig`, and `PermissionsFetcher`.
- The package set is clearly trying to standardize service scaffolding and reduce repeated infrastructure code across the platform. ^[inferred]
- `rustycog-logger` is included in the meta package but not in the root workspace member list, and the README still mentions macros/examples that are not present in this tree. ^[ambiguous]

## Open Questions

- The wiki still does not catalog which RustyCog crates are used by each service in production.
- There is not yet a single compatibility matrix showing stable versus evolving SDK surfaces. ^[ambiguous]
- The current code makes the crate boundaries clear, but it does not define a formal public API policy for consumers outside this workspace. ^[ambiguous]

## Sources

- [[references/rustycog-crate-catalog]] — Code-backed inventory of the current crate surfaces
- [[references/platform-building-blocks]] — RustyCog capabilities and shared event utilities
- [[references/rustycog-service-construction]] — Service-build guidance around the stack
- [[projects/rustycog/rustycog]] — Concrete SDK project page
- [[skills/building-rustycog-services]] — Practical workflow using this stack