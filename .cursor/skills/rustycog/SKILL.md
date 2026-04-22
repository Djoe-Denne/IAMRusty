---
name: rustycog
description: Workflows and pitfalls for building Rust microservices on the RustyCog platform — covers scaffolding new services in the Manifesto style and the per-crate usage of rustycog-core, rustycog-config, rustycog-db, rustycog-command, rustycog-events, rustycog-http, rustycog-permission, rustycog-testing, and rustycog-logger. Use when scaffolding a new RustyCog/Manifesto service, wiring AppState, RouteBuilder, DbConnectionPool, CommandRegistry, PermissionsFetcher, QueueConfig, or DomainEvent, or when the user mentions any rustycog-* crate, hexagonal Rust services, the Manifesto template, or RustyCog setup/composition root work.
---

# RustyCog

Workflow guidance for building and maintaining Rust microservices on the RustyCog platform. Use the dispatch table below to load the right reference file for the task at hand — do not load every reference up front.

## When to use this skill

Trigger this skill when the user is:

- Scaffolding a new service that should look like the `Manifesto` reference service.
- Adding or changing wiring for any `rustycog-*` crate.
- Touching `AppState`, `RouteBuilder`, `DbConnectionPool`, `CommandRegistryBuilder`, `PermissionChecker`, `OpenFgaClientConfig`, `setup_logging`, `QueueConfig`, or `DomainEvent`.
- Debugging a RustyCog setup pitfall (config prefix surprises, `max_attempts = 0` disabling retries, missing OpenFGA type, etc.).

## Dispatch table

Pick the smallest set of references that match the task. Load each only when needed.

| Task | Reference |
|------|-----------|
| Scaffolding a brand-new service end-to-end | [references/building-rustycog-services.md](references/building-rustycog-services.md) |
| Domain/service error types, retryability, HTTP status mapping | [references/using-rustycog-core.md](references/using-rustycog-core.md) |
| Typed config loading, env prefixes, `QueueConfig`, `load_config_part` | [references/using-rustycog-config.md](references/using-rustycog-config.md) |
| `DbConnectionPool`, read/write split, replica fallback | [references/using-rustycog-db.md](references/using-rustycog-db.md) |
| Defining `Command`/`CommandHandler`, registry, retry policy | [references/using-rustycog-command.md](references/using-rustycog-command.md) |
| Domain events, publishers/consumers, multi-queue setup | [references/using-rustycog-events.md](references/using-rustycog-events.md) |
| `RouteBuilder`, auth modes, middleware composition | [references/using-rustycog-http.md](references/using-rustycog-http.md) |
| `PermissionChecker`, OpenFGA-backed guards, with_permission_on | [references/using-rustycog-permission.md](references/using-rustycog-permission.md) |
| Integration tests, `setup_test_server`, Kafka/SQS testcontainers, `OpenFgaMockService` | [references/using-rustycog-testing.md](references/using-rustycog-testing.md) |
| Authoring a wiremock-backed fixture for an HTTP collaborator (incl. `reset()` and cache caveats) | `.cursor/skills/creating-wiremock-fixtures/SKILL.md` |
| `setup_logging`, `HasLoggingConfig`, Loki feature wiring | [references/using-rustycog-logger.md](references/using-rustycog-logger.md) |

## Cross-cutting rules

These hold across every RustyCog crate and override anything that contradicts them in a single reference file:

- **One composition root.** Wire concrete dependencies (pools, registries, fetchers, publishers) once in `setup` and pass them down — never recreate them per handler.
- **One execution surface for commands.** All HTTP and queue adapters go through the same `GenericCommandService` so retry, validation, and tracing are consistent.
- **Config sections are shared contracts.** `server`, `database`, `logging`, `queue`, and `command` sections must match the structs RustyCog expects; do not invent parallel shapes.
- **`load_config_part("server")` reads `SERVER_*` env overrides**, not your service prefix. Use the full typed loader unless you specifically want a section's own prefix.
- **Permission middleware takes one builder call:** `.with_permission_on(Permission, object_type)`. The shared `Arc<dyn PermissionChecker>` on `AppState` answers every decision — there is no per-route fetcher.
- **Permission middleware extracts the deepest UUID-shaped path segment only.** Non-UUID path segments (e.g. `{component_type}`) are skipped.
- **Object type must exist in `openfga/model.fga`.** Typos fail closed with 403 plus a logged OpenFGA error.
- **`OpenFgaClientConfig.cache_ttl_seconds = Some(0)` is the test-config opt-out for `CachedPermissionChecker`.** The composition root must honor it (skip the cache decoration entirely when 0); otherwise grant-then-revoke flows in tests serve a stale allow.
- **`max_attempts = 0` disables retries.** It does not mean "default" or "infinite" — set it intentionally.
- **`setup_logging` is a global singleton.** Call it exactly once, early, and never alongside hand-rolled `tracing_subscriber` setup.
- **Queue factories can degrade to no-op.** A "successful" startup does not prove the transport is live — add an explicit health check.

## Workflow when starting a new service

If the task is "build a new RustyCog service from scratch", follow this order regardless of which crates are involved:

1. Read [references/building-rustycog-services.md](references/building-rustycog-services.md) first — it sets the vertical-slice shape.
2. Decide `rustycog-meta` umbrella vs individual `rustycog-*` crates and lock that into the workspace `Cargo.toml`.
3. Write the typed config struct (see `using-rustycog-config`) and decide explicitly between `setup_logging` and hand-rolled tracing.
4. Build the composition root: `DbConnectionPool` → repositories → command registry → `AppState`.
5. Compose routes through `RouteBuilder`; add permissions only on routes that need them.
6. Add integration tests with `setup_test_server` before adding Kafka/SQS-backed checks.

## Provenance note

These references are distilled from the AIForAll Obsidian vault under `obsidian/AI FOR ALL/skills/`. Source files in the original RustyCog and Manifesto repositories are listed at the top of each reference. Items marked `^[ambiguous]` are points where the source guidance and current code disagree — flag them to the user rather than picking silently.
