---
title: >-
  Hive Events
category: project
tags: [events, sqs, integration, visibility/internal]
sources:
  - hive-events/README.md
summary: >-
  Hive Events is a shared crate of organization-domain event contracts and queue names used for inter-service communication.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# Hive Events

Hive Events is a contract crate for domain events emitted by the Hive organization domain. It strengthens the repo's `[[concepts/event-driven-microservice-platform]]` by defining payloads and queue-routing conventions that downstream services, including `[[entities/telegraph]]`, can consume.

## Key Ideas

- The crate groups events around organization lifecycle, members, invitations, and external integrations.
- Queue routing is explicit, with separate queues for organization state, notifications, and sync monitoring.
- The crate gives the platform a typed event surface even when the producer service itself is not co-located in this repository. ^[inferred]
- Its notification queue integration complements the higher-level platform picture in `[[projects/aiforall/aiforall]]`.

## Open Questions

- The Hive service itself is not documented in this source batch, so the full producer-side workflow is only partially visible.
- The relationship between Hive project/org events and `[[projects/manifesto/manifesto]]` membership cascading is architectural rather than implementation-level in the current docs.

## Sources

- [[references/platform-building-blocks]] — Shared SDK and event-contract building blocks
