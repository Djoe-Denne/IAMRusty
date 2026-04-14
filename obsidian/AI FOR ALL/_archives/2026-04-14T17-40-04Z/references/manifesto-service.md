---
title: >-
  Manifesto Service and Project ADR
category: references
tags: [reference, projects, components, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - docs/project/Archi.md
summary: >-
  Combined source summary for Manifesto's project model, setup workflow, and the wider project-service architecture record.
provenance:
  extracted: 0.88
  inferred: 0.07
  ambiguous: 0.05
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# Manifesto Service and Project ADR

These sources jointly describe the current `[[projects/manifesto/manifesto]]` service and the broader project-service architecture it is expected to support.

## Key Ideas

- Manifesto owns project records, component attachments, and member access with explicit lifecycle/state models.
- Setup guidance focuses on database creation, migrations, layered configuration, and basic development workflow.
- The architecture record expands the scope toward component registries, JWT-backed impersonation, and organization-change cascading.
- The sources together describe both a concrete MVP and a larger orchestration target around `[[concepts/component-based-project-orchestration]]`.

## Open Questions

- The docs do not give a crisp implementation-status boundary for every ADR decision.
- Operational details for external component services are architectural rather than implementation-specific in this pass.

## Sources

- [[projects/manifesto/manifesto]] — Service overview page
- [[concepts/component-based-project-orchestration]] — Main architectural concept extracted here
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern tied to cascading changes
