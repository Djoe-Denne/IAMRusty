---
title: >-
  Manifesto
category: project
tags: [projects, orchestration, blueprint, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/SETUP.md
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
  Manifesto is AIForAll's project-service MVP and clearest in-repo RustyCog blueprint, with current behavior grounded in live code and older architecture notes treated cautiously.
provenance:
  extracted: 0.74
  inferred: 0.11
  ambiguous: 0.15
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-19T11:49:06.1450368Z
---

# Manifesto

## Indexes

- [[projects/manifesto/concepts/index]] — concepts
- [[projects/manifesto/skills/index]] — skills
- [[projects/manifesto/references/index]] — references

Manifesto is the project-management service for AIForAll. It owns project records, component attachments, member access, and the orchestration model described in `[[projects/manifesto/concepts/component-based-project-orchestration]]`. It also doubles as the strongest practical reference for `[[skills/building-rustycog-services]]`, even when its current runtime diverges from some guide-era RustyCog recommendations. ^[ambiguous]

## Key Ideas

- Projects are treated as assemblies of independently implemented components with their own lifecycle and configuration flow.
- The current runtime path is concrete and code-backed: `src/main.rs` loads `ManifestoConfig`, builds `Application` in `setup/src/app.rs`, wires `ManifestoCommandRegistryFactory`, creates `AppState`, and starts the HTTP surface through `RouteBuilder`.
- Manifesto is where `[[concepts/shared-rust-microservice-sdk]]` becomes operational guidance for new services, not just a library catalog.
- Ownership supports both personal and organization-backed projects, which ties Manifesto into the wider `[[concepts/event-driven-microservice-platform]]` and cross-service permission model.
- `README.md` and `IMPLEMENTATION_STATUS.md` still point readers at `docs/project/Archi.md`, but that file is not present in the checked-in repo, so the broader registry, impersonation, and cascading story can only be treated as historical architecture intent for now. Conflict to resolve. ^[ambiguous]
- Manifesto's own RustyCog guide set is intentionally preserved as a two-story record: it recommends explicit logging/config/retry patterns, while the current service still initializes tracing directly, hardcodes the component-service timeout in setup, and leaves `[command.retry]` unwired in `ManifestoConfig`. Conflict to resolve. ^[ambiguous]

## Related

- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Ownership bootstrap, defaults, and publish/archive transitions.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission model.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - External component catalog integration and mock fallback.
- [[projects/manifesto/references/manifesto-entity-model]] - Project, component, membership, and project-scoped RBAC entities.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Live route and permission behavior.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted by project, component, and member flows.
- [[projects/manifesto/references/manifesto-runtime-and-configuration]] - `MANIFESTO_*` config loading, queue publisher wiring, and live runtime drift.
- [[projects/manifesto/references/manifesto-testing-and-fixtures]] - Real-server test harness, DB fixtures, and the default no-SQS test posture.
- [[projects/manifesto/skills/extending-manifesto-project-service]] - Practical workflow for adding commands, routes, permissions, events, and tests.

## Open Questions

- Some ADR elements read like target architecture while the README emphasizes an MVP that is already mostly complete. ^[ambiguous]
- How much of the missing `docs/project/Archi.md` story is still roadmap material versus live system contract? ^[ambiguous]
- Should Manifesto align its runtime with the guide-prescribed RustyCog logging and retry patterns, or explicitly document itself as a deliberate variant? ^[ambiguous]

## Sources

- [[projects/manifesto/references/manifesto-service]] — Product model, runtime wiring, and project-service ADR summary
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog build and wiring guides, including current drift
- [[skills/building-rustycog-services]] — Practical workflow distilled from those guides
