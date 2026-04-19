# Manifesto Service

Manifesto manages projects, attached components, and project membership for AIForAll.

## Current Status

Manifesto is a production-ready baseline after the April 2026 remediation pass.

- Bearer auth now uses the shared verified HS256-only service-side verifier.
- Optional-auth resource reads evaluate anonymous callers correctly, and `GET /api/projects` filters by caller visibility/access.
- Non-public project and component reads require real access.
- Component catalog calls fail closed and honor configured `api_key` and `timeout_seconds`.
- Component add/remove fails if the matching component-instance ACL resource cannot be synchronized.
- `logging.level`, `[command.retry]`, and business limits are wired into live runtime behavior.
- Apparatus component-status consumption is wired into startup when queue config resolves to a real consumer.
- Checked-in `default`, `development`, and `test` configs keep queues disabled by default so local/test boots do not inherit broker defaults accidentally.

## Overview

Manifesto owns three core areas:

- **Projects**: ownership, status, visibility, collaboration flags, and data classification
- **Components**: external capabilities attached to projects, each with its own lifecycle
- **Members**: project-scoped permissions across `project`, `component`, and `member` resources

Current lifecycle models:

- Project status: `draft -> active -> archived`
- Component status: `pending -> configured -> active -> disabled`

## Runtime Notes

- `GET /api/projects` is optionally authenticated and filters by caller visibility/access in the service and repository layers.
- Public project/component resource reads can succeed anonymously through optional auth plus permission middleware; non-public reads require permission.
- Project creation bootstraps owner access immediately.
- Component add/remove keeps the matching component-instance ACL resource in sync or fails the request.
- Manifesto publishes its own domain events on a best-effort basis.
- When queue support is enabled, Manifesto also consumes apparatus `component_status_changed` events and reconciles stored component state.
- `ComponentResponse.endpoint` and `ComponentResponse.access_token` are still `None`; provisioning handoff is not implemented yet.

## Getting Started

Run migrations:

```bash
cargo run -p manifesto-migration -- up
```

Run the service:

```bash
cargo run -p manifesto-service
```

Run tests:

```bash
cargo test -p manifesto-service
```

## Configuration

Manifesto uses the shared RustyCog config model with the `MANIFESTO_` prefix.

Important sections:

- `auth.jwt.hs256_secret` (current shared verifier path is HS256-only)
- `database`
- `logging.level`
- `command.retry`
- `queue`
- `service.component_service`
- `service.business`

The checked-in configs set:

- development/test JWT secrets for local use
- queue type to `disabled`
- retry config defaults
- local component-service defaults

If you want queue-backed behavior locally, explicitly override the queue section instead of relying on library defaults.

## Documentation

- [`IMPLEMENTATION_STATUS.md`](IMPLEMENTATION_STATUS.md) - current runtime truth and remaining limits
- [`SETUP.md`](SETUP.md) - local setup, config, and run/test commands
- [`docs/rustycog-service-build-guide.md`](docs/rustycog-service-build-guide.md) - RustyCog service construction guide
- [`docs/rustycog-implementation-and-usage-guide.md`](docs/rustycog-implementation-and-usage-guide.md) - Manifesto-specific implementation notes
- [`openspecs.yaml`](openspecs.yaml) - OpenAPI surface

## License

Workspace license applies.
