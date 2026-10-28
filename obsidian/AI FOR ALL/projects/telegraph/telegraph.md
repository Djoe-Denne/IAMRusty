---
title: >-
  Telegraph
category: project
tags: [communication, events, notifications, visibility/internal]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - README.md
  - services/Telegraph/openspecs.yaml
  - services/Telegraph/setup/src/app.rs
  - services/Telegraph/infra/src/event/consumer.rs
  - services/Telegraph/http/src/lib.rs
summary: >-
  Telegraph : notifications durables, outbox transactionnelle et HTTP possédé ; preuves SQS/OpenFGA/replay/403/200 livrées, limites et statuts ADR préservés.
provenance:
  extracted: 0.83
  inferred: 0.09
  ambiguous: 0.08
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-10-27T19:27:11Z
---

# Telegraph

## Indexes

- [[projects/telegraph/concepts/index]] — concepts
- [[projects/telegraph/skills/index]] — skills
- [[projects/telegraph/references/index]] — references

`Telegraph` is the communication service in the AIForAll workspace. Use `[[projects/rustycog/references/index]]` for the shared service shell; use this page and the linked Telegraph references for the event-routing, descriptor, delivery, and notification behaviors that Telegraph adds on top of that baseline.

## RustyCog Baseline

- `[[projects/rustycog/references/index]]` is the canonical map for the shared command, config, HTTP, permission, event, DB, and testing crates used by Telegraph.
- `[[references/rustycog-service-construction]]` and `[[skills/building-rustycog-services]]` describe the default RustyCog service assembly flow that Telegraph reuses.
- Read `[[projects/rustycog/references/rustycog-command]]`, `[[projects/rustycog/references/rustycog-config]]`, `[[projects/rustycog/references/rustycog-events]]`, `[[projects/rustycog/references/rustycog-http]]`, and `[[projects/rustycog/references/rustycog-testing]]` for the shared runtime mechanics.

## Service-Specific Differences

- Telegraph has queue and authenticated HTTP entry paths. Since the October delivery, a supervised outbox dispatcher publishes durable notification intentions while the HTTP listener has an explicit owner; these are distinct lifecycles.
- Event payloads are transformed into user-facing output through `[[projects/telegraph/concepts/descriptor-driven-communications]]`, where per-event TOML descriptors and Tera templates determine whether an event yields email, in-app notification content, or both.
- Queue events flow through `[[projects/telegraph/concepts/queue-driven-command-processing]]`, so async consumers and HTTP handlers both delegate into the same RustyCog command runtime instead of bypassing it.
- The notification API follows `[[concepts/centralized-authorization-service]]`: the route layer checks `notification#recipient` through OpenFGA. Telegraph now records `NotificationCreated` atomically; the real queued event passes through [[projects/sentinel-sync/sentinel-sync]] to produce the recipient tuple. There is no permission-middleware fallback that bypasses a deny.
- [[projects/telegraph/concepts/durable-notification-publication]] describes the writer transaction, suspended NoOp transport and lease-based recovery. The delivered SDK/pin and separate component/vertical proofs are recorded in [[projects/aiforall/references/mission-telegraph-outbox-20261027]].
- Runtime behavior depends on `[[concepts/structured-service-configuration]]`, especially the split between transport-level `queue` settings and Telegraph-specific `queues.*` event routing and `communication.*` delivery settings.
- The root repo overview and some Telegraph config/model surfaces still describe SMS alongside email and notifications, but the currently wired processor composite only registers email and notification handlers. ^[ambiguous]
- Queue command failures still flatten to `ServiceError::infrastructure` — the only **divergent** error verdict in [[concepts/architecture-coherence-across-services]].
- Queue boot should go through [[projects/aiforall/concepts/queue-readiness-signaling]] so a rustycog no-op is visible on `/ready`.
- Integration tests: one `#[path = "fixtures/mod.rs"]` in `tests/common.rs` (Sonar `duplicate_mod`). See [[projects/aiforall/skills/fixing-sonar-clippy-in-services]].
- T14b (HEAD `a27ea5b`) : HTTPS compose via dual-bind rustycog, hôte **8445**, CA mesh distincte de Lazaret, client cert **optionnel**. Preuve `services/Telegraph/tests/https_mesh_optional_mtls.rs`. Concept : [[projects/aiforall/concepts/https-platform-mesh]].

## Related

- [[projects/rustycog/references/index]] - Canonical shared framework map that the service pages below build on.
- [[projects/telegraph/references/telegraph-service]] - Code-backed overview of Telegraph's crate layout, shared dependencies, and parallel runtime shape.
- [[projects/telegraph/references/telegraph-entity-model]] - Communication, template, and delivery entities owned by Telegraph.
- [[projects/telegraph/references/telegraph-runtime-and-configuration]] - `TELEGRAPH_*` config loading, queue routing, template paths, and local runtime drift.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] - The live notification route table, ownership checks, and OpenAPI drift.
- [[projects/telegraph/references/telegraph-event-processing]] - SQS consumption, command dispatch, descriptor loading, and delivery-mode routing.
- [[projects/telegraph/references/telegraph-testing-and-smtp-fixtures]] - Real SQS, SMTP, DB, and JWT-backed integration tests.
- [[projects/rustycog/references/rustycog-outbox]] — durable intention, finite retries and persistent eligibility.
- [[projects/rustycog/references/rustycog-http]] — caller-owned HTTP shutdown; Drop is not async join.
- [[projects/telegraph/skills/building-event-driven-notification-services]] - Reusable workflow for building Telegraph-style communication services.
- [[projects/aiforall/concepts/https-platform-mesh]] — T14b HTTPS + CA client optionnelle.

## Open Questions

- The root `README.md` says Telegraph runs on port `8081` in the shared stack, while `services/Telegraph/docker-compose.yml` exposes `8080:8080`. Conflict to resolve. ^[ambiguous]
- The repo overview says IAMRusty publishes to `user-events`, while Telegraph's own queue-routing examples are keyed under `test-user-events`; the naming split needs a single operator-facing story. Conflict to resolve. ^[ambiguous]
- `http/src/handlers/communication.rs` defines richer send-message DTOs, but the live route table only exposes notification read-model endpoints. ^[ambiguous]

## Sources

- [[projects/rustycog/references/index]]
- [[projects/telegraph/references/telegraph-service]]
- [[projects/telegraph/references/telegraph-runtime-and-configuration]]
- [[projects/telegraph/references/telegraph-http-and-notification-api]]
- [[projects/telegraph/references/telegraph-event-processing]]
- [[projects/telegraph/references/telegraph-testing-and-smtp-fixtures]]
