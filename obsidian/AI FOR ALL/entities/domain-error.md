---
title: DomainError
category: entities
tags: [rustycog, errors, domain, visibility/internal]
sources:
  - rustycog/rustycog-core/src/error.rs
summary: DomainError is the domain-layer error enum that RustyCog maps into ServiceError for transport and orchestration layers.
provenance:
  extracted: 0.89
  inferred: 0.05
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# DomainError

`DomainError` is the domain-facing error type in `[[projects/rustycog/references/rustycog-core]]`.

## Key Ideas

- It captures domain-level failures such as entity-not-found, invalid input, business-rule violation, unauthorized operations, and external-service/internal failures.
- It includes convenience constructors (`entity_not_found`, `invalid_input`, `permission_denied`, and others) for consistent domain code.
- RustyCog provides `From<DomainError> for ServiceError`, which bridges domain failures into shared runtime error handling.
- This separation keeps domain services expressive while still supporting centralized retry/HTTP behavior in upper layers.

## Open Questions

- The enum documentation references one service context while the type is reused across the SDK. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/rustycog/references/rustycog-core]]
- [[entities/service-error]]
- [[projects/rustycog/references/rustycog-command]]
