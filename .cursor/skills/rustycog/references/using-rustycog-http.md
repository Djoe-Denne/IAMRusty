# Using RustyCog HTTP

Use this guide when wiring `rustycog-http` (Axum-based HTTP layer with `RouteBuilder`).

## Workflow

- Build `AppState` with your command service and user-id extractor.
- Compose routes through `RouteBuilder` and choose auth mode per route chain.
- Configure permission-protected routes in this order: `permissions_dir` → `resource` → `with_permission_fetcher` → `with_permission`.
- Keep health endpoint and tracing/correlation middleware in the standard builder path.
- Call `build(server_config)` once after all routes are registered.

## Common Pitfalls

- Applying `with_permission` before setting resource and fetcher context.
- Using optional-auth mode while expecting fully public behavior from permission middleware.
- Letting permission model path mistakes panic at startup instead of validating early.

## Source files

- `rustycog/rustycog-http/src/builder.rs`
- `rustycog/rustycog-http/src/lib.rs`
- `rustycog/rustycog-http/src/middleware_permission.rs`

## Key types

- `RouteBuilder` — fluent route composition with auth/permission/middleware
- `AppState` — shared state holding command service and user-id extractor
- `authenticated()` / `might_be_authenticated()` — explicit auth-mode selectors
