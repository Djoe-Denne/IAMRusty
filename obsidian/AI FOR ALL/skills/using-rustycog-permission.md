---
title: Using RustyCog Permission
category: skills
tags: [rustycog, permissions, openfga, skills, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/checker.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - openfga/model.fga
summary: Wire OpenFgaPermissionChecker into a service. HTTP ITs use TestOpenFga. Manifesto org_scope needs RelationshipTuple and read_tuples on the rustycog pin.
provenance:
  extracted: 0.80
  inferred: 0.16
  ambiguous: 0.04
created: 2026-04-22T17:30:00Z
updated: 2026-09-09T16:45:00Z
---

# Using RustyCog Permission

Use this guide when integrating [[projects/rustycog/references/rustycog-permission]] into a service.

## Workflow

- Build an `OpenFgaPermissionChecker` from `OpenFgaClientConfig` in your composition root.
- Read `config.openfga.cache_ttl_seconds` and skip the `CachedPermissionChecker` decoration entirely when it is `Some(0)`; otherwise wrap with the configured TTL (default 15s when `None`). Then wrap with `MetricsPermissionChecker` before storing the result in `AppState`.
- Pass that single `Arc<dyn PermissionChecker>` into `AppState::new(command_service, user_id_extractor, checker)`.
- On every guarded route call `.with_permission_on(Permission::X, "<openfga_type>")` — the only authz knob.
- Make sure each guarded route uses a UUID path parameter; middleware only binds the **deepest** UUID into `ResourceRef`. For routes like `/api/projects/{project_id}/components/{component_id}`, the resource is the component id, not the project id — important when arranging stubs in tests.
- For unit tests, use `InMemoryPermissionChecker` and explicit `allow(...)` calls. For Hive / Telegraph / Manifesto HTTP ITs, use [[projects/rustycog/references/openfga-real-testcontainer-fixture]] (`TestOpenFga`). `OpenFgaMockService` is crate-level only.
- Manifesto org-admin listing uses `OpenFgaPermissionChecker::read_tuples` and `RelationshipTuple`. Those APIs must exist on the pinned rustycog **main** or CI will not compile. See [[projects/aiforall/concepts/rustycog-git-submodule]].

## Test config

HTTP ITs: point OpenFGA at `TestOpenFga` (random port, fresh store). Set `openfga.cache_ttl_seconds = 0` if a cached checker is wired.

Do not point service Check at the WireMock singleton. Collaborator HTTP stubs use [[projects/rustycog/references/isolated-wiremock-fixture]] when they would otherwise collide.

## Common pitfalls

- Naming `object_type` for something that does not exist in [openfga/model.fga](../../../openfga/model.fga). The check fails closed with a logged 4xx from OpenFGA.
- Building a fresh checker per request. The composition root must build it once.
- Assuming an empty `InMemoryPermissionChecker` allows by default — it denies everything until you call `allow`.
- Forgetting to publish the matching domain event so [[projects/sentinel-sync/sentinel-sync]] can write the corresponding tuple. Routes will silently 403 until the tuple arrives.
- Leaving `cache_ttl_seconds` at its `None` default in test configs when a cache layer is enabled.
- Assuming leftover OpenFGA Write authorizes Manifesto mutations without an active DB member — it does not.

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
- [[projects/rustycog/references/openfga-mock-service]]
- [[skills/stubbing-http-with-wiremock]]
- [[entities/permission-checker]]
- [[concepts/openfga-as-authorization-engine]]
- [[concepts/centralized-authorization-service]]
- [[projects/sentinel-sync/sentinel-sync]]
- [[projects/manifesto/references/manifesto-testing-and-fixtures]]
