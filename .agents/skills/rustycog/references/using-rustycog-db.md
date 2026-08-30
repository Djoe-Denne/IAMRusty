# Using RustyCog DB

Use this guide when integrating `rustycog-db` into service setup.

## Workflow

- Define `DatabaseConfig` in your service config and load it before building repositories.
- Create one shared `DbConnectionPool` with `DbConnectionPool::new(&db_config)`.
- Pass `get_write_connection()` into write repositories and `get_read_connection()` into read/query repositories.
- Configure read replicas only when needed; fallback to primary is automatic when none are available.
- Keep repository wiring in setup/composition root so business logic remains storage-agnostic.

## Common Pitfalls

- Sending all read workloads to `get_write_connection()` and bypassing replica routing.
- Ignoring replica connection failures in startup logs.
- Recreating pools in many handlers instead of sharing one pool instance.

## Source files

- `rustycog/rustycog-db/src/lib.rs`
- `rustycog/rustycog-config/src/lib.rs`

## Key types

- `DbConnectionPool` — shared pool, one per process
- `DatabaseConfig` — config section consumed by the pool constructor
