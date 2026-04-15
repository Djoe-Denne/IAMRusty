---
title: >-
  Manifesto
category: project
tags: [projects, orchestration, blueprint, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - docs/project/Archi.md
  - Manifesto/IMPLEMENTATION_STATUS.md
  - Manifesto/src/main.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/configuration/src/lib.rs
  - Manifesto/tests/common.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  Manifesto is AIForAll's project-service MVP and clearest in-repo RustyCog blueprint, though its ADRs still reach beyond the current implementation.
provenance:
  extracted: 0.68
  inferred: 0.13
  ambiguous: 0.19
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T20:28:20.9129598Z
---

# Manifesto

## Indexes

- [[projects/manifesto/concepts/index]] — concepts
- [[projects/manifesto/references/index]] — references

Manifesto is the project-management service for AIForAll. It owns project records, component attachments, member access, and the orchestration model described in `[[projects/manifesto/concepts/component-based-project-orchestration]]`. It also doubles as the strongest practical reference for `[[skills/building-rustycog-services]]`, even when its current runtime diverges from some guide-era RustyCog recommendations. ^[ambiguous]

## Key Ideas

- Projects are treated as assemblies of independently implemented components with their own lifecycle and configuration flow.
- The current runtime path is concrete and code-backed: `src/main.rs` loads `ManifestoConfig`, builds `Application` in `setup/src/app.rs`, wires `ManifestoCommandRegistryFactory`, creates `AppState`, and starts the HTTP surface through `RouteBuilder`.
- The service follows the same layered style as `[[projects/iamrusty/concepts/hexagonal-architecture]]`, and its docs make the setup crate, command factory, route builder, and permission fetchers concrete rather than abstract.
- Manifesto is where `[[concepts/shared-rust-microservice-sdk]]` becomes operational guidance for new services, not just a library catalog.
- Ownership supports both personal and organization-backed projects, which ties Manifesto into the wider `[[concepts/event-driven-microservice-platform]]` and cross-service permission model.
- The code and `IMPLEMENTATION_STATUS.md` describe a production-ready MVP for project, component, and member management, while `docs/project/Archi.md` still reaches toward a broader component-registry and impersonation architecture. Conflict to resolve. ^[ambiguous]
- Manifesto's own RustyCog guide set is intentionally preserved as a two-story record: it recommends explicit logging/config/retry patterns, while the current service still initializes tracing directly, hardcodes the component-service timeout in setup, and leaves `[command.retry]` unwired in `ManifestoConfig`. Conflict to resolve. ^[ambiguous]

## Related

- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Ownership bootstrap, defaults, and publish/archive transitions.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission model.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - External component catalog integration and mock fallback.
- [[projects/manifesto/references/manifesto-entity-model]] - Project, component, membership, and project-scoped RBAC entities.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Live route and permission behavior.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted by project, component, and member flows.

## Open Questions

- Some ADR elements read like target architecture while the README emphasizes an MVP that is already mostly complete. ^[ambiguous]
- Which `docs/project/Archi.md` behaviors are already live versus still roadmap material, especially around registry, impersonation, and cascading workflows? ^[ambiguous]
- Should Manifesto align its runtime with the guide-prescribed RustyCog logging and retry patterns, or explicitly document itself as a deliberate variant? ^[ambiguous]

## Sources

- [[projects/manifesto/references/manifesto-service]] — Product model, runtime wiring, and project-service ADR summary
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog build and wiring guides, including current drift
- [[skills/building-rustycog-services]] — Practical workflow distilled from those guides
