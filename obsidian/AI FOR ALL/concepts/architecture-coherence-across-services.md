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
  - docs/adr/0100-services-metier-hexagonaux-rustycog.md
  - docs/adr/README.md
summary: >-
  Quatre slices RustyCog (ADR 0100). Divergences restantes : JWT/JWKS, errors, OpenAPI Hive, câblages OpenFGA. Logging Manifesto aligné (wiki août caduc).
provenance:
  extracted: 0.80
  inferred: 0.16
  ambiguous: 0.04
created: 2026-08-31T13:30:00Z
updated: 2026-09-12T10:20:00Z
---

# Architecture coherence across services

The 2026-08-29 comparison asked whether [[projects/manifesto/manifesto]], [[projects/telegraph/telegraph]], [[projects/iamrusty/iamrusty]], and [[projects/hive/hive]] apply the same RustyCog / IAM strategies. Layout, tests, and DI are coherent. Photograph rétroactive : [[projects/aiforall/decisions/0100-hexagone-rustycog]] (ADR 0100–0103).

## Shared scaffold

- Vertical slice: `domain` / `application` / `infra` / `http` / `setup` / `configuration` / `migration`.
- One composition root, `GenericCommandService`, `RouteBuilder` + `AppState`.
- Runtime prefixes: `/manifesto`, `/telegraph`, `/iam`, `/hive` — same contract as [[projects/aiforall/references/modular-monolith-runtime]].
- Golden path for scaffolding: Manifesto. Golden path for the IdP: IAMRusty.

## Structural divergences

- **JWT:** rustycog-http `UserIdExtractor` is HS256-only; IAM mints the same HMAC (`iss=iamrusty`, `aud=aiforall`). JWKS exists but is unused. See [[projects/aiforall/concepts/jwt-issuer-vs-consumer]] and ADR 0302.
- **Logging:** the August review said Manifesto still booted a hand-rolled `tracing_subscriber`. ADR 0100 photographs the **current** code: all four `configuration` crates reexport `rustycog::logger::setup_logging`. sentinel-sync / monolith may still use `tracing_subscriber::fmt`. ^[inferred]
- **Errors:** all four still map HTTP locally instead of `ServiceError::http_status_code`. Telegraph is the only **divergent** verdict: queue command failures flatten to `ServiceError::infrastructure`, which falsifies retry / ack-nack.
- **OpenAPI:** Hive is the only **divergent** contract — the spec is wider than `create_router` (ADR 0103). See [[projects/hive/concepts/command-registry-route-parity]].
- **OpenFGA:** four strategies, not one. IAM as IdP skipping `with_permission_on` is justified (ADR 0302 / 0400). ^[inferred]

## Related

- [[concepts/shared-rust-microservice-sdk]]
- [[concepts/event-driven-microservice-platform]]
- [[projects/aiforall/aiforall]]
- [[projects/aiforall/decisions/index]]
- [[skills/building-rustycog-services]]
