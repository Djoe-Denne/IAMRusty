---
title: >-
  Manifesto
category: project
tags: [projects, orchestration, blueprint, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - docs/project/Archi.md
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  Manifesto manages projects and components while also serving as the clearest blueprint for building RustyCog-based services.
provenance:
  extracted: 0.75
  inferred: 0.17
  ambiguous: 0.08
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:03:47.5107188Z
---

# Manifesto

Manifesto is the project-management service for AIForAll. It owns project records, component attachments, member access, and the orchestration model described in `[[concepts/component-based-project-orchestration]]`. It also doubles as the strongest practical reference for `[[skills/building-rustycog-services]]`.

## Key Ideas

- Projects are treated as assemblies of independently implemented components with their own lifecycle and configuration flow.
- The service follows the same layered style as `[[concepts/hexagonal-architecture]]`, and its docs make the setup crate, command factory, route builder, and permission fetchers concrete rather than abstract.
- Manifesto is where `[[concepts/shared-rust-microservice-sdk]]` becomes operational guidance for new services, not just a library catalog.
- Ownership supports both personal and organization-backed projects, which ties Manifesto into the wider `[[concepts/event-driven-microservice-platform]]` and cross-service permission model.
- The project docs explicitly call out a few gaps between declared configuration and active runtime wiring, which is useful operationally but means some patterns are blueprint-first rather than fully enforced everywhere. ^[ambiguous]

## Open Questions

- Some ADR elements read like target architecture while the README emphasizes an MVP that is already mostly complete. ^[ambiguous]
- The exact boundary between current Manifesto implementation and the future component-service ecosystem still needs deeper source coverage.

## Sources

- [[references/manifesto-service]] — Product model, setup flow, and project-service ADR summary
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog build and wiring guides
- [[skills/building-rustycog-services]] — Practical workflow distilled from those guides
