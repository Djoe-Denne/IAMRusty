---
title: >-
  Manifesto Runtime and Configuration
category: references
tags: [reference, configuration, projects, visibility/internal]
sources:
  - Manifesto/src/main.rs
  - Manifesto/config/default.toml
  - Manifesto/config/development.toml
  - Manifesto/config/test.toml
  - Manifesto/configuration/src/lib.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/setup/src/config.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
summary: >-
  Code-backed map of Manifesto's MANIFESTO-prefixed config loading, queue publisher setup, and the runtime drift between configured knobs and live startup behavior.
provenance:
  extracted: 0.82
  inferred: 0.08
  ambiguous: 0.10
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-04-19T11:49:06.1450368Z
---

# Manifesto Runtime and Configuration

These sources describe how `[[projects/manifesto/manifesto]]` loads configuration, starts its HTTP runtime, and wires optional event publishing and component-service integration.

## Key Ideas

- `ManifestoConfig` uses the `MANIFESTO` prefix and composes `server`, `logging`, `queue`, `database`, `scaleway`, and `service` sections in one typed config model.
- The checked-in TOML files currently focus on `server`, `database`, `command.retry`, and `logging`, while `service.component_service` and `service.business` mostly rely on code defaults unless env or local overrides provide them.
- `src/main.rs` calls `load_config()`, clones `config.server`, builds `Application`, and starts the `RouteBuilder`-backed HTTP server.
- `setup/src/app.rs` creates a multi-queue event publisher from `config.queue` unless a custom publisher is injected, which is how tests or alternate bootstraps can override publication behavior.
- `ComponentServiceClient` reads `base_url` from `config.service.component_service.base_url`, but setup currently constructs the client with a hardcoded `30` second timeout instead of using `timeout_seconds` from config. Conflict to resolve. ^[ambiguous]
- `setup/src/config.rs` exposes `setup_logging()` that respects `config.logging.level`, but the live `main.rs` path initializes tracing directly and falls back to `manifesto=info,rustycog=info` when env is absent. Conflict to resolve. ^[ambiguous]
- The checked-in TOML files define `[command.retry]`, but `ManifestoConfig` has no `command` field and `ManifestoCommandRegistryFactory` still builds its registry with plain `CommandRegistryBuilder::new()`, so retry settings are not wired into the live registry path. Conflict to resolve. ^[ambiguous]

## Open Questions

- Should Manifesto start consuming `service.component_service.timeout_seconds`, `logging.level`, and `[command.retry]`, or should those knobs be documented as guide-era leftovers until they are wired end to end? ^[ambiguous]
- Should the checked-in TOML files begin surfacing `service.component_service` and `service.business` explicitly, or is the default-only code path intentional for the MVP? ^[inferred]

## Sources

- [[projects/manifesto/manifesto]] - Service hub and current MVP framing.
- [[concepts/structured-service-configuration]] - Shared typed-config pattern that Manifesto specializes.
- [[references/rustycog-service-construction]] - Guide-versus-runtime drift that this page narrows to Manifesto.
- [[projects/rustycog/references/rustycog-config]] - Crate-level loader and config primitive details.
- [[projects/manifesto/references/manifesto-service]] - Composition-root and route-surface context for the same runtime.
