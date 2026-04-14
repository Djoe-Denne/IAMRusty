---
title: Manifesto Event Model
category: references
tags: [reference, events, projects, visibility/internal]
sources:
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/infra/src/event/event_adapter.rs
summary: Code-backed view of the events Manifesto emits today, including its own domain events and the separate apparatus event adapter.
provenance:
  extracted: 0.82
  inferred: 0.08
  ambiguous: 0.10
created: 2026-04-14T20:25:00Z
updated: 2026-04-14T20:25:00Z
---

# Manifesto Event Model

`[[projects/manifesto/manifesto]]` is not merely event-ready in theory. The current use cases already publish Manifesto domain events at the application boundary, and the infra layer also carries a separate adapter for apparatus-style component events.

## Key Ideas

- Project flows publish `ProjectCreated`, `ProjectUpdated`, `ProjectDeleted`, `ProjectPublished`, and `ProjectArchived` after the corresponding state changes succeed.
- Component flows publish `ComponentAdded`, `ComponentStatusChanged`, and `ComponentRemoved` when components are attached, transitioned, or removed.
- Member flows publish `MemberAdded`, `MemberPermissionsUpdated`, `MemberRemoved`, `PermissionGranted`, and `PermissionRevoked`.
- Event publishing is best-effort in the current use cases: failures are logged with `tracing::warn!` but do not abort the main business transaction.
- `infra/src/event/event_adapter.rs` introduces a second vocabulary through `ApparatusDomainEvent::ComponentStatusChanged`, which suggests Manifesto participates in both a Manifesto-local event model and a broader component/appartus integration model. ^[ambiguous]
- The coexistence of `manifesto_events` in the application layer and `apparatus_events` in the adapter layer is a meaningful service-specific detail, not just generic RustyCog plumbing.

## Open Questions

- Where exactly should the apparatus adapter be wired into the live runtime, and which workflows are expected to use it instead of direct Manifesto domain events? ^[ambiguous]
- Should best-effort event publishing remain the default, or should some lifecycle transitions become hard-fail when publication breaks? ^[inferred]

## Sources

- [[projects/manifesto/manifesto]] - Service overview and runtime context.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Route and use-case entrypoints that trigger these events.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Project lifecycle transitions and their emitted events.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - Component-side behavior around add/remove/status changes.
- [[concepts/event-driven-microservice-platform]] - Platform-wide async coordination context.
