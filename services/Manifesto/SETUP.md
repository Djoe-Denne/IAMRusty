# Manifesto Setup

## Prerequisites

- Rust stable
- PostgreSQL 12+
- Access to the workspace root so shared `rustycog` crates resolve normally
- Optional: Kafka/SQS only if you plan to enable queue-backed runtime behavior explicitly

## Database

Create databases:

```bash
createdb manifesto_dev
createdb manifesto_test
```

Run migrations:

```bash
cargo run -p manifesto-migration -- up
```

Roll back the latest migration batch:

```bash
cargo run -p manifesto-migration -- down
```

## Configuration

Manifesto uses layered TOML plus `MANIFESTO_` environment overrides:

- `config/default.toml`
- `config/development.toml`
- `config/test.toml`
- environment variables with `__` for nesting

Key sections:

- `auth.jwt.hs256_secret`: service-side JWT verifier secret for the current HS256-only shared verifier path
- `database`: primary DB connection and credentials
- `logging.level`: runtime log level
- `command.retry`: shared command retry policy
- `queue`: queue transport choice; checked-in local/test configs set `type = "disabled"`
- `service.component_service.base_url`
- `service.component_service.api_key`
- `service.component_service.timeout_seconds`
- `service.business.*`: quotas, pagination defaults, and validation limits

Example overrides:

```bash
export RUN_ENV=development
export MANIFESTO_DATABASE__HOST=localhost
export MANIFESTO_DATABASE__CREDS__USERNAME=postgres
export MANIFESTO_DATABASE__CREDS__PASSWORD=postgres
export MANIFESTO_AUTH__JWT__HS256_SECRET=rustycog-dev-hs256-secret
export MANIFESTO_SERVICE__COMPONENT_SERVICE__BASE_URL=http://localhost:9000
```

## Queue Behavior

The checked-in `default`, `development`, and `test` configs all disable queues on purpose:

```toml
[queue]
type = "disabled"
```

This keeps local boots and default tests stable without accidental broker assumptions.

If you want queue-backed publication or apparatus consumption, override the full queue section explicitly for that environment.

## Running the Service

```bash
export RUN_ENV=development
cargo run -p manifesto-service
```

By default the service listens on `http://127.0.0.1:8082` in development.

## Running Tests

Shared Manifesto tests:

```bash
export RUN_ENV=test
cargo test -p manifesto-service
```

Shared HTTP auth middleware tests:

```bash
cargo test -p rustycog-http --test permission_middleware_tests
```

## Operational Notes

- Component add/validation flows fail closed if the configured component service is unavailable or returns non-success responses.
- Apparatus event consumption is wired in `setup/src/app.rs`, but it only runs when queue config resolves to a real consumer.
- Project/component resource reads use optional auth plus permission middleware; `GET /api/projects` uses optional auth plus visibility filtering in the application/repository layers.
- Component add/remove fails if the matching component-instance ACL resource cannot be synchronized.
- `ComponentResponse.endpoint` and `access_token` are intentionally still unset pending a later provisioning handoff design.

## Troubleshooting

### Configuration does not load

- Verify `RUN_ENV`
- Check TOML syntax in `config/*.toml`
- Verify nested env vars use `__`
- Confirm `MANIFESTO_AUTH__JWT__HS256_SECRET` is set when using auth-protected routes outside checked-in dev/test defaults

### Component add flows fail

- Confirm `service.component_service.base_url` points at a reachable service
- Confirm `service.component_service.api_key` matches what the catalog expects
- Check upstream response codes; Manifesto does not fall back to mock component data anymore

### Queue behavior is missing

- Confirm the active config does not still have `[queue] type = "disabled"`
- Confirm the chosen transport is reachable
- Remember that the checked-in local/test configs disable queues by default
