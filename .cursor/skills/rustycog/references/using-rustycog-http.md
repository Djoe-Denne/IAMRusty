# Using RustyCog HTTP

Use this guide when wiring `rustycog-http` (Axum-based HTTP layer with `RouteBuilder`).

## Workflow

- Build `UserIdExtractor` from `[auth.jwt]` (`hs256_secret`, optional `issuer` / `audience`) then `AppState::new(command_service, user_id_extractor, permission_checker)`. The checker is the OpenFGA-backed `Arc<dyn PermissionChecker>` from `using-rustycog-permission.md`.
- The extractor is **HS256-only**. It does not fetch `/iam/.well-known/jwks.json`. Non-empty `issuer` / `audience` become required claims (`iss=iamrusty`, `aud=aiforall` on this platform). Clients send `Authorization: Bearer`. See `docs/platform/authn-jwt.md` and `docs/guides/jwt-consommateur.md`.
- Compose routes through `RouteBuilder` and pick the auth mode per route chain (`.authenticated()` or `.might_be_authenticated()`).
- For every protected route call `.with_permission_on(Permission::X, "<openfga_type>")` immediately after the auth-mode call. There is no `permissions_dir`, no `resource(...)`, and no `with_permission_fetcher(...)`.
- Keep `health_check` and the standard tracing/correlation middleware in the builder path.
- Call `build(server_config)` once after all routes are registered.

## Common Pitfalls

- Putting `with_permission_on` before the route's auth mode — the optional/required mode must be set first so the middleware knows whether to reject anonymous callers.
- Using a non-UUID path parameter for the resource id — the middleware only binds the deepest UUID-shaped segment into `ResourceRef`.
- Naming an `object_type` that is not defined in `openfga/model.fga` — every check returns 403 with an upstream error logged.
- Trying to wire a per-route checker. The single composition-root checker on `AppState` is shared across every request.
- Expecting RS256 / JWKS verification in a consumer service. Mint HS256 with the shared HMAC until rustycog-http grows a JWKS path. IAM's composition root refuses an RS256 issuer for this reason.
- Omitting `iss`/`aud` on test tokens while `[auth.jwt]` sets them — `create_jwt_token` already emits `iamrusty` / `aiforall`.

## Source files

- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-http/src/lib.rs`
- `rustycog/rustycog-http/src/jwt_handler.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`

## Key types

- `RouteBuilder` — fluent route composition with auth/permission/middleware
- `AppState` — shared state holding command service, user-id extractor, and permission checker
- `UserIdExtractor` — HS256 bearer verifier (`sub` UUID, `exp`, `iat`, `jti`, optional `iss`/`aud`)
- `authenticated()` / `might_be_authenticated()` — explicit auth-mode selectors
- `with_permission_on(Permission, object_type)` — route permission guard backed by the `AppState` checker
