---
title: AIForAll Roadmap
category: roadmap
tags: [platform, testing, database, events, visibility/internal]
summary: >-
  Near-term roadmap for AIForAll: Sentinel Sync service tests, verification of transactional DB load, and a RustyCog Events outbox pattern.
status: planned
created: 2026-04-25T11:42:00Z
updated: 2026-04-25T11:42:00Z
---

# AIForAll Roadmap

This roadmap captures the next platform functionality focus. The immediate direction is to turn the current service and event plumbing into verified behavior: authorization sync must be tested, database transaction behavior must be measured, and event publication must survive domain-write failure modes.

## Near-Term Focus

### Sentinel-5 / Sentinel Sync service tests

The first focus area is the test strategy for [[projects/sentinel-sync/sentinel-sync|Sentinel Sync]], especially the worker path that consumes service events and writes OpenFGA relation tuples.

Focus:

- Cover the `sentinel-sync` translator and handler boundaries with realistic service events.
- Verify the event-to-tuple mapping documented in [[projects/sentinel-sync/references/event-to-tuple-mapping]].
- Exercise idempotency so repeated `event_id` values do not duplicate relation writes.
- Prefer real protocol fixtures where the failure mode matters, especially queue delivery and OpenFGA writes.

Done means the service has confidence-building tests around the [[projects/sentinel-sync/references/sentinel-sync-worker]] path: valid events create or delete the expected tuples, unsupported or malformed events fail predictably, and repeated delivery is safe.

### Transaction load verification in the DB model

The second focus area is proving how the database model behaves under transactional load. This is about validating the practical limits and guarantees of the current persistence design rather than assuming that pool configuration and schema shape are sufficient.

Focus:

- Define representative transactional scenarios for the core service write paths.
- Measure contention, connection-pool behavior, and read/write routing through [[projects/rustycog/references/rustycog-db]] and [[entities/db-connection-pool]].
- Confirm that transactions preserve domain invariants under concurrent writes.
- Capture bottlenecks as concrete schema, query, transaction-boundary, or pool-tuning changes.

Done means the team has repeatable evidence for expected transactional load, known failure thresholds, and a short list of changes needed before higher-volume workflows depend on the model.

### RustyCog Events outbox pattern

The third focus area is adding an outbox pattern to [[projects/rustycog/references/rustycog-events|RustyCog Events]]. The current event publisher abstraction supports transport selection and SQS fanout, but domain writes still need a durable bridge between database commits and event dispatch.

Focus:

- Introduce an outbox record that is written in the same database transaction as the domain change.
- Add a dispatcher that reads pending outbox records and publishes through [[entities/event-publisher]].
- Make delivery idempotent at the event-envelope level so retries are safe.
- Define retry, dead-letter, and observability expectations before treating no-op fallback as acceptable behavior.
- Keep service code transport-agnostic, with RustyCog owning the shared outbox mechanics.

Done means a service can commit a domain change and its corresponding event atomically, then dispatch the event asynchronously without losing it when process, network, or queue setup failures happen between commit and publish.

## Roadmap Shape

The workstreams reinforce each other:

- Sentinel Sync tests prove that downstream authorization state can be rebuilt from events.
- Transaction load verification proves that upstream domain writes hold under concurrency.
- The RustyCog Events outbox pattern connects those two guarantees by making event publication durable.

Together, these features move AIForAll toward a more reliable event-driven platform: services own their data, events carry the integration contract, and authorization state remains synchronized through tested, repeatable infrastructure.

## Related Notes

- [[projects/aiforall/aiforall]]
- [[concepts/event-driven-microservice-platform]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[projects/sentinel-sync/sentinel-sync]]
- [[projects/rustycog/references/rustycog-db]]
- [[projects/rustycog/references/rustycog-events]]
