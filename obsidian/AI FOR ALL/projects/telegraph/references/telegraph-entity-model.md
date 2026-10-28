---
title: Telegraph Entity Model
category: references
tags: [reference, entities, communication, visibility/internal]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - services/Telegraph/domain/src/entity/communication.rs
  - services/Telegraph/domain/src/entity/delivery.rs
  - services/Telegraph/domain/src/entity/template.rs
summary: Telegraph communication and delivery entities, plus the atomic writer boundary linking notification, optional delivery and an infrastructure outbox intention.
provenance:
  extracted: 0.90
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-14T20:28:20.9129598Z
updated: 2026-10-27T19:27:11Z
---

# Telegraph Entity Model

This page lists the main entities `[[projects/telegraph/telegraph]]` owns in its communication domain.

## Key Entities

- `Communication` is the high-level message entity and currently branches into email and notification variants.
- `CommunicationRecipient` is embedded in those payloads so a message can target a user ID, an email address, or both.
- `CommunicationDescriptor` and its per-mode subtypes load descriptor metadata that steers how user-facing messages are built.
- `MessageTemplate` stores reusable content plus placeholder rendering logic.
- `NotificationCommunication` is the user-visible read model exposed by Telegraph's HTTP API, while `MessageDelivery` and `DeliveryAttempt` track provider-facing execution details and retries behind the scenes.
- `MessageDelivery` and `DeliveryAttempt` track provider-facing execution, retries, outcomes, and delivery telemetry.

## Atomic write boundary

The October delivery commits the notification, its optional delivery row and one `NotificationCreated` intention in the same writer transaction. The outbox is infrastructure durability, not a new public domain entity or a reason to expose SeaORM through the domain.

The notification-only component proof expects notification/delivery/outbox **1/0/1**; the full IAM-to-notification vertical expects **1/1/1**. Reconstructing the publisher keeps the original rows, event identity and payload instead of recreating them. See [[projects/telegraph/concepts/durable-notification-publication]] and [[projects/aiforall/references/mission-telegraph-outbox-20261027]].

## Open Questions

- The entity layer still carries SMS-oriented template content, but the live runtime and wiki emphasize email and notification flows more heavily. ^[ambiguous]

## Sources

- [[entities/communication]] - Canonical communication-side entity page.
- [[projects/telegraph/references/telegraph-event-processing]] - Runtime path that fills and sends these entities.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] - API surface exposing the notification part of the model.
- [[projects/telegraph/concepts/descriptor-driven-communications]] - Descriptor pattern that shapes the communication entities.
