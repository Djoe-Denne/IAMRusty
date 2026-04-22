---
title: RustyCog Permission
category: references
tags: [reference, rustycog, permissions, openfga, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/checker.rs
  - openfga/model.fga
summary: rustycog-permission defines the PermissionChecker trait plus an OpenFGA-backed client, an in-memory test checker, a short-TTL LRU cache decorator (now opt-out via cache_ttl_seconds), and a metrics decorator. All Casbin code has been removed.
provenance:
  extracted: 0.92
  inferred: 0.05
  ambiguous: 0.03
created: 2026-04-15
updated: 2026-04-22T17:30:00Z
---

# RustyCog Permission

`rustycog-permission` is the authorization client used by every RustyCog service. It exposes permission primitives and a checker trait; the production implementation issues `Check` calls against the centralized [[concepts/openfga-as-authorization-engine]] deployment.

## Surface

- `Permission` — `Read`, `Write`, `Admin`, `Owner`. Maps to OpenFGA relations (`read`, `write`, `administer`, `own`) via `Permission::relation()`.
- `Subject` — authenticated caller, wrapping the user UUID. Rendered as `user:{uuid}` on the wire.
- `ResourceRef` — `{ object_type, object_id }`. `object_type` must match a type in [openfga/model.fga](../../../../openfga/model.fga).
- `ResourceId` — the legacy UUID-only resource wrapper, kept for middleware path extraction.
- `PermissionChecker` — async trait `check(subject, action, resource) -> Result<bool, DomainError>`.

## Implementations

- `OpenFgaPermissionChecker` — production. Built from `OpenFgaClientConfig` (`api_url`, `store_id`, optional `authorization_model_id`, optional `api_token`, optional `cache_ttl_seconds`). POSTs to `/stores/{id}/check`.
- `InMemoryPermissionChecker` — deterministic, test-only. `allow` / `deny` mutate an internal set of tuples.
- `CachedPermissionChecker` — decorates any inner `Arc<dyn PermissionChecker>` with a `moka` LRU cache keyed by `(user_id, permission, object_type, object_id)`. Time-based invalidation only.
- `MetricsPermissionChecker` — `tracing`-instrumented decorator emitting structured `permission decision` events for every check, including `decision="allow"|"deny"` and `elapsed_us`.

## Wiring

The checker is constructed once in each service's composition root and injected into `AppState` so HTTP middleware (`with_permission_on`) can share it.

```rust
let raw = Arc::new(OpenFgaPermissionChecker::new(cfg.openfga.clone())?);
let cache_ttl = cfg.openfga.cache_ttl_seconds.unwrap_or(15);
let inner: Arc<dyn PermissionChecker> = if cache_ttl == 0 {
    raw
} else {
    Arc::new(CachedPermissionChecker::new(raw, Duration::from_secs(cache_ttl), 10_000))
};
let checker: Arc<dyn PermissionChecker> = Arc::new(MetricsPermissionChecker::new(inner));
```

`cache_ttl_seconds` (added 2026-04-22) makes the cache decoration opt-out:

- `None` (default) — production behavior, 15s TTL.
- `Some(n)` for `n > 0` — explicit TTL override.
- `Some(0)` — skip the cache entirely. **Required** in test configs that use [[projects/rustycog/references/openfga-mock-service]] and need to flip a `Check` decision mid-test (e.g. grant ➜ revoke ➜ deny scenarios). Without it, `CachedPermissionChecker` would serve a stale allow from the first request and the second decision would never reach the wiremock fake.

## Linked Entities

- [[entities/permission-checker]]
- [[entities/subject]]
- [[entities/resource-ref]]
- [[entities/resource-id]]

## Related

- [[concepts/openfga-as-authorization-engine]]
- [[concepts/zanzibar-relation-tuples]]
- [[projects/sentinel-sync/sentinel-sync]]
- [[projects/rustycog/references/rustycog-http]]
- [[projects/rustycog/references/openfga-mock-service]] — wiremock-backed `Check` fake for service tests.

## Removed

- `PermissionEngine`, `CasbinPermissionEngine`, `PermissionsFetcher` — replaced by `PermissionChecker` + OpenFGA. See [[entities/permissions-fetcher]] (marked removed).
