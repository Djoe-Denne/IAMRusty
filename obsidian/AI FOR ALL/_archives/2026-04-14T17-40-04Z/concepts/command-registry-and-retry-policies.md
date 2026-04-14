---
title: >-
  Command Registry and Retry Policies
category: concepts
tags: [commands, reliability, rust, visibility/internal]
sources:
  - IAMRusty/docs/COMMAND_PATTERN.md
  - IAMRusty/docs/COMMAND_RETRY_CONFIGURATION.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - rustycog/rustycog-command/src/lib.rs
  - rustycog/rustycog-command/src/registry.rs
summary: >-
  Commands are dispatched through registries that centralize validation, timeouts, retry policy, metrics, and tracing behind one execution surface.
provenance:
  extracted: 0.76
  inferred: 0.12
  ambiguous: 0.12
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:13:01.1911009Z
---

# Command Registry and Retry Policies

The repo's command pattern is more than a transport abstraction. In `[[projects/iamrusty/iamrusty]]` and the Manifesto guidance, it becomes the place where operational behavior is standardized and where `[[concepts/shared-rust-microservice-sdk]]` becomes concrete.

## Key Ideas

- `Command` requires a `command_type`, `command_id`, and `validate()` method, while `CommandContext` carries execution IDs, optional user IDs, request IDs, and free-form metadata for cross-cutting concerns.
- `CommandRegistry` validates before execution, looks up handlers by command type, wraps execution in a timeout, retries retryable failures, and records success/failure metrics from one place.
- Retryability is explicit in the current code: only `Infrastructure` and `Timeout` command errors are retried by the built-in `RetryPolicy`.
- `GenericCommandService` is intentionally thin, which keeps services routing all use cases through one registry-backed execution surface instead of hand-rolling orchestration in each handler.
- `rustycog-config` can supply retry settings to the registry through `CommandRetryConfig`, so jitter, backoff, and attempt counts are part of service configuration instead of hard-coded behavior.
- The config crate supports per-command override maps, but the registry consumes a resolved `RetryPolicy` rather than consulting override maps itself, so per-command behavior still depends on service composition. ^[ambiguous]
- Registration still accepts a `CommandErrorMapper`, yet the current execution path expects handlers to already return `CommandError`, so automatic domain-error translation is not visible in this crate alone. ^[ambiguous]

## Open Questions

- The wiki does not yet map which services use full per-command retry resolution versus only default registry behavior.
- The current tree shows where retries and metrics happen, but it does not show whether every service keeps identical timeout defaults or metrics collectors. ^[ambiguous]
- Command examples span multiple maturity levels, so not every documented feature should be assumed universally active. ^[ambiguous]

## Sources

- [[references/rustycog-crate-catalog]] — Code-backed inventory of the command/runtime crates
- [[references/iamrusty-command-execution]] — Main command and retry source summary
- [[concepts/structured-service-configuration]] — Config model that supplies retry policy
- [[skills/building-rustycog-services]] — Service-wiring workflow that uses this pattern
