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
  Manifesto-specific configuration notes layered on top of RustyCog's shared config model, especially around MANIFESTO-prefixed settings and the knobs that still drift from runtime behavior.
provenance:
  extracted: 0.82
  inferred: 0.08
  ambiguous: 0.10
created: 2026-04-19T11:49:06.1450368Z
updated: 2026-04-19T12:08:26.9393504Z
---

# Manifesto Runtime and Configuration

This page narrows the shared configuration story from `[[projects/rustycog/references/rustycog-config]]` to the places where `[[projects/manifesto/manifesto]]` adds service-specific settings or drifts from the generic RustyCog expectations.

## RustyCog Baseline

- `[[projects/rustycog/references/rustycog-config]]` explains the shared typed-config and env-prefix model that Manifesto reuses.
- `[[concepts/structured-service-configuration]]` captures the cross-service pattern: config files define runtime policy and the service composition root consumes those typed sections.
- `[[references/rustycog-service-construction]]` describes the generic startup sequence that this page specializes.

## Service-Specific Differences

- `ManifestoConfig` uses the `MANIFESTO` prefix and composes `server`, `logging`, `queue`, `database`, `scaleway`, and `service` sections in one typed service-local model.
- The checked-in TOML files emphasize `server`, `database`, `command.retry`, and `logging`, while `service.component_service` and `service.business` still depend more heavily on defaults or local overrides than the guides imply.
- `src/main.rs` follows the standard RustyCog startup flow, but the runtime it launches is specifically the Manifesto HTTP surface plus optional event publication and component-service integration.
- `setup/src/app.rs` creates a multi-queue event publisher from `config.queue` unless a custom publisher is injected, which makes publication behavior overrideable in tests or alternate boot paths.
- `ComponentServiceClient` reads `base_url` from `config.service.component_service.base_url`, but setup still hardcodes a `30` second timeout instead of using `timeout_seconds` from config. ^[ambiguous]
- `setup/src/config.rs` exposes `setup_logging()` that respects `config.logging.level`, but the live `main.rs` path still initializes tracing directly and falls back to `manifesto=info,rustycog=info` when env is absent. ^[ambiguous]
- The TOML files define `[command.retry]`, but `ManifestoConfig` has no `command` field and `ManifestoCommandRegistryFactory` still builds its registry with plain `CommandRegistryBuilder::new()`, so retry policy is not wired into the live registry path. ^[ambiguous]

## Open Questions

- Should Manifesto start consuming `service.component_service.timeout_seconds`, `logging.level`, and `[command.retry]`, or should those knobs be documented as guide-era leftovers until they are wired end to end? ^[ambiguous]
- Should the checked-in TOML files begin surfacing `service.component_service` and `service.business` explicitly, or is the default-only code path intentional for the MVP? ^[inferred]

## Sources

- [[projects/manifesto/manifesto]] - Service hub and current MVP framing.
- [[concepts/structured-service-configuration]] - Shared typed-config pattern that Manifesto specializes.
- [[references/rustycog-service-construction]] - Guide-versus-runtime drift that this page narrows to Manifesto.
- [[projects/rustycog/references/rustycog-config]] - Crate-level loader and config primitive details.
- [[projects/manifesto/references/manifesto-service]] - Composition-root and route-surface context for the same runtime.
