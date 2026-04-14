---
title: Hive Runtime and Configuration
category: references
tags: [reference, configuration, integrations, visibility/internal]
sources:
  - Hive/config/default.toml
  - Hive/config/development.toml
  - Hive/config/test.toml
  - Hive/configuration/src/lib.rs
  - Hive/setup/src/app.rs
  - Hive/infra/src/event/event_adapter.rs
summary: Hive uses HIVE-prefixed typed config for DB, IAM, external-provider, command, and queue settings, then wires a multi-queue event publisher from that runtime state.
provenance:
  extracted: 0.76
  inferred: 0.13
  ambiguous: 0.11
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Hive Runtime and Configuration

These sources describe how `[[projects/hive/hive]]` is configured and started: the `HIVE` env prefix, typed config loading, environment-specific TOML overrides, DB and queue behavior, and the outbound service settings Hive uses to talk to IAM and external-provider systems.

## Key Ideas

- `AppConfig` implements `rustycog_config::ConfigLoader` with the env prefix `HIVE` and includes `server`, `database`, `iam_service`, `external_provider_service`, `logging`, `scaleway`, `command`, and `queue` sections.
- Default config enables SQS-style queue publishing with `default_queue = "user-events"`, while development points at `localstack:4566` and test disables the queue entirely.
- Hive declares command retry settings in config, but dev and test set `max_attempts = 0` while default config uses `3`, so the live retry posture depends heavily on environment.
- `setup/src/app.rs` creates a `MultiQueueEventPublisher` from `config.queue` and `HiveErrorMapper`, so queue publishing is part of the normal runtime assembly rather than an optional bolt-on.
- Conflict to resolve: unlike `[[projects/telegraph/telegraph]]`, Hive does not add a second service-specific queue-routing schema on top of `QueueConfig`; unlike `[[projects/iamrusty/iamrusty]]`, it adds explicit outbound `iam_service` and `external_provider_service` blocks instead. Both are valid `rustycog-config` service shapes. ^[ambiguous]
- Conflict to resolve: both `iam_service` and `external_provider_service` default to `localhost:8080` in `config/default.toml`, which is an operator-facing ambiguity until a stronger environment story pins them to distinct services. ^[ambiguous]

## Open Questions

- The config crate comment says “IAM service configuration,” which is correct but easy to misread as imported service config instead of Hive's outbound dependency settings.
- Hive declares queue publishing even though its default tests run with queue disabled, so production and test integration stories differ more sharply than in Telegraph. ^[ambiguous]

## Sources

- [[projects/hive/hive]] - Project page for the service using this runtime shape.
- [[concepts/structured-service-configuration]] - Cross-service comparison of config loader patterns.
- [[references/hive-command-execution]] - Registry and event publisher wiring that depends on this config.
- [[concepts/external-provider-sync-jobs]] - Domain flows that consume the outbound service config.