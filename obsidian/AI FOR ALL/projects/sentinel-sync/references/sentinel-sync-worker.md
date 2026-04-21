---
title: Sentinel Sync Worker
category: reference
tags: [reference, sentinel-sync, authorization, openfga, events]
sources:
  - sentinel-sync/src/main.rs
  - sentinel-sync/src/handler.rs
  - sentinel-sync/src/fga_client.rs
  - sentinel-sync/src/translator/mod.rs
  - sentinel-sync/src/idempotency.rs
  - sentinel-sync/src/config.rs
summary: The sentinel-sync worker consumes per-service domain events, runs each through a translator that emits OpenFGA tuple writes/deletes, and applies them through an atomic Write call. An event ledger guarantees idempotent processing.
updated: 2026-04-20
---

# Sentinel Sync Worker

`sentinel-sync` is a Rust binary (crate `sentinel-sync/`) that bridges the existing event bus to the centralized OpenFGA store.

## Layout

- `main.rs` — boots logging, loads config, builds the OpenFGA write client, the idempotency ledger, the translator list, and the concrete event consumer; waits on SIGINT.
- `config.rs` — `SentinelSyncConfig` with `logging`, `queue` (shared RustyCog types), plus the worker-specific `openfga` and `idempotency` sections. Loads from `config/sentinel-sync.toml` and `SENTINEL_SYNC__*` env vars.
- `fga_client.rs` — minimal `reqwest`-based OpenFGA HTTP client covering `POST /stores/{id}/write` (atomic writes + deletes) and the `Tuple` helper types.
- `idempotency.rs` — `EventLedger` trait plus `InMemoryEventLedger`. Postgres backend is reserved (`idempotency.backend = "postgres"` returns an error until implemented).
- `translator/` — one module per producer service (`hive`, `manifesto`, `iam`). Each implements `Translator::translate(raw_event) -> Option<TupleDelta>`.
- `handler.rs` — `SyncEventHandler` dispatches one event: records in the ledger, tries translators in order, applies the resulting delta atomically.

## Data flow

```mermaid
flowchart LR
    Queue["Kafka/SQS queue"] --> Consumer["rustycog-events EventConsumer"]
    Consumer --> Handler["SyncEventHandler"]
    Handler --> Ledger["EventLedger.record(event_id)"]
    Ledger -->|first time| Translate["Translator.translate(raw)"]
    Translate -->|"TupleDelta"| FGA["OpenFGA /write (atomic)"]
    Ledger -->|duplicate| Skip["no-op"]
```

## Configuration

```toml
[openfga]
api_url = "http://localhost:8080"
store_id = "01HX..."
authorization_model_id = "01HX..."   # optional
api_token = "..."                    # optional

[idempotency]
backend = "in-memory"                # or "postgres" (planned)

[queue]
kind = "kafka"                       # shared RustyCog QueueConfig
# ...
```

Sample file: [sentinel-sync/config/sentinel-sync.toml.example](../../../../sentinel-sync/config/sentinel-sync.toml.example).

## Idempotency

`SyncEventHandler` records every `event_id` in the ledger before processing. A duplicate record is treated as "already applied" and silently skipped. Retried deliveries and replays are safe.

The in-memory ledger is appropriate for tests and local runs; the planned Postgres backend will store `(event_id, processed_at)` rows in a dedicated schema next to OpenFGA's datastore so restarts are safe too.

## Translators

Each translator decodes the raw JSON into the service's `DomainEvent` enum. Decoding failure yields `None` (the event belongs to another service). A successful decode returns a `TupleDelta { writes, deletes }`. An empty delta is valid — most domain events are not authorization-relevant.

The concrete event-to-tuple mappings live in [[projects/sentinel-sync/references/event-to-tuple-mapping]].

## Related

- [[projects/sentinel-sync/sentinel-sync]]
- [[projects/sentinel-sync/references/openfga-model]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[concepts/openfga-as-authorization-engine]]
- [[entities/relation-tuple]]
