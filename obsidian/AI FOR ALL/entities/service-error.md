---
title: ServiceError
category: entities
tags: [rustycog, errors, runtime, visibility/internal]
sources:
  - rustycog/rustycog-core/src/error.rs
summary: ServiceError is RustyCog's shared runtime error envelope with category, retryability, and HTTP status mapping helpers.
provenance:
  extracted: 0.9
  inferred: 0.04
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# ServiceError

`ServiceError` is the main cross-layer runtime error type defined in `[[projects/rustycog/references/rustycog-core]]`.

## Key Ideas

- It models validation, authentication, authorization, business, infrastructure, not-found, conflict, rate-limit, unavailable, timeout, and internal failures.
- `http_status_code()` gives one canonical status mapping used by HTTP-facing code paths.
- `is_retryable()` marks infrastructure/transient categories used by retry logic in `[[entities/command-registry]]`.
- Builder helpers (`validation_field`, `not_found_resource`, and similar) reduce ad hoc error construction in services.

## Open Questions

- Domain-to-service conversion behavior is centralized, but service teams still need explicit conventions for preserving domain context in messages and codes. ^[inferred]

## Sources

- [[projects/rustycog/references/rustycog-core]]
- [[entities/domain-error]]
- [[projects/rustycog/references/rustycog-command]]
