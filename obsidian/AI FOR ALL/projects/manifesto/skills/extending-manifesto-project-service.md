---
title: >-
  Extending Manifesto Project Service
category: skills
tags: [projects, services, permissions, visibility/internal]
sources:
  - Manifesto/setup/src/app.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/resources/permissions/project.conf
  - Manifesto/resources/permissions/component.conf
  - Manifesto/resources/permissions/member.conf
  - Manifesto/tests/common.rs
summary: >-
  Workflow for adding a new Manifesto capability by threading commands, routes, permissions, events, and tests through the existing project-service shape.
provenance:
  extracted: 0.80
  inferred: 0.12
  ambiguous: 0.08
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-04-19T11:49:06.1450368Z
---

# Extending Manifesto Project Service

Use this page when adding a new capability to `[[projects/manifesto/manifesto]]`. The practical path is not “just add a handler”: Manifesto usually threads one change through entities, use cases, commands, routes, permission models, events, and tests together.

## Workflow

- Decide the scope first: does the change belong under the existing `project`, `component`, or `member` surfaces, or does it justify a genuinely new resource boundary?
- If the change adds or reshapes persisted state, update the domain entity, repository flow, migration, and DB fixtures together so the write path and the tests stay aligned.
- Add or update the use case first, then expose it through a command and handler pair; keep `command_type()` aligned with the registration key in `ManifestoCommandRegistryFactory`.
- Wire the route in `http/src/lib.rs` through `RouteBuilder`, choosing the right `resource(...)`, auth mode, permission fetcher, and permission requirement before treating the handler as complete.
- If the change affects authorization, decide whether the existing generic resource paths are enough or whether you also need the `{resource_id}`-style specific permission path used for component-instance grants.
- Keep the Casbin model files under `resources/permissions/` in sync with the route surface you expose, especially when adding a new action or changing how many resource IDs the fetcher is expected to interpret.
- Publish a Manifesto domain event from the use case when the change matters outside the local transaction, but remember the current service treats publication as best effort and logs failures instead of aborting the main write.
- Close the loop with API tests through `setup_test_server()`, real JWTs, and DB fixtures; cover both the happy path and at least one permission-denied or invalid-state case.

## Common Checks

- If you add a new config knob, verify that `src/main.rs` or `setup/src/app.rs` actually consumes it rather than assuming the presence of a TOML key makes it live.
- If you add or extend component-facing behavior, remember that the current project-detail response still returns `endpoint` and `access_token` as `None`; do not assume Manifesto already owns runtime handoff to the component service.
- If you add queue-sensitive behavior, the default test harness will not exercise SQS for you; add targeted coverage instead of relying on the standard API suites alone.

## Sources

- [[projects/manifesto/manifesto]] - Service hub and MVP framing.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Route and permission surface you will usually extend.
- [[projects/manifesto/references/manifesto-runtime-and-configuration]] - Startup and config wiring you may need to touch.
- [[projects/manifesto/references/manifesto-event-model]] - Best-effort event publication behavior.
- [[projects/manifesto/references/manifesto-testing-and-fixtures]] - Default harness, fixtures, and coverage patterns.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Project lifecycle behavior often affected by new flows.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus instance-scoped permission model.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - Component validation and external adapter boundary.
- [[skills/building-rustycog-services]] - Broader RustyCog service-building workflow behind the same composition pattern.
