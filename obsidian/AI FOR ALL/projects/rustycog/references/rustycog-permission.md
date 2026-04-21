---
title: RustyCog Permission
category: references
tags: [reference, rustycog, permissions, openfga, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/checker.rs
  - openfga/model.fga
summary: rustycog-permission defines the PermissionChecker trait plus an OpenFGA-backed client, an in-memory test checker, and a short-TTL LRU cache decorator. All Casbin code has been removed.
provenance:
  extracted: 0.92
  inferred: 0.05
  ambiguous: 0.03
created: 2026-04-15
updated: 2026-04-20
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

- `OpenFgaPermissionChecker` — production. Built from `OpenFgaClientConfig` (`api_url`, `store_id`, optional `authorization_model_id`, optional `api_token`). POSTs to `/stores/{id}/check`.
- `InMemoryPermissionChecker` — deterministic, test-only. `allow` / `deny` mutate an internal set of tuples.
- `CachedPermissionChecker` — decorates any inner `Arc<dyn PermissionChecker>` with a `moka` LRU cache keyed by `(user_id, permission, object_type, object_id)`. Time-based invalidation only.

## Wiring

The checker is constructed once in each service's composition root and injected into `AppState` so HTTP middleware (`with_permission_on`) can share it.

```rust
let config = OpenFgaClientConfig {
    api_url: cfg.openfga.api_url.clone(),
    store_id: cfg.openfga.store_id.clone(),
    authorization_model_id: cfg.openfga.authorization_model_id.clone(),
    api_token: cfg.openfga.api_token.clone(),
};
let raw = Arc::new(OpenFgaPermissionChecker::new(config)?);
let checker: Arc<dyn PermissionChecker> =
    Arc::new(CachedPermissionChecker::new(raw, Duration::from_secs(15), 10_000));
```

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

## Removed

- `PermissionEngine`, `CasbinPermissionEngine`, `PermissionsFetcher` — replaced by `PermissionChecker` + OpenFGA. See [[entities/permissions-fetcher]] (marked removed).
