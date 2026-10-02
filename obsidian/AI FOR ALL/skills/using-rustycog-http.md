---
title: Using RustyCog HTTP
category: skills
tags: [rustycog, http, skills, visibility/internal]
sources:
  - rustycog/rustycog-http/src/builder.rs
  - rustycog/rustycog-http/src/lib.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: >-
  RouteBuilder, auth modes, OpenFGA guard. Overlay mesh : trusted_gateway_san
  non vide = principal passerelle, pas de JWT in-process.
updated: 2026-10-01T16:45:00Z
provenance:
  extracted: 0.80
  inferred: 0.18
  ambiguous: 0.02
---

# Using RustyCog HTTP

Use this guide when wiring [[projects/rustycog/references/rustycog-http]].

## Workflow

- Build `AppState::new(command_service, user_id_extractor, permission_checker)`. The checker is the OpenFGA-backed `Arc<dyn PermissionChecker>` from [[skills/using-rustycog-permission]].
- Compose routes through `RouteBuilder` and pick the auth mode per route chain (`.authenticated()` or `.might_be_authenticated()`).
- For every protected route call `.with_permission_on(Permission::X, "<openfga_type>")` immediately after the auth-mode call. There is no `permissions_dir`, no `resource(...)`, no `with_permission_fetcher(...)`.
- Keep `health_check` and the standard tracing/correlation middleware in the builder path.
- Call `build(server_config)` once after every route is registered.

## Common pitfalls

- Putting `with_permission_on` before the route's auth mode — the optional/required mode must be set first so the middleware knows whether to reject anonymous callers.
- Using a non-UUID path parameter for the resource id — the middleware only binds the deepest UUID-shaped segment into `ResourceRef`.
- Naming an `object_type` that is not defined in [openfga/model.fga](../../../openfga/model.fga) — every check returns 403 with an upstream error logged.
- Trying to wire a per-route checker. The single composition-root checker on `AppState` is shared across every request.
- In mesh mode, a non-empty `auth.mesh.trusted_gateway_san` makes rustycog trust `x-principal-iss`/`sub` from the gateway SAN only and skip in-process JWT. Empty SAN keeps [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]. See [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

## Source files

- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`
- `rustycog/rustycog-http/src/lib.rs`

## Sources

- [[projects/rustycog/references/rustycog-http]]
- [[entities/route-builder]]
- [[entities/permission-checker]]
- [[concepts/centralized-authorization-service]]
