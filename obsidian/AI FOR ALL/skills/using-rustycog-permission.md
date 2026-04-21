---
title: Using RustyCog Permission
category: skills
tags: [rustycog, permissions, openfga, skills, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/checker.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - openfga/model.fga
summary: Procedure for wiring an OpenFGA-backed PermissionChecker into a service and adding centralized authorization checks to routes.
updated: 2026-04-20
---

# Using RustyCog Permission

Use this guide when integrating [[projects/rustycog/references/rustycog-permission]] into a service.

## Workflow

- Build an `OpenFgaPermissionChecker` from `OpenFgaClientConfig` in your composition root.
- Wrap it in `CachedPermissionChecker` (short TTL, e.g. 15s) and then `MetricsPermissionChecker` before storing the result in `AppState`.
- Pass that single `Arc<dyn PermissionChecker>` into `AppState::new(command_service, user_id_extractor, checker)`.
- On every guarded route call `.with_permission_on(Permission::X, "<openfga_type>")` — the only authz knob.
- Make sure each guarded route uses a UUID path parameter; middleware only binds the deepest UUID into `ResourceRef`.
- Test with `InMemoryPermissionChecker` and explicit `allow(...)` calls — never reach for real OpenFGA in unit tests.

## Common pitfalls

- Naming `object_type` for something that does not exist in [openfga/model.fga](../../../openfga/model.fga). The check fails closed with a logged 4xx from OpenFGA.
- Building a fresh checker per request. The composition root must build it once.
- Assuming an empty `InMemoryPermissionChecker` allows by default — it denies everything until you call `allow`.
- Forgetting to publish the matching domain event so [[projects/sentinel-sync/sentinel-sync]] can write the corresponding tuple. Routes will silently 403 until the tuple arrives.

## Source files

- `rustycog/rustycog-permission/src/lib.rs`
- `rustycog/rustycog-permission/src/checker.rs`
- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`
- `openfga/model.fga`

## Key types

- `PermissionChecker` — async trait `check(subject, action, resource) -> Result<bool, DomainError>`.
- `OpenFgaPermissionChecker` — production implementation.
- `CachedPermissionChecker` — short-TTL LRU decorator (`moka`).
- `MetricsPermissionChecker` — `tracing`-instrumented decorator emitting per-decision events.
- `InMemoryPermissionChecker` — test-only checker.
- `Subject`, `ResourceRef`, `ResourceId` — authorization primitives.

## Sources

- [[projects/rustycog/references/rustycog-permission]]
- [[projects/rustycog/references/rustycog-http]]
- [[entities/permission-checker]]
- [[concepts/openfga-as-authorization-engine]]
- [[concepts/centralized-authorization-service]]
- [[projects/sentinel-sync/sentinel-sync]]
