---
title: Wiki Index
category: navigation
tags: [index, navigation, wiki]
summary: >-
  Index wiki AIForAll : ADR vague 2, Apparatus P0/P1/P2 Partial (HEAD 7455ee5).
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
updated: 2026-09-13T10:25:00Z
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - docs/adr/README.md
  - docs/apparatus-p0-implementation-prompt.md
  - docs/apparatus-p1-implementation-prompt.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
---

# Wiki Index

Central entry point for this vault. Use the area indexes for full catalogs; use project hubs for service-scoped concepts, skills, and references.

## Browse by area

- [[projects/index]] — service hubs and project knowledge areas
- [[concepts/index]] — shared concepts plus pointers into project concept indexes
- [[entities/index]] — core business nouns plus per-service entity inventories
- [[skills/index]] — portable skills plus pointers into project skill indexes
- [[references/index]] — platform references plus pointers into project reference indexes

## Project homes

- [[projects/aiforall/aiforall]] — platform overview and roadmap
- [[projects/iamrusty/iamrusty]] — IAM and OAuth
- [[projects/hive/hive]] — organizations and permissions
- [[projects/hive-events/hive-events]] — Hive domain events
- [[projects/telegraph/telegraph]] — notifications and communication
- [[projects/manifesto/manifesto]] — project-service MVP and RustyCog blueprint
- [[projects/rustycog/rustycog]] — shared Rust SDK (crate map: [[projects/rustycog/references/index]])
- [[projects/sentinel-sync/sentinel-sync]] — centralized OpenFGA authorization and the sync worker

## Architecture — ADR

- [[projects/aiforall/decisions/index]] — vague 2 rétroactive (0100–0502) : hexagone, tests, events, services, plateforme.
- [[projects/manifesto/decisions/index]] — vague 1 Apparatus Accepted ; réalité `Partial` (`docs/adr/`).

## Fonctionnalité — Apparatus

- [[projects/manifesto/concepts/apparatus-p0-contracts]] — crates P0 livrées + audit + P0.1 42/42.
- [[projects/manifesto/concepts/apparatus-p1-persistence]] — persistance P1 T1–T7 (commit `7455ee5`).
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]] — ticker in-process Partial ; P2.1 ensuite.
- [[projects/manifesto/concepts/apparatus-platform]] — vision ; Factory/host encore absents.
- [[projects/manifesto/references/apparatus-implementation-plan]] — phases et preuves P0/P1/P2.

## Recent Additions

- [[journal/2026-09-13]] — P2 Partial dans HEAD `7455ee5` ; working tree propre ; P2.1 ensuite.
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]] — ticker, T1–T7, écarts A/D/I.
- [[projects/aiforall/decisions/index]] — 21 ADR rétroactives 0100–0502.
- [[journal/2026-09-12]] — audit P0, P1 TDD 78 tests, ban Muse Spark, vague 2 ADR.
- [[projects/aiforall/skills/running-apparatus-p1-tests]] — `cargo test --test apparatus_p1_t* -- --test-threads=1`.
- [[projects/manifesto/decisions/0001-apparatus-binding]] — [[projects/manifesto/decisions/0005-apparatus-protocol]] — pages ADR vague 1.
- [[journal/2026-09-11]] — ADR, harnais orchestrator, P0 crates + tests.
- [[projects/aiforall/concepts/orchestrator-agent-harness]] — contrôle plane Cursor.
- [[projects/aiforall/skills/running-apparatus-p0-tests]] — `cargo test --features test-harness`.
- [[journal/2026-09-09]] — eight Cursor sessions since last wiki sync.
- [[projects/aiforall/references/cursor-history-2026-09]] — visibility/join, AuthZ correction, rustycog `main` pin.
- [[projects/manifesto/concepts/immediate-membership-acl]] — DB membership gate; Suspended admin-only.
- [[projects/manifesto/concepts/membership-restore-and-cas]] — grace restore, unique owner, CAS + outbox.
- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]] — envelope, v1 no-op, monotonic revision.
- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]] — exact project/component diff; OpenFGA 1.5 Read.
- [[projects/rustycog/references/isolated-wiremock-fixture]] — `MockServerFixture::isolated()`.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] — org-owned public/private; join is Direct/read; L-PARTNERSHIP open.
- GitHub handbook `docs/README.md` (2026-09-01) — JWT how-to, nouveau service, parcours métier. Concept JWT updated: [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].
- [[concepts/architecture-coherence-across-services]] — August 2026 four-service comparison (scaffold shared, JWT/logging/errors/OpenAPI diverge).
- [[projects/aiforall/concepts/rustycog-git-submodule]] — rustycog is a pinned gitlink, not a vendored tree.
- [[projects/aiforall/skills/running-parallel-sonar-lanes]] — file-disjoint Sonar agent lanes.
- [[projects/aiforall/references/cursor-history-2026-04-to-08]] — distilled Cursor sessions since the April wiki wave.
- [[projects/rustycog/rustycog]] — hub restored (crate reference pages still missing on disk).
- [[journal/2026-08-31]] — catch-up ingest note.
- [[projects/rustycog/references/rustycog-outbox]] — transactional outbox bridge with Mermaid flow and sequence diagrams for DB-backed event durability.
- [[projects/aiforall/roadmap]] — near-term focus on Sentinel Sync tests, DB transaction-load verification, and the RustyCog Events outbox pattern.
- [[projects/rustycog/references/rustycog-events]] — SQS fanout now uses per-event destination queues and multi-queue consumer polling in RustyCog.
- [[projects/aiforall/skills/running-aiforall-runtime-modes]] — operational workflow for microservice and `oodhive-monolith` runtime modes.
- [[projects/aiforall/references/modular-monolith-runtime]] — dual runtime mode for AIForAll: standalone microservices plus the `oodhive-monolith` modular monolith.
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]] — real OpenFGA testcontainer fixture, random-port config contract, and migration notes from the old wiremock fake.
