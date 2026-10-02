---
title: >-
  AIForAll
category: project
tags: [platform, microservices, rust, visibility/internal]
sources:
  - README.md
  - Cargo.toml
  - monolith/Cargo.toml
  - monolith/src/runtime.rs
  - docs/reviews/iam-architecture-comparison.md
  - .agents/skills/rustycog-submodule/SKILL.md
  - C:/Users/djden/source/repos/AIForAll/.env
  - docs/adr/README.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/adr/0007-closeout.md
summary: >-
  Workspace RustyCog. P2 et P3 Implemented. ADR-0008 Accepted / Implemented
  (Kind V1). JWT 0304 Partial. Mesh 0308 Partial (§7 Compose). Gold path 0605 livré en réalité. Factory/host hors livré.
provenance:
  extracted: 0.80
  inferred: 0.16
  ambiguous: 0.04
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-10-01T16:45:00Z
---

# AIForAll

AIForAll is a Rust-based microservices workspace centered on [[projects/iamrusty/iamrusty]], [[projects/telegraph/telegraph]], [[projects/hive/hive]], [[projects/manifesto/manifesto]], [[projects/lazaret/lazaret]], the [[projects/rustycog/rustycog]] SDK (git submodule), [[projects/sentinel-sync/sentinel-sync]], and event crates such as [[projects/hive-events/hive-events]].

## Key Ideas

- The workspace is a [[concepts/event-driven-microservice-platform]] with shared local infrastructure (Postgres, LocalStack, OpenFGA).
- A top-level Docker Compose flow runs the services plus PostgreSQL, LocalStack, and OpenFGA.
- Two runtime modes: standalone microservice binaries and the `oodhive-monolith` modular monolith under one HTTP listener.
- Shared patterns live in [[concepts/shared-rust-microservice-sdk]]. The SDK tree is pinned as [[projects/aiforall/concepts/rustycog-git-submodule]].
- August 2026 reviews show one hexagonal scaffold with remaining gaps on JWT/JWKS, errors, OpenAPI Hive, and OpenFGA wiring — [[concepts/architecture-coherence-across-services]]. Photograph rétroactive : [[projects/aiforall/decisions/index]].
- Queue factories must surface rustycog no-ops on `/ready` — [[projects/aiforall/concepts/queue-readiness-signaling]].
- Project work is routed through [[projects/aiforall/concepts/orchestrator-agent-harness]]. Architecture vivante : [[projects/aiforall/concepts/architecte-agent]].
- Apparatus P0 crates live at workspace root ; P2 ticker is in Manifesto (**Implemented**) — [[projects/manifesto/concepts/apparatus-p2-reconciliation]] (ADR 0006). Frontière P3 : [[projects/lazaret/lazaret]] (ADR 0007 **Implemented** T1–T14b ; hors-jalon APP-05 / G/E / K8s-as-P3 / 2e proto). Moteur P4 : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (ADR-0008 **Accepted** / **Implemented**, Kind V1 ; dette D-TRANSIT-TCB / D-ADMB / D-PROD). Gates préprod : [[projects/manifesto/decisions/0009-0011-gates-preprod]]. Mesh HTTPS : [[projects/aiforall/concepts/https-platform-mesh]]. Factory/host hors livré (ADR 0406).
- Access JWT cible RS256 / JWKS : [[projects/aiforall/decisions/0304-access-jwt-trust]] (0304 Partial). Mesh AuthN : [[projects/aiforall/decisions/0308-mesh-authn-jwt]] (Partial, §7 Compose). Connecteurs IdP : [[projects/iamrusty/decisions/index]] (0407–0411 Implemented). Gold path Kind : [[projects/aiforall/decisions/0605-gold-path-kind]].

## Runtime Modes

- **Microservices:** `iam-service`, `telegraph-service`, `hive-service`, `manifesto-service`, and `lazaret-service` remain independently runnable packages.
- **Modular monolith:** `[[projects/aiforall/references/modular-monolith-runtime]]` documents the `oodhive-monolith` package, which composes IAMRusty, Telegraph, Hive, Manifesto, and Lazaret routers at `/iam`, `/telegraph`, `/hive`, `/manifesto`, and `/lazaret` while keeping SQS/event semantics unchanged.

## Roadmap

- [[projects/aiforall/roadmap]] still tracks Sentinel Sync tests, transactional DB load, the outbox, and IAM provider adapters. The 2026-08 wave added Sonar/Clippy policy, rustycog pinning, readiness signaling, and architecture-coherence follow-ups.

## Skills

- [[projects/aiforall/skills/running-aiforall-runtime-modes]]
- [[projects/aiforall/skills/fixing-sonar-clippy-in-services]]
- [[projects/aiforall/skills/running-parallel-sonar-lanes]]
- [[projects/aiforall/skills/running-apparatus-p0-tests]]
- [[projects/aiforall/skills/running-apparatus-p1-tests]]
- [[projects/aiforall/skills/running-apparatus-p2-tests]]
- [[projects/aiforall/skills/running-apparatus-p4-tests]]
- [[projects/lazaret/skills/running-apparatus-p3-tests]]
- GitHub handbook: `docs/README.md` (JWT, nouveau service, parcours métier). Agent skill: `.agents/skills/aiforall-new-service/SKILL.md`.

## Décisions

- [[projects/iamrusty/decisions/index]] — vague 3 IAM-IdP (ADR **Accepted** / **Implemented** 0407–0411)
- [[projects/aiforall/decisions/index]] — vague 2 photo + Vague 4 (0600–0602 Proposed ; 0604/0605 Réalité Implemented)
- [[projects/manifesto/decisions/index]] — vague 1 Apparatus (0002–0008 Implemented ; 0001 Partial ; 0009–0011 Proposed)

## Recent history

- [[projects/aiforall/references/cursor-history-2026-09]]
- [[projects/aiforall/references/cursor-history-2026-04-to-08]]
- [[journal/2026-09-27]]
- [[journal/2026-09-22]]
- [[journal/2026-09-20]]
- [[journal/2026-09-18]]
- [[journal/2026-09-17]]
- [[journal/2026-09-13]]
- [[journal/2026-09-12]]
- [[journal/2026-09-11]]
- [[journal/2026-09-09]]
- [[journal/2026-08-31]]
- [[journal/2026-09-01]]
- [[journal/2026-09-02]]

## Open Questions

- Event crates: [[projects/hive-events/hive-events]] plus `iam-events`, `manifesto-events`, `telegraph-events` (see `docs/platform/events-outbox.md`).
- Telegraph SMS: still not a live delivery mode; handbook `docs/functional/notifications.md` documents email + in-app only.

## Sources

- [[references/aiforall-platform]] — Repository overview and shared dev workflow
- [[projects/aiforall/references/modular-monolith-runtime]] — Runtime-mode decision and monolith composition notes
- [[projects/lazaret/lazaret]] — P3 capability boundary (Implemented T1–T14b, A-DEC 2026-09-20)
- [[projects/aiforall/concepts/https-platform-mesh]] — T14b dual-bind + CA mesh
