# Using RustyCog Permission

Use this guide when integrating `rustycog-permission` (OpenFGA-backed authorization).

## Workflow

- Build an `OpenFgaPermissionChecker` from `OpenFgaClientConfig` in your service's composition root.
- Wrap it in `CachedPermissionChecker` (short TTL) and then `MetricsPermissionChecker` before handing it to `AppState::new(...)`.
- On every guarded route call `.with_permission_on(Permission::X, "<openfga_type>")` where the object type matches one in `openfga/model.fga`.
- Ensure every protected route uses a UUID path parameter — middleware only binds the deepest UUID path segment into the `ResourceRef`.
- Never build a checker per request. `AppState` already holds the shared `Arc<dyn PermissionChecker>`.

## Common pitfalls

- Configuring `with_permission_on(_, "member")` when your OpenFGA model has no `member` type. Confirm the type exists in [openfga/model.fga](../../../../openfga/model.fga) first.
- Emitting domain events that have no matching translator arm in `sentinel-sync` — the authz graph falls out of sync silently. Keep [[projects/sentinel-sync/references/event-to-tuple-mapping]] up to date.
- Forgetting to set `OPENFGA__STORE_ID` and `OPENFGA__AUTHORIZATION_MODEL_ID` in non-default environments. The checker fails closed with an infrastructure error.
- Treating `InMemoryPermissionChecker` as a fallback in production — it always denies unless you explicitly call `allow`.

## Source files

- `rustycog/rustycog-permission/src/lib.rs`
- `rustycog/rustycog-permission/src/checker.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`
- `rustycog/rustycog-http/src/builder.rs`
- `openfga/model.fga`

## Key types

- `PermissionChecker` — async trait `check(subject, action, resource) -> Result<bool, DomainError>`.
- `OpenFgaPermissionChecker` — production implementation.
- `CachedPermissionChecker` — `moka` LRU decorator with time-based invalidation.
- `MetricsPermissionChecker` — instrumented decorator emitting `tracing` events for every decision.
- `InMemoryPermissionChecker` — test-only checker.
- `Subject`, `ResourceRef`, `ResourceId` — authorization primitives.
