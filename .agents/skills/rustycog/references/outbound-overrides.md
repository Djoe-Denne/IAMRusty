# Outbound overrides (composition root)

Typed bag for host-injected outbound **RPC** adapters. Not domain events.

Canon: `docs/adr/0104-outbound-overrides-composition-root.md` (Proposed / Partial). Clarifies 0102; does **not** supersede it. Dual runtime: 0404. Hive→IAM instance: 0306. HTTP credential: 0307.

## When to apply

- New cross-hexagon **outbound port** (this service’s setup calls another bounded context).
- Wiring `oodhive-monolith` InProcess vs standalone HTTP.
- Touching `*OutboundOverrides`, `AppBuilder::with_outbound`, named sugar setters, or `monolith/src/in_process_*.rs`.

## Rules (do this)

1. **Bag lives on the consumer `*-setup` crate** (`HiveOutboundOverrides`, `LazaretOutboundOverrides`, …). Named `Option<Arc<dyn Port>>` fields typed from the **consumer** domain. `Default` = all `None` = HTTP clients built by that setup.
2. **One object into the composition root:** `AppBuilder::with_outbound(bag)`. Named setters (`with_iam_organization_signer_client`, `with_binding_grant_snapshots`) are **sugar** that fill the bag. New host wiring: prefer `with_outbound`. Keep existing sugar.
3. **Only `oodhive-monolith` builds InProcess bridges**, in `monolith/`, from **provider** setup façades (`organization_signer()`, `binding_grant_snapshots()`), then fills the consumer bag **before** `build`. Microservice binary and IT: empty bag.
4. **Fail-closed in the monolith:** missing provider façade → refuse boot. No HTTP fallback inside the host (`monolith/src/runtime.rs`).
5. **InProcess = capability** (`Arc<dyn Port>`), not an internal token. HTTP (standalones **and** nested internal routes) uses fail-closed workload credential (0307). **Credential 0307 = HTTP path only.**
6. Resolve in consumer setup: `Some(injected)` wins; else construct the HTTP adapter from consumer config.

## Forbidden

- DI container / service locator
- `HashMap` / `Any` override map
- Consumer `*-application` / `*-infra` / `*-setup` importing **provider** crates
- Config boolean `in_process`
- Putting `EventPublisher` / outbox / queue consumers in the bag (different seam — below)

## Adding a new outbound port

1. Add `Option<Arc<dyn ConsumerPort>>` on the **consumer** bag.
2. Resolve injected vs HTTP in that setup (0307 credential only on HTTP).
3. Expose a getter on the **provider** `Application` — no shared crate.
4. In `monolith/`: bridge the consumer port over the façade; fail-closed if `None`; pass `Some` via `with_outbound`.
5. Leave standalone `main` and IT on `Default`.

## Proven instances (Partial)

| Consumer bag | Field | Provider façade | Host bridge |
|---|---|---|---|
| `HiveOutboundOverrides` | `iam_organization_signer` | `iam_app.organization_signer()` | `monolith/src/in_process_iam_signer.rs` |
| `LazaretOutboundOverrides` | `binding_grant_snapshots` | `manifesto_app.binding_grant_snapshots()` | `monolith/src/in_process_binding_grant.rs` |

Hive runtime currently uses sugar `with_iam_organization_signer_client`; Lazaret uses `with_outbound`. Both are valid.

## Not this seam — domain events

Do **not** merge events into 0104.

Each setup builds its own `EventPublisher` via `create_signaled_multi_queue_event_publisher` + `OutboxDispatcher` from `QueueConfig` (SQS / Kafka / NoOp). See 0300 (contract, no transport in the crate), 0301 (outbox), 0202 (queues opt-in). `maybe_event_publisher` on IAM/Manifesto is a **test hook**; `monolith/src/runtime.rs` passes `None`. There is no `DirectEventForwarder` / `InProcessEventPublisher` / `LocalEventBus`. `MockEventPublisher` is test-only (0202), not a monolith bus.

Details: [using-rustycog-events.md](using-rustycog-events.md).
