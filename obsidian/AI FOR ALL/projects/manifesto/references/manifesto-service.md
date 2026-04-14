---
title: >-
  Manifesto Service and Project ADR
category: references
tags: [reference, projects, components, visibility/internal]
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
summary: >-
  Code-backed view of Manifesto's runtime shape, route surface, permission model, and the gaps between its MVP implementation and broader project-service ADRs.
provenance:
  extracted: 0.81
  inferred: 0.06
  ambiguous: 0.13
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T20:08:52.0803248Z
---

# Manifesto Service and Project ADR

These sources jointly describe the current `[[projects/manifesto/manifesto]]` service and the broader project-service architecture it is expected to support, while also exposing where the live MVP and the ADR still diverge.

## Key Ideas

- Manifesto owns project records, component attachments, and member access with explicit lifecycle/state models.
- `src/main.rs` loads `ManifestoConfig`, initializes tracing directly, builds `Application`, and hands the server off to the `RouteBuilder`-based HTTP shell.
- `setup/src/app.rs` wires `DbConnectionPool`, an optional multi-queue event publisher, repositories, permission fetchers, use cases, `ManifestoCommandRegistryFactory`, `GenericCommandService`, and `AppState` in one composition root.
- `http/src/lib.rs` exposes project, component, member, and permission-management routes with resource-scoped permission fetchers rather than ad hoc authorization checks.
- `tests/common.rs` shows a RustyCog-style integration-test harness with migrations, a real server, DB-backed fixtures, and `has_sqs() == false` for the default Manifesto test descriptor.
- `IMPLEMENTATION_STATUS.md` frames the service as a mostly complete MVP with eight migrations and broad CRUD coverage, while `docs/project/Archi.md` expands the scope toward component registries, impersonation, and cascading workflows that are not fully demonstrated in the current code. Conflict to resolve. ^[ambiguous]
- The docs and config still advertise some runtime knobs that are not fully wired end to end, including `[command.retry]`, `logging.level`, and `service.component_service.timeout_seconds`. Conflict to resolve. ^[ambiguous]

## Open Questions

- The docs do not give a crisp implementation-status boundary for every ADR decision. Conflict to resolve. ^[ambiguous]
- Event infrastructure, component integration, and component-detail expansion are described as partly ready and partly placeholder depending on which source you read. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/manifesto/manifesto]] — Service overview page
- [[projects/manifesto/concepts/component-based-project-orchestration]] — Main architectural concept extracted here
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog guidance checked against the live service
- [[concepts/event-driven-microservice-platform]] — Async coordination pattern tied to cascading changes
