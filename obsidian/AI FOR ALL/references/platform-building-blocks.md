---
title: >-
  Platform Building Blocks
category: references
tags: [reference, sdk, events, visibility/internal]
sources:
  - rustycog/README.md
  - Cargo.toml
  - rustycog/Cargo.toml
  - rustycog/rustycog-config/src/lib.rs
  - rustycog/rustycog-events/src/lib.rs
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-testing/src/common/kafka_testcontainer.rs
  - rustycog/rustycog-testing/src/common/sqs_testcontainer.rs
  - hive-events/README.md
summary: >-
  Source summary for the shared Rust SDK crates and event-contract packages that give services a common runtime, transport, and testing foundation.
provenance:
  extracted: 0.86
  inferred: 0.09
  ambiguous: 0.05
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Platform Building Blocks

These sources cover the reusable foundation beneath the application services: `[[projects/rustycog/rustycog]]` for implementation patterns and `[[projects/hive-events/hive-events]]` for event contracts.

## Key Ideas

- The root workspace includes most RustyCog crates directly, while `rustycog/Cargo.toml` also publishes a `rustycog-meta` umbrella package that groups them for consumers.
- RustyCog covers more than raw transport. The current code spreads shared concerns across config loading, command execution, HTTP startup, permissions, event publishing, logging, DB access, and integration testing.
- `QueueConfig` and the event factories support Kafka, SQS, or disabled/no-op transports from the same runtime abstraction, while the test harness provisions Kafka and LocalStack fixtures to verify those paths.
- Hive Events provides queue names and JSON-serializable payload contracts for organization-domain integration flows.
- Together they show that AIForAll is investing both in shared runtime primitives and in shared message schemas.
- The README-level packaging story is slightly uneven with the checked-in code, because the docs still mention macros/examples that are not visible in this tree. ^[ambiguous]

## Open Questions

- The current sources do not yet connect every consuming service back to the exact SDK crates and event contracts it uses.
- The code supports both Kafka and SQS, but the wiki still does not map which services standardize on which transport in production. ^[ambiguous]
- More service-specific sources will be needed to map actual adoption depth across the workspace.

## Sources

- [[projects/rustycog/rustycog]] — Shared SDK project
- [[projects/hive-events/hive-events]] — Event-contract project
- [[projects/rustycog/references/rustycog-crate-catalog]] — Code-backed inventory of the RustyCog crates
- [[concepts/shared-rust-microservice-sdk]] — Cross-project abstraction distilled from these sources