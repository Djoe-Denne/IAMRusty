---
title: DbConnectionPool
category: entities
tags: [rustycog, database, runtime, visibility/internal]
sources:
  - rustycog/rustycog-db/src/lib.rs
  - rustycog/rustycog-config/src/lib.rs
summary: DbConnectionPool encapsulates write/read SeaORM connections with replica fallback and round-robin read distribution.
provenance:
  extracted: 0.91
  inferred: 0.04
  ambiguous: 0.05
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# DbConnectionPool

`DbConnectionPool` is the shared database connection abstraction used by RustyCog-based services.

## Key Ideas

- It holds one primary write connection and one or more read connections.
- It supports explicit read replicas and falls back to primary if replicas are absent or unavailable.
- `get_read_connection()` applies round-robin strategy for multiple replicas.
- The pool is configured from `DatabaseConfig`, linking service config directly to DB runtime wiring.

## Open Questions

- The crate centralizes pooling defaults, but there is no documented per-service override strategy for environments with different load profiles. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-db]]
- [[entities/queue-config]]
- [[concepts/structured-service-configuration]]
