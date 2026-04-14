---
title: Structured Service Configuration
category: concepts
tags: [configuration, env, rust, visibility/internal]
sources:
  - IAMRusty/docs/DATABASE_CONFIGURATION.md
  - IAMRusty/configuration/src/lib.rs
  - IAMRusty/config/default.toml
  - IAMRusty/config/test.toml
  - Telegraph/configuration/src/lib.rs
  - Telegraph/config/default.toml
  - Telegraph/config/development.toml
  - Telegraph/config/test.toml
  - Hive/configuration/src/lib.rs
  - Hive/config/default.toml
  - Hive/config/development.toml
  - Hive/config/test.toml
summary: AIForAll services use typed config loaders, but IAMRusty, Telegraph, and Hive diverge in env prefixes, queue models, and service-specific config sections.
provenance:
  extracted: 0.71
  inferred: 0.13
  ambiguous: 0.16
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Structured Service Configuration

Across `[[projects/iamrusty/iamrusty]]`, `[[projects/telegraph/telegraph]]`, and `[[projects/hive/hive]]`, configuration is treated as typed runtime state rather than a loose collection of env vars. All three services build on shared `rustycog-config` loaders, but they organize service-specific concerns differently.

## Key Ideas

- IAMRusty's `AppConfig` combines server, database, OAuth, JWT, logging, command, queue, and legacy Kafka sections into one loadable structure under the `IAM` env prefix.
- Telegraph's `TelegraphConfig` uses the `TELEGRAPH` env prefix and layers service-specific `queues` and `communication` blocks on top of shared `ServerConfig`, `QueueConfig`, `DatabaseConfig`, and logging traits.
- Hive's `AppConfig` uses the `HIVE` env prefix and keeps the shared `QueueConfig` shape, but adds outbound `iam_service`, `external_provider_service`, and `command` sections for an HTTP-first service that publishes organization events.
- In both services, environment-specific TOML files plus `RUN_ENV`-selected loading let tests use `port = 0` patterns for DB or queue dependencies without hardcoding ports.
- Telegraph separates queue transport (`queue`) from event routing (`queues.*.events`, per-event `modes`, optional `template` names), which gives it a more communication-pipeline-specific config shape than IAMRusty's single `AppConfig` pattern.
- Hive's config also includes `command.retry`, but unlike IAMRusty's current documented runtime the live composition path does not obviously bind that retry config into the registry. Conflict to resolve. ^[ambiguous]
- Conflict to resolve: IAMRusty consolidates queue/runtime policy into one service config model, Telegraph adds a second queue-routing schema and channel-specific `communication.*` sections, and Hive keeps one queue block but adds explicit outbound service sections. All three `rustycog-config` service shapes coexist today. ^[ambiguous]
- Telegraph's `config/default.toml` documents `[communication.sms]`, but `CommunicationConfig` currently includes `email`, `notification`, and `template` only. Conflict to resolve. ^[ambiguous]

## Open Questions

- Root docs and service-local docs still mix multiple operator-facing stories: IAMRusty's docs drift between `APP_` and `IAM_`, and Telegraph's top-level README port story does not match its local compose file. ^[ambiguous]
- Hive's default config points both `iam_service` and `external_provider_service` at `localhost:8080`, which is operationally ambiguous until environment conventions make those dependencies distinct. ^[ambiguous]
- Telegraph already makes `template_dir` configurable, but descriptor loading is still hardcoded in setup rather than being part of the config model. ^[inferred]

## Sources

- [[projects/iamrusty/iamrusty]] - Service using the `IAM`-prefixed `AppConfig` variant.
- [[projects/telegraph/telegraph]] - Service using `TELEGRAPH` plus queue-routing and communication sections.
- [[projects/hive/hive]] - Service using `HIVE` plus outbound IAM and external-provider sections.
- [[references/iamrusty-runtime-and-security]] - IAMRusty-specific runtime, JWT, and queue details.
- [[references/telegraph-runtime-and-configuration]] - Telegraph-specific queue, template, SMTP, and port behavior.
- [[references/hive-runtime-and-configuration]] - Hive-specific command, queue, and outbound service behavior.
- [[concepts/integration-testing-with-real-infrastructure]] - Real-infrastructure tests rely on these config shapes.