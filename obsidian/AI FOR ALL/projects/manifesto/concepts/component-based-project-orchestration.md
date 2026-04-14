---
title: >-
  Component-Based Project Orchestration
category: concepts
tags: [projects, components, orchestration, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - docs/project/Archi.md
  - Manifesto/IMPLEMENTATION_STATUS.md
summary: >-
  Projects are modeled as shells that orchestrate independently implemented components through contracts, registries, and lifecycle states, though some of that architecture is still aspirational.
provenance:
  extracted: 0.61
  inferred: 0.13
  ambiguous: 0.26
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T20:08:52.0803248Z
---

# Component-Based Project Orchestration

The project-service architecture described for `[[projects/manifesto/manifesto]]` treats a project as an orchestrator over independently implemented component services rather than a monolith that owns every feature end to end.

## Key Ideas

- Projects move through explicit lifecycle states, while attached components have their own status progression from pending to active.
- Component services are expected to expose a shared contract for manifest, configuration, validation, activation, and health.
- The ADR favors a Redis-backed registry plus self-registration so components can be discovered dynamically.
- Configuration ownership stays with the component service, while Manifesto tracks overall project and component state.
- Cross-domain authorization for operations like project creation is described through signed impersonation tokens rather than direct cross-service trust.
- The current code and implementation-status docs show a project/component/member MVP with real CRUD, permission checks, and migrations, but they do not demonstrate the full registry and impersonation model described in the ADR. Conflict to resolve. ^[ambiguous]
- In practice, Manifesto already acts as the orchestration shell for project records and component attachments, while the broader component ecosystem remains partly blueprint-level. ^[inferred]

## Open Questions

- Which pieces of the registry and impersonation design are already implemented versus still planned? Conflict to resolve. ^[ambiguous]
- The docs outline how component autonomy should work, but not yet the operational policy for failure handling across components.

## Sources

- [[projects/manifesto/references/manifesto-service]] — Manifesto model, setup, and ADR details
- [[projects/manifesto/manifesto]] — Service overview anchored to this orchestration model
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern used for cascading changes
