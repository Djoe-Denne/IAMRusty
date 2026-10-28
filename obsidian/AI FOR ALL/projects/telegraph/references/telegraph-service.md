---
title: Telegraph Service
category: references
tags: [reference, communication, architecture, visibility/internal]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - README.md
  - services/Telegraph/setup/src/app.rs
  - services/Telegraph/infra/src/event/consumer.rs
  - services/Telegraph/http/src/lib.rs
  - services/Telegraph/configuration/src/lib.rs
summary: Telegraph composition root, atomic notification/outbox writer, supervised dispatcher and owned HTTP listener, with disabled transport preserving intentions and HTTP availability.
provenance:
  extracted: 0.87
  inferred: 0.07
  ambiguous: 0.06
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-10-27T19:27:11Z
---

# Telegraph Service

This page is the Telegraph-specific companion to `[[projects/rustycog/references/index]]` and `[[references/rustycog-service-construction]]`. It skips the generic RustyCog service shell and keeps the parts of `[[projects/telegraph/telegraph]]` that are meaningfully different.

## RustyCog Baseline

- `[[projects/rustycog/references/index]]` is the shared reference map for the service layout, command runtime, config loading, HTTP shell, queue transport, and testing harness this page assumes.
- `[[references/rustycog-service-construction]]` and `[[skills/building-rustycog-services]]` cover the generic composition-root story that Telegraph reuses.
- `[[projects/rustycog/references/rustycog-command]]`, `[[projects/rustycog/references/rustycog-config]]`, `[[projects/rustycog/references/rustycog-events]]`, and `[[projects/rustycog/references/rustycog-http]]` explain the shared primitives that are not repeated here.

## Service-Specific Differences

- Telegraph keeps the same broad crate split used elsewhere in the repo, but its composition root is specialized around email, templates, notifications, and event-consumer wiring rather than CRUD-style HTTP only.
- `setup/src/app.rs` creates the email adapter and service, template service, database pool, repositories, notification service, permission fetcher, communication factory, event processor, command registry, command service, and final event consumer.
- `TelegraphApp::run()` supervises business background tasks alongside an explicitly owned HTTP server. The consumer and outbox dispatcher are distinct from the HTTP owner; parent completion alone does not attest descendant termination.
- The notification service delegates both creation shapes to the shared writer transaction: notification, optional delivery and one outbox intention commit together. No direct network publish after commit determines the business result.
- Dispatch is gated on a Live publisher with concrete non-NoOp transport. Disabled/degraded transport preserves intentions and HTTP availability; it must not turn NoOp into fictitious published rows.
- Standalone and the monolith adopt [[projects/rustycog/references/rustycog-http|owned HTTP shutdown]] under the shared 5 s grace/6 s absolute stop-and-join budget. Local shutdown does not release a durable SQL lease by itself.
- HTTP exposure is intentionally narrow: the live server only wires notification read-model routes, while richer communication DTOs remain present in code but not in the active route table. ^[ambiguous]
- The root repo README describes Telegraph as the platform communication service for emails, notifications, and SMS, but the service-level code currently resolves that promise mainly through email plus notification flows. ^[ambiguous]

## Open Questions

The October implementation and its scoped proofs are in [[projects/aiforall/references/mission-telegraph-outbox-20261027]]. This is delivered code, not an automatic promotion of ADR0313/0503 from Proposed or ADR0301 from Partial.

- Telegraph has no project-local README in this source tree, so the top-level repo README and the code are doing most of the documentation work today.
- The queue consumer is arguably the service's primary ingress path, but the current codebase does not declare one canonical “main interface” between HTTP and queue processing. ^[inferred]

## Sources

- [[projects/telegraph/telegraph]] - Main project overview.
- [[projects/telegraph/concepts/queue-driven-command-processing]] - How async events are pushed through the command runtime.
- [[projects/telegraph/references/telegraph-runtime-and-configuration]] - Runtime config, queue routing, templates, and local deployment details.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] - Authenticated route surface and ownership model.
- [[projects/rustycog/references/index]] - Crate-level map for the shared SDK surfaces Telegraph composes.
