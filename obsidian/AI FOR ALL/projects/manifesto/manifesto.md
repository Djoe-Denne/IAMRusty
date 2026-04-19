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
  Manifesto is AIForAll's project-service and a practical RustyCog variant, with this page focused on orchestration-specific behavior and the ways the live service diverges from the shared baseline.
provenance:
  extracted: 0.74
  inferred: 0.11
  ambiguous: 0.15
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Manifesto

## Indexes

- [[projects/manifesto/concepts/index]] — concepts
- [[projects/manifesto/skills/index]] — skills
- [[projects/manifesto/references/index]] — references

Manifesto is the project-management service for AIForAll. Use `[[projects/rustycog/references/index]]` for the shared service shell and crate behavior; use this page and the linked Manifesto references for the project-domain rules, service-specific wiring, and guide-versus-runtime drift.

## RustyCog Baseline

- `[[projects/rustycog/references/index]]` is the canonical map for the shared command, config, HTTP, permissions, DB, event, and testing crates that Manifesto composes.
- `[[references/rustycog-service-construction]]` and `[[skills/building-rustycog-services]]` describe the default RustyCog service assembly flow that Manifesto largely follows.
- Read `[[projects/rustycog/references/rustycog-command]]`, `[[projects/rustycog/references/rustycog-config]]`, `[[projects/rustycog/references/rustycog-http]]`, `[[projects/rustycog/references/rustycog-permission]]`, and `[[projects/rustycog/references/rustycog-testing]]` for the baseline behavior that this project specializes.

## Service-Specific Differences

- Manifesto treats projects as assemblies of independently implemented components with their own lifecycle, visibility, and configuration flow.
- Ownership supports both personal and organization-backed projects, which ties the service into the wider `[[concepts/event-driven-microservice-platform]]` and cross-service permission model.
- The composition root is still recognizably RustyCog-shaped, but Manifesto adds project-, component-, and member-scoped permission fetchers plus its own `ManifestoCommandRegistryFactory`.
- Manifesto remains the clearest in-repo example of how RustyCog gets used in practice, but it is no longer a perfect baseline because the live runtime still diverges from some guide-era recommendations around logging, retry wiring, and component-service timeouts. ^[ambiguous]
- The pages under `[[projects/manifesto/references/index]]` should be read as delta documentation: they describe what Manifesto changes, adds, or leaves unresolved on top of the shared RustyCog shell.

## Related

- [[projects/rustycog/references/index]] - Canonical shared framework map that the service pages below build on.
- [[references/rustycog-service-construction]] - Generic RustyCog construction flow that Manifesto specializes.
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

- Which parts of the older Manifesto-authored RustyCog guides should still be treated as normative for new services when the live runtime now differs in a few important places? ^[ambiguous]
- Should Manifesto eventually consume the guide-advertised logging, retry, and timeout knobs end to end, or stay documented as a deliberate RustyCog variant? ^[ambiguous]

## Sources

- [[projects/manifesto/references/manifesto-service]] — Product model, runtime wiring, and project-service ADR summary
- [[projects/rustycog/references/index]] — Shared crate-level baseline for the runtime this service specializes
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog build and wiring guides, including current drift
- [[skills/building-rustycog-services]] — Practical workflow distilled from those guides
