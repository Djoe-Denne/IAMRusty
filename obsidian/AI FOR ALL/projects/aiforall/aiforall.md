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
summary: >-
  Repo-level map of AIForAll: four RustyCog services, rustycog git submodule, dual runtime modes, and the 2026-08 coherence / Sonar wave.
provenance:
  extracted: 0.78
  inferred: 0.18
  ambiguous: 0.04
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-09-02T18:00:00Z
---

# AIForAll

AIForAll is a Rust-based microservices workspace centered on [[projects/iamrusty/iamrusty]], [[projects/telegraph/telegraph]], [[projects/hive/hive]], [[projects/manifesto/manifesto]], the [[projects/rustycog/rustycog]] SDK (git submodule), [[projects/sentinel-sync/sentinel-sync]], and event crates such as [[projects/hive-events/hive-events]].

## Key Ideas

- The workspace is a [[concepts/event-driven-microservice-platform]] with shared local infrastructure (Postgres, LocalStack, OpenFGA).
- A top-level Docker Compose flow runs the services plus PostgreSQL, LocalStack, and OpenFGA.
- Two runtime modes: standalone microservice binaries and the `oodhive-monolith` modular monolith under one HTTP listener.
- Shared patterns live in [[concepts/shared-rust-microservice-sdk]]. The SDK tree is pinned as [[projects/aiforall/concepts/rustycog-git-submodule]].
- August 2026 reviews show one hexagonal scaffold with structural gaps on JWT, logging, errors, OpenAPI, and OpenFGA — [[concepts/architecture-coherence-across-services]].
- Queue factories must surface rustycog no-ops on `/ready` — [[projects/aiforall/concepts/queue-readiness-signaling]].

## Runtime Modes

- **Microservices:** `iam-service`, `telegraph-service`, `hive-service`, and `manifesto-service` remain independently runnable packages.
- **Modular monolith:** `[[projects/aiforall/references/modular-monolith-runtime]]` documents the `oodhive-monolith` package, which composes IAMRusty, Telegraph, Hive, and Manifesto routers at `/iam`, `/telegraph`, `/hive`, and `/manifesto` while keeping SQS/event semantics unchanged.

## Roadmap

- [[projects/aiforall/roadmap]] still tracks Sentinel Sync tests, transactional DB load, the outbox, and IAM provider adapters. The 2026-08 wave added Sonar/Clippy policy, rustycog pinning, readiness signaling, and architecture-coherence follow-ups.

## Skills

- [[projects/aiforall/skills/running-aiforall-runtime-modes]]
- [[projects/aiforall/skills/fixing-sonar-clippy-in-services]]
- [[projects/aiforall/skills/running-parallel-sonar-lanes]]
- GitHub handbook: `docs/README.md` (JWT, nouveau service, parcours métier). Agent skill: `.agents/skills/aiforall-new-service/SKILL.md`.

## Recent history

- [[projects/aiforall/references/cursor-history-2026-04-to-08]]
- [[journal/2026-08-31]]
- [[journal/2026-09-01]]
- [[journal/2026-09-02]]

## Open Questions

- Event crates: [[projects/hive-events/hive-events]] plus `iam-events`, `manifesto-events`, `telegraph-events` (see `docs/platform/events-outbox.md`).
- Telegraph SMS: still not a live delivery mode; handbook `docs/functional/notifications.md` documents email + in-app only.

## Sources

- [[references/aiforall-platform]] — Repository overview and shared dev workflow
- [[projects/aiforall/references/modular-monolith-runtime]] — Runtime-mode decision and monolith composition notes
