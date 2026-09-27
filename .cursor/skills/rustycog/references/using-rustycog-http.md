# Using RustyCog HTTP

Use this guide when wiring `rustycog-http` (Axum-based HTTP layer with `RouteBuilder`).

## Workflow

- Build `UserIdExtractor` from `[auth.jwt]` (`jwks_url`, `allowed_algorithms=["RS256"]`, `audience`) then `AppState::new(command_service, user_id_extractor, permission_checker)`. The checker is the OpenFGA-backed `Arc<dyn PermissionChecker>` from `using-rustycog-permission.md`.
- The extractor verifies **RS256 via JWKS** (`kid` + JWK `iss`). It also inserts `JwtPrincipal { iss, sub, org }` into request extensions alongside the bare `Uuid` `sub`. Runtime is not HS256-only; mint RS256 for consumers. HS256 is an explicit `allowed_algorithms` flag in test.toml, not the default. See `docs/platform/authn-jwt.md` and `docs/guides/jwt-consommateur.md`.
- Compose routes through `RouteBuilder` and pick the auth mode per route chain (`.authenticated()` or `.might_be_authenticated()`).
- For every protected route call `.with_permission_on(Permission::X, "<openfga_type>")` immediately after the auth-mode call. There is no `permissions_dir`, no `resource(...)`, and no `with_permission_fetcher(...)`.
- Keep `health_check` and the standard tracing/correlation middleware in the builder path.
- Call `build(server_config)` once after all routes are registered.

## Common Pitfalls

- Putting `with_permission_on` before the route's auth mode — the optional/required mode must be set first so the middleware knows whether to reject anonymous callers.
- Using a non-UUID path parameter for the resource id — the middleware only binds the deepest UUID-shaped segment into `ResourceRef`.
- Naming an `object_type` that is not defined in `openfga/model.fga` — every check returns 403 with an upstream error logged.
- Trying to wire a per-route checker. The single composition-root checker on `AppState` is shared across every request.
- Treating the extractor as HS256-only, or refusing to mint RS256 — runtime consumers verify RS256 + JWKS. Minting HS256 for `allowed_algorithms=[RS256]` is rejected; the HS256 IT window exists only when test.toml lists it explicitly.
- Hardcoding `iss=iamrusty` in handlers — read `Extension<JwtPrincipal>.iss` instead.

## Source files

- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-http/src/lib.rs`
- `rustycog/rustycog-http/src/jwt_handler.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`

## Key types

- `RouteBuilder` — fluent route composition with auth/permission/middleware
- `AppState` — shared state holding command service, user-id extractor, and permission checker
- `UserIdExtractor` — RS256 JWKS bearer verifier (+ optional HS256 window); emits `JwtPrincipal`
- `JwtPrincipal` — `(iss, sub, org)` from a verified access token
- `authenticated()` / `might_be_authenticated()` — explicit auth-mode selectors
- `with_permission_on(Permission, object_type)` — route permission guard backed by the `AppState` checker
