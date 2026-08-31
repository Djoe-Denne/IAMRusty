---
title: >-
  Architecture coherence across services
category: concepts
tags: [architecture, rustycog, platform, visibility/internal]
sources:
  - docs/reviews/iam-architecture-comparison.md
  - docs/reviews/iam-manifesto-architecture.md
  - docs/reviews/iam-telegraf-architecture.md
  - docs/reviews/iam-rusty-architecture.md
  - docs/reviews/iam-hive-architecture.md
  - cursor-conversation/architecture-reviews-2026-08-29
summary: >-
  The four RustyCog services share one hexagonal scaffold; they diverge on JWT, logging, errors, OpenAPI fidelity, and OpenFGA wiring.
provenance:
  extracted: 0.78
  inferred: 0.18
  ambiguous: 0.04
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Architecture coherence across services

The 2026-08-29 comparison asked whether [[projects/manifesto/manifesto]], [[projects/telegraph/telegraph]], [[projects/iamrusty/iamrusty]], and [[projects/hive/hive]] apply the same RustyCog / IAM strategies. Layout, tests, and DI are coherent. The remaining gaps are structural, not cosmetic.

## Shared scaffold

- Vertical slice: `domain` / `application` / `infra` / `http` / `setup` / `configuration` / `migration`.
- One composition root, `GenericCommandService`, `RouteBuilder` + `AppState`.
- Runtime prefixes: `/manifesto`, `/telegraph`, `/iam`, `/hive` — same contract as [[projects/aiforall/references/modular-monolith-runtime]].
- Golden path for scaffolding: Manifesto. Golden path for the IdP: IAMRusty.

## Structural divergences

- **JWT:** rustycog-http `UserIdExtractor` is HS256-only; IAMRusty still guards RS256 issuance outside `test-relaxed-jwt`. See [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].
- **Logging:** Manifesto still boots a hand-rolled `tracing_subscriber`. The other three call `rustycog::logger::setup_logging`. The skill says call the singleton once and never alongside a custom subscriber. ^[ambiguous]
- **Errors:** all four still map HTTP locally instead of `ServiceError::http_status_code`. Telegraph is the only **divergent** verdict: queue command failures flatten to `ServiceError::infrastructure`, which falsifies retry / ack-nack.
- **OpenAPI:** Hive is the only **divergent** contract — the spec is wider than `create_router`, and `/roles` was live while the registry was empty (500). See [[projects/hive/concepts/command-registry-route-parity]].
- **OpenFGA:** four strategies, not one. Telegraph mark-read can 403 fail-closed if `NotificationCreated` never writes tuples. IAMRusty as IdP skipping `with_permission_on` is justified N/A. ^[inferred]

## Related

- [[concepts/shared-rust-microservice-sdk]]
- [[concepts/event-driven-microservice-platform]]
- [[projects/aiforall/aiforall]]
- [[skills/building-rustycog-services]]
