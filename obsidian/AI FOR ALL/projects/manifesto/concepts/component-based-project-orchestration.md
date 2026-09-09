---
title: >-
  Component-Based Project Orchestration
category: concepts
tags: [projects, components, orchestration, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/README.md
  - Manifesto/IMPLEMENTATION_STATUS.md
  - Manifesto/setup/src/app.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/infra/src/adapters/component_service_client.rs
summary: "Orchestration actuelle de ProjectComponent, distincte de la future plateforme Apparatus documentée avec bindings, Factory et contrôleur."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-09-09T17:50:00Z
---

# Component-Based Project Orchestration

The project-service architecture described for `[[projects/manifesto/manifesto]]` treats a project as an orchestrator over independently implemented component services rather than a monolith that owns every feature end to end.

## Key Ideas

- Projects move through explicit lifecycle states, while attached components have their own status progression from pending to active.
- Component services are expected to expose a shared contract for manifest, configuration, validation, activation, and health.
- The live MVP already demonstrates orchestration through `ComponentServiceClient`, component validation, permission-aware component attachment, and status transitions, even before a broader component platform exists.
- Configuration ownership stays with the component service, while Manifesto tracks overall project and component state.
- `README.md` and `IMPLEMENTATION_STATUS.md` still describe a broader registry, impersonation, and cascading model, but the repo no longer contains the referenced `docs/project/Archi.md` file that would have defined that architecture in detail. Conflict to resolve. ^[ambiguous]
- The current code and implementation-status docs show a project/component/member MVP with real CRUD, permission checks, and migrations, but they do not demonstrate Redis-backed discovery or signed impersonation tokens in the live runtime. Conflict to resolve. ^[ambiguous]
- In practice, Manifesto already acts as the orchestration shell for project records and component attachments, while the broader component ecosystem remains partly blueprint-level. ^[inferred]

## Apparatus — évolution proposée

Le document utilisateur du 9 septembre 2026 précise une cible, pas une description de fonctionnalités livrées. [[projects/manifesto/references/apparatus-source-reconciliation]] confirme le socle d’attachement/ACL/statuts et l’absence de Factory, registry versionné, contrôleur de workloads et host UI.

[[projects/manifesto/concepts/apparatus-platform]] remplace les anciennes pistes vagues de registry/impersonation par une proposition explicite : catalogue métier, release immuable, binding et runtime séparés, gateway de capacités sans bearer IAM exposé. Le lifecycle proposé possède ses propres générations/opérations et conserve l’ancre `ProjectComponent` en V1. ^[inferred]

## Open Questions

- Which pieces of the registry and impersonation design are already implemented versus still planned? Conflict to resolve. ^[ambiguous]
- The docs outline how component autonomy should work, but not yet the operational policy for failure handling across components.

## Sources

- [[projects/manifesto/references/manifesto-service]] — Manifesto model, setup, and ADR details
- [[projects/manifesto/manifesto]] — Service overview anchored to this orchestration model
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern used for cascading changes
