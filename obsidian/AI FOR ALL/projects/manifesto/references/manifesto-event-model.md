---
title: Manifesto Event Model
category: references
tags: [reference, events, projects, visibility/internal]
sources:
  - Manifesto/setup/src/app.rs
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/infra/src/event/consumer.rs
  - Manifesto/infra/src/event/processors/component_processor.rs
summary: >-
  Manifesto publishes domain events from application flows. AuthZ mutation
  paths write outbox rows in the same UoW as the row change; inbound apparatus
  status still reconciles stored component state.
provenance:
  extracted: 0.89
  inferred: 0.07
  ambiguous: 0.04
created: 2026-04-14T20:25:00Z
updated: 2026-09-09T16:45:00Z
---

# Manifesto Event Model

`[[projects/manifesto/manifesto]]` has two live event behaviors today: it publishes Manifesto domain events from application flows, and it can consume apparatus component-status events to reconcile stored component state.

## Key Ideas

- Project flows publish `ProjectCreated`, `ProjectUpdated`, `ProjectVisibilityChanged` (only when visibility actually flips), `ProjectDeleted`, `ProjectPublished`, and `ProjectArchived`.
- Component flows publish `ComponentAdded`, `ComponentStatusChanged`, and `ComponentRemoved`.
- Member flows publish `MemberAdded`, `MemberPermissionsUpdated`, `MemberRemoved`, `PermissionGranted`, and `PermissionRevoked`.
- `setup/src/app.rs` injects the same `EventPublisher` into project, component, and member use cases, defaulting to a multi-queue publisher unless tests or alternate bootstraps override it.
- AuthZ-relevant mutations persist outbox events in the **same** unit of work as the row change (CAS-locked). A failed outbox insert rolls back the business write. Non-AuthZ publication may still be best-effort depending on the path. ^[inferred]
- Envelope and monotonic revision for sentinel-sync: [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]].
- `ApparatusEventConsumer` is constructed in setup and started alongside the HTTP server only when queue config resolves to a real consumer.
- `ComponentStatusProcessor` handles inbound `apparatus_events::ComponentStatusChangedEvent` messages by updating the matching stored component:
  - duplicates are treated as no-ops,
  - stale events are ignored,
  - applied timestamps use the event's `changed_at`.
- The old unused outbound apparatus adapter is no longer part of the live runtime. Outbound publication currently uses only Manifesto's own domain-event vocabulary.
- `update_project` emits `ProjectVisibilityChanged` (old + new) before `ProjectUpdated` when visibility changes. [[projects/sentinel-sync/sentinel-sync]] syncs `viewer@user:*` from that event only. `ProjectUpdated` stays a tuple no-op.
- `ProjectPublished` / `ProjectArchived` are lifecycle events. Publish does not write `user:*`. Archive still deletes the wildcard. See [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].

## Checked-In Queue Posture

- Checked-in `default`, `development`, and `test` configs all disable queues.
- That means local/test boots use no-op publisher/consumer behavior unless queue settings are explicitly overridden.
- Focused runtime tests also cover the enabled-config path falling back to a safe no-op consumer when no broker fixture is present.

## Open Questions

- Visibility flips emit `ProjectVisibilityChanged`; `ProjectPublished` no longer writes `viewer@user:*` (answered 2026-09-02).
- AuthZ mutation outbox is now in-transaction (answered 2026-09-09). See [[projects/manifesto/concepts/membership-restore-and-cas]].
- If queue-backed operation becomes a default CI path later, which event contracts deserve end-to-end broker coverage instead of unit-level runtime tests?

## Sources

- [[projects/manifesto/manifesto]] - Service overview and runtime context.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Route and use-case entrypoints that trigger these events.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Project lifecycle transitions and their emitted events.
- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]] - Envelope, v1 no-op, monotonic ledger.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] - Publish vs public; visibility event now live.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - Component-side validation and runtime status updates.
- [[concepts/event-driven-microservice-platform]] - Platform-wide async coordination context.
