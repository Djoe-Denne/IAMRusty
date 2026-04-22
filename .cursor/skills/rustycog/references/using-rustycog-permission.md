# Using RustyCog Permission

Use this guide when integrating `rustycog-permission` (OpenFGA-backed authorization).

## Workflow

- Build an `OpenFgaPermissionChecker` from `OpenFgaClientConfig` in your service's composition root.
- Read `config.openfga.cache_ttl_seconds` and **skip the `CachedPermissionChecker` decoration entirely when it is `Some(0)`**; otherwise wrap with the configured TTL (default 15s when `None`). Then wrap with `MetricsPermissionChecker` before handing the `Arc<dyn PermissionChecker>` to `AppState::new(...)`.
- On every guarded route call `.with_permission_on(Permission::X, "<openfga_type>")` where the object type matches one in `openfga/model.fga`.
- Ensure every protected route uses a UUID path parameter — middleware only binds the **deepest** UUID path segment into the `ResourceRef`. For routes like `/api/projects/{project_id}/components/{component_id}`, the resource id is the component id, not the project id.
- Never build a checker per request. `AppState` already holds the shared `Arc<dyn PermissionChecker>`.

## Test wiring

Use `rustycog_testing::permission::OpenFgaFixtures::service().await` (the in-crate wiremock-backed fake of OpenFGA's `Check`) for service integration tests. Configure the test config to:

- `openfga.api_url = "http://127.0.0.1:3000"` (the singleton wiremock listener)
- `openfga.store_id` matches the fixture default (`01h0test0store0fixture000openfga`) or call `service_with_store_id(...)`
- `openfga.cache_ttl_seconds = 0` so `Check` is re-issued every middleware invocation (otherwise grant ➜ revoke ➜ deny shapes don't work in-process)

Per-test arrangement patterns:

- Happy path → harness mounts `mock_check_any(true)` once in `setup_test_server`; tests need no per-tuple work.
- Denial test → `openfga.reset().await` then `openfga.mock_check_deny(subject, action, resource).await` (reset is mandatory because wiremock is first-match-wins).
- Phase-flip → reset between phases and mount fresh stubs each time.

For unit tests that don't boot the service, use `InMemoryPermissionChecker` and explicit `allow(...)` calls.

## Common pitfalls

- Configuring `with_permission_on(_, "member")` when your OpenFGA model has no `member` type. Confirm the type exists in [openfga/model.fga](../../../../openfga/model.fga) first.
- Emitting domain events that have no matching translator arm in `sentinel-sync` — the authz graph falls out of sync silently.
- Forgetting to set `OPENFGA__STORE_ID` and `OPENFGA__AUTHORIZATION_MODEL_ID` in non-default environments. The checker fails closed with an infrastructure error.
- Treating `InMemoryPermissionChecker` as a fallback in production — it always denies unless you explicitly call `allow`.
- Leaving `cache_ttl_seconds` unset in test configs and then debugging why `mock_check_deny` arranged after a previous `mock_check_allow` for the same tuple has no effect — the cache served the stale allow.
- Mounting per-tuple stubs on top of `mock_check_any(true)` without calling `openfga.reset().await` first. wiremock matches in registration order; the catch-all wins.

## Source files

- `rustycog/rustycog-permission/src/lib.rs`
- `rustycog/rustycog-permission/src/checker.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`
- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-testing/src/permission/service.rs` — `OpenFgaMockService`
- `openfga/model.fga`
- `Manifesto/setup/src/app.rs`, `Manifesto/config/test.toml`, `Manifesto/tests/common.rs` — canonical wiring of the cache TTL knob plus the wiremock fake.

## Key types

- `PermissionChecker` — async trait `check(subject, action, resource) -> Result<bool, DomainError>`.
- `OpenFgaPermissionChecker` — production implementation.
- `OpenFgaClientConfig` — config record. Includes `cache_ttl_seconds: Option<u64>` (default `None` = 15s in production; set `Some(0)` in tests to disable caching).
- `CachedPermissionChecker` — `moka` LRU decorator with time-based invalidation. Skip its decoration entirely in the composition root when `cache_ttl_seconds == Some(0)`.
- `MetricsPermissionChecker` — instrumented decorator emitting `tracing` events for every decision.
- `InMemoryPermissionChecker` — test-only checker.
- `OpenFgaMockService` (in `rustycog-testing`) — wiremock-backed `Check` fake with `mock_check_allow` / `mock_check_deny` / `mock_check_any` / `reset()` / `client_config()`.
- `Subject`, `ResourceRef`, `ResourceId` — authorization primitives.
