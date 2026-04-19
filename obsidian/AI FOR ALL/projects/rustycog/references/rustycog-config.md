---
title: RustyCog Config
category: references
tags: [reference, rustycog, configuration, visibility/internal]
sources:
  - rustycog/rustycog-config/src/lib.rs
summary: rustycog-config provides typed config primitives, loader traits, queue/database/server models, and RUN_ENV-aware loading behavior for services.
provenance:
  extracted: 0.86
  inferred: 0.06
  ambiguous: 0.08
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Config

`rustycog-config` is the typed configuration foundation for `[[projects/rustycog/rustycog]]` services.

## Key Ideas

- The crate defines shared runtime structs (`ServerConfig`, `DatabaseConfig`, `LoggingConfig`, `CommandConfig`, `KafkaConfig`, `SqsConfig`, and `QueueConfig`).
- `ConfigLoader` and `ConfigCache` traits let each service keep its own `AppConfig` while reusing one loading/caching mechanism.
- `load_config_fresh()` and `load_config_with_cache()` choose config files from `RUN_ENV` and apply env overrides via service-specific prefixes.
- `load_config_part("server")` and similar helpers load one section at a time, but they use section-based env prefixes (`SERVER_*`, `QUEUE_*`, and so on).
- Queue support is transport-polymorphic through `QueueConfig::{Kafka,Sqs,Disabled}` so event code can switch transports without changing high-level calling code.
- Random-port caching in DB/Kafka/SQS config makes test runs stable once a random port is resolved for the process.

## Linked Entities

- [[entities/queue-config]]
- [[entities/db-connection-pool]]

## Open Questions

- The SQS URL builder currently uses a Scaleway-style host format while other settings and naming remain AWS-oriented. Conflict to resolve. ^[ambiguous]
- The loader path still does not auto-merge a universal `config/default.toml` baseline before environment-specific files. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[concepts/structured-service-configuration]]
- [[projects/rustycog/rustycog]]
