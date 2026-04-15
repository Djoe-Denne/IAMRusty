---
title: CommandRegistry
category: entities
tags: [rustycog, commands, orchestration, visibility/internal]
sources:
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-command/src/lib.rs
summary: CommandRegistry is the RustyCog execution hub that validates commands, routes handlers, enforces timeout/retry policy, and emits metrics.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# CommandRegistry

`CommandRegistry` is the typed command dispatch core from `[[projects/rustycog/references/rustycog-command]]`.

## Key Ideas

- Handlers are registered by command type and executed through one centralized runtime path.
- Registry execution always applies validation, timeout control, retry checks, tracing logs, and optional metrics collection.
- `RegistryConfig` defines timeout and retry policy and can be derived from `rustycog-config` retry settings.
- `CommandRegistryBuilder` is the ergonomic path most services use to compose registries in setup code.

## Open Questions

- Retry strategy consistency still varies by service composition, even though the shared registry supports one standard policy model. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-command]]
- [[entities/command-context]]
- [[concepts/command-registry-and-retry-policies]]
