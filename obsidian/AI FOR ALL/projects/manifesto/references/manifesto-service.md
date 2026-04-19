---
title: >-
  Manifesto Service and Project ADR
category: references
tags: [reference, projects, components, visibility/internal]
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
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  Manifesto-specific runtime notes that sit on top of the shared RustyCog service shell, highlighting project-domain wiring and the main guide-versus-runtime differences.
provenance:
  extracted: 0.84
  inferred: 0.06
  ambiguous: 0.10
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Manifesto Service and Project ADR

This page is the Manifesto-specific companion to `[[projects/rustycog/references/index]]` and `[[references/rustycog-service-construction]]`. It assumes the shared RustyCog service shell is already understood and keeps the details that are unique to `[[projects/manifesto/manifesto]]`.

## RustyCog Baseline

- `[[projects/rustycog/references/index]]` is the canonical map for the shared crates and runtime conventions this service composes.
- `[[references/rustycog-service-construction]]` and `[[skills/building-rustycog-services]]` cover the generic order of operations: typed config, composition root, command registry, `AppState`, `RouteBuilder`, and integration tests.
- `[[projects/rustycog/references/rustycog-command]]`, `[[projects/rustycog/references/rustycog-config]]`, `[[projects/rustycog/references/rustycog-http]]`, and `[[projects/rustycog/references/rustycog-testing]]` explain the shared behavior that this page does not repeat.

## Service-Specific Differences

- Manifesto owns project records, component attachments, and member access with explicit lifecycle and state models that do not exist in the framework itself.
- `src/main.rs` still follows the standard RustyCog boot path, but the live service uses `ManifestoConfig`, `ManifestoCommandRegistryFactory`, and a composition root specialized around project orchestration.
- `setup/src/app.rs` adds an optional multi-queue event publisher, a component-service client, and resource-scoped permission fetchers for `project`, `component`, and `member` resources before handing everything to `GenericCommandService`.
- `http/src/lib.rs` exposes project, component, member, and permission-management routes, so the service's HTTP surface is shaped by Manifesto's domain more than by generic RustyCog routing concerns.
- `tests/common.rs` keeps the shared real-server harness but fixes the default Manifesto posture at `has_sqs() == false`, making DB-backed integration tests the primary confidence path.
- Older setup notes and ADR-style material still matter as historical context, but the stronger runtime truth now lives in `setup/src/app.rs`, `configuration/src/lib.rs`, `http/src/lib.rs`, and the service-specific reference pages linked from `[[projects/manifesto/references/index]]`. ^[inferred]
- Some guide-advertised knobs remain only partly wired in the live runtime, especially `[command.retry]`, `logging.level`, and `service.component_service.timeout_seconds`. ^[ambiguous]

## Open Questions

- The docs still do not give a crisp implementation-status boundary for every ADR decision, so readers need the per-topic delta pages to separate current behavior from historical intent. ^[ambiguous]
- Event infrastructure, component integration, and component-detail expansion are still described as partly ready and partly placeholder depending on which source you read. ^[ambiguous]

## Sources

- [[projects/manifesto/manifesto]] — Service overview page
- [[projects/rustycog/references/index]] — Shared framework baseline for the runtime being specialized here
- [[projects/manifesto/concepts/component-based-project-orchestration]] — Main architectural concept extracted here
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog guidance checked against the live service
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern tied to cascading changes
