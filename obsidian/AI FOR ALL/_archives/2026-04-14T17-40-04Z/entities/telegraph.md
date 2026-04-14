---
title: >-
  Telegraph
category: entities
tags: [service, notifications, messaging, visibility/internal]
sources:
  - README.md
  - hive-events/README.md
summary: >-
  Telegraph is the communication service that turns platform events into email and notification workflows.
provenance:
  extracted: 0.74
  inferred: 0.16
  ambiguous: 0.10
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# Telegraph

Telegraph is the platform communication service. In the current source set it appears primarily as the consumer side of the `[[concepts/event-driven-microservice-platform]]`, receiving messages from services like `[[projects/iamrusty/iamrusty]]` and producers represented by `[[projects/hive-events/hive-events]]`.

## Key Ideas

- The top-level repository README describes Telegraph as responsible for emails, notifications, and SMS.
- Hive Events documents a notification queue whose messages are consumed by Telegraph and mapped to predefined templates.
- Telegraph is a key integration point that turns otherwise internal domain events into user-facing communication.
- The currently ingested docs say much more about what Telegraph consumes than about its internal implementation. ^[inferred]
- SMS support is named in the repository overview but not elaborated in the detailed event docs. ^[ambiguous]

## Open Questions

- No standalone Telegraph architecture or setup source has been ingested yet.
- Template management, retry behavior, and channel-specific delivery logic remain outside this first wiki pass.

## Sources

- [[references/aiforall-platform]] — Repo-level service overview
- [[references/platform-building-blocks]] — Queue-driven notification contract context
- [[projects/aiforall/aiforall]] — Platform page where Telegraph fits operationally
