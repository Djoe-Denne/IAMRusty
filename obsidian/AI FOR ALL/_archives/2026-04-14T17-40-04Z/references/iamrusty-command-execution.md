---
title: >-
  IAMRusty Command Execution Guides
category: references
tags: [reference, commands, reliability, visibility/internal]
sources:
  - IAMRusty/docs/COMMAND_PATTERN.md
  - IAMRusty/docs/COMMAND_RETRY_CONFIGURATION.md
summary: >-
  Source summary for IAMRusty's registry-based command execution model and environment-specific retry tuning.
provenance:
  extracted: 0.91
  inferred: 0.06
  ambiguous: 0.03
created: 2026-04-14T17:03:47.5107188Z
updated: 2026-04-14T17:03:47.5107188Z
---

# IAMRusty Command Execution Guides

These sources explain how `[[projects/iamrusty/iamrusty]]` centralizes cross-cutting concerns through `[[concepts/command-registry-and-retry-policies]]` rather than scattering retries, logging, and error handling across handlers.

## Key Ideas

- `GenericCommandService` dispatches typed commands through a registry assembled by builders and factories.
- Command handlers stay focused while retries, timeouts, metrics, tracing, and validation are applied centrally.
- Error mapping is pluggable per command, which makes the command system easier to extract into `[[concepts/shared-rust-microservice-sdk]]` style tooling.
- Retry policy resolution follows a hierarchy of command-specific override, service default, then system fallback.
- Test profiles disable jitter for determinism, while production guidance favors conservative backoff and jitter.

## Open Questions

- Some examples appear to be reference patterns rather than a full inventory of every command currently wired in runtime. ^[ambiguous]
- Per-command retry resolution is described clearly, but the exact adoption level of that capability varies by service.

## Sources

- [[concepts/command-registry-and-retry-policies]] — Main reliability and dispatch pattern
- [[skills/building-rustycog-services]] — Workflow for wiring command registries in new services
- [[projects/iamrusty/iamrusty]] — First concrete project using these patterns in depth
