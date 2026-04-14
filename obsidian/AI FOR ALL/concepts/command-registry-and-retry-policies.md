---
title: Command Registry and Retry Policies
category: concepts
tags: [commands, reliability, rust, visibility/internal]
sources:
  - IAMRusty/docs/COMMAND_PATTERN.md
  - IAMRusty/docs/COMMAND_RETRY_CONFIGURATION.md
  - IAMRusty/application/src/command/factory.rs
  - IAMRusty/config/test.toml
  - Telegraph/application/src/command/factory.rs
  - Telegraph/setup/src/app.rs
  - Hive/application/src/command/factory.rs
  - Hive/setup/src/app.rs
  - Hive/config/default.toml
summary: Repo services use typed command registries to centralize handlers, but IAMRusty, Telegraph, and Hive diverge in retry wiring, registry breadth, and transport entrypoints.
provenance:
  extracted: 0.73
  inferred: 0.15
  ambiguous: 0.12
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Command Registry and Retry Policies

Across `[[projects/iamrusty/iamrusty]]`, `[[projects/telegraph/telegraph]]`, and `[[projects/hive/hive]]`, request handlers and queue consumers delegate into typed command registries instead of calling use cases directly. The shared `rustycog` command layer provides one orchestration surface, but the services configure and use that surface differently.

## Key Ideas

- IAMRusty's `CommandRegistryFactory::create_iam_registry` registers the service's auth-heavy command set and binds retry behavior from `CommandConfig`, so retry policy is part of the live runtime assembly.
- Telegraph's `TelegraphCommandRegistryFactory::create_telegraph_registry` registers `process_event`, `get_notifications`, `get_unread_count`, and `mark_notification_read`, then injects the resulting `GenericCommandService` into both `AppState` and the SQS-backed event consumer.
- Hive's `HiveCommandRegistryFactory::create_hive_registry` registers organization, member, invitation, external-link, and sync-job commands, then injects the resulting `GenericCommandService` into `AppState` for an HTTP-first service that also publishes domain events.
- In both services, command types are paired with dedicated handlers and error mappers, which keeps domain and infrastructure failures from leaking raw details into transport code.
- The command layer remains the main bridge between transport and use cases: IAMRusty uses it from HTTP handlers, Telegraph uses it from both HTTP handlers and queue-driven event handling through `[[concepts/queue-driven-command-processing]]`, and Hive uses it from HTTP handlers whose use cases then publish `[[projects/hive-events/hive-events]]` events.
- Conflict to resolve: IAMRusty explicitly configures registry retry behavior, while Telegraph and Hive currently build registries with plain `CommandRegistryBuilder::new()` and no visible service-specific retry binding even though Hive declares command retry config in TOML. Both `rustycog` usage patterns exist in the live repo. ^[ambiguous]
- IAMRusty's current test config sets `max_attempts = 0`, which already makes its live test retry posture stricter than many of the surrounding docs imply. ^[ambiguous]

## Open Questions

- Should HTTP-first services like Hive and queue-first services like Telegraph standardize on the same explicit registry retry configuration that IAMRusty binds through `CommandConfig`? Conflict to resolve. ^[ambiguous]
- Queue-driven command execution currently uses a thinner `CommandContext` than authenticated HTTP requests, so cross-service context conventions are still evolving. ^[inferred]

## Sources

- [[projects/iamrusty/iamrusty]] - HTTP-first service using the configured registry variant.
- [[projects/telegraph/telegraph]] - Queue-first service using the same command runtime differently.
- [[projects/hive/hive]] - HTTP-first organization service with a broader registry than its live route table.
- [[references/iamrusty-command-execution]] - Concrete IAMRusty registry composition and retry behavior.
- [[references/hive-command-execution]] - Hive registry composition and event-publishing use cases.
- [[concepts/queue-driven-command-processing]] - Telegraph's async command-dispatch variant.
- [[concepts/structured-service-configuration]] - Retry and registry wiring still depend on each service's config approach.