---
title: >-
  Component-Based Project Orchestration
category: concepts
tags: [projects, components, orchestration, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - docs/project/Archi.md
summary: >-
  Projects are modeled as shells that orchestrate independently implemented components through contracts, registries, and lifecycle states.
provenance:
  extracted: 0.70
  inferred: 0.18
  ambiguous: 0.12
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# Component-Based Project Orchestration

The project-service architecture described for `[[projects/manifesto/manifesto]]` treats a project as an orchestrator over independently implemented component services rather than a monolith that owns every feature end to end.

## Key Ideas

- Projects move through explicit lifecycle states, while attached components have their own status progression from pending to active.
- Component services are expected to expose a shared contract for manifest, configuration, validation, activation, and health.
- The ADR favors a Redis-backed registry plus self-registration so components can be discovered dynamically.
- Configuration ownership stays with the component service, while Manifesto tracks overall project and component state.
- Cross-domain authorization for operations like project creation is described through signed impersonation tokens rather than direct cross-service trust.
- Some of these ideas appear in the ADR as target architecture while the implementation docs focus on the current MVP surface. ^[ambiguous]

## Open Questions

- Which pieces of the registry and impersonation design are already implemented versus still planned?
- The docs outline how component autonomy should work, but not yet the operational policy for failure handling across components.

## Sources

- [[references/manifesto-service]] — Manifesto model, setup, and ADR details
- [[projects/manifesto/manifesto]] — Service overview anchored to this orchestration model
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern used for cascading changes
