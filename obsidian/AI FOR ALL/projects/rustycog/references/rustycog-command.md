---
title: RustyCog Command
category: references
tags: [reference, rustycog, commands, visibility/internal]
sources:
  - rustycog/rustycog-command/src/lib.rs
  - rustycog/rustycog-command/src/registry.rs
  - rustycog/rustycog-config/src/lib.rs
summary: rustycog-command provides the command traits, command context, registry, retry policy, timeout handling, and execution metrics pipeline.
provenance:
  extracted: 0.88
  inferred: 0.06
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# RustyCog Command

`rustycog-command` implements the typed command runtime used by platform services and integrated into `[[projects/rustycog/references/rustycog-http]]`.

## Key Ideas

- `Command`, `CommandHandler`, and `CommandContext` define a common contract for validated command execution.
- `CommandRegistry` stores type-erased handlers and orchestrates validation, timeout handling, retry logic, tracing, and metrics.
- `RetryPolicy` supports exponential backoff with optional jitter and classifies retryable errors (`Infrastructure` and `Timeout`).
- `RegistryConfig::from_retry_config()` bridges runtime retry settings from `rustycog-config`.
- `CommandRegistryBuilder` gives services a fluent way to register handlers and produce one shared registry.
- The command layer is transport-agnostic and can be reused by HTTP handlers, queue consumers, or test harnesses.

## Linked Entities

- [[entities/command-registry]]
- [[entities/command-context]]
- [[entities/service-error]]

## Open Questions

- The crate exposes mapper interfaces, but consistency of domain-to-command error mapping still depends on each service factory implementation. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-crate-catalog]]
- [[concepts/command-registry-and-retry-policies]]
- [[projects/rustycog/rustycog]]
