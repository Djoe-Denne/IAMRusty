---
title: >-
  Wiki Index
category: navigation
tags: [index, navigation, wiki]
summary: >-
  Index wiki AIForAll au 27 sept. 2026 : P4 Kind V1 Implemented, IAM-IdP
  0407–0411 Implemented, JWT 0304 Partial, gold path 0605 livré en réalité.
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
updated: 2026-09-27T09:20:00Z
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - docs/adr/README.md
  - docs/apparatus-p0-implementation-prompt.md
  - docs/apparatus-p1-implementation-prompt.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/adr/0007-closeout.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/apparatus-p4-implementation-prompt.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
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
- [[projects/lazaret/lazaret]] — P3 capability/data boundary (ADR-0007 Implemented T1–T14b)
- [[projects/rustycog/rustycog]] — shared Rust SDK (crate map: [[projects/rustycog/references/index]])
- [[projects/sentinel-sync/sentinel-sync]] — centralized OpenFGA authorization and the sync worker

## Architecture — ADR

- [[projects/aiforall/decisions/index]] — vague 2 rétroactive (0100–0502) plus JWT 0304–0309 et cloud 0600–0605.
- [[projects/manifesto/decisions/index]] — Apparatus 0001–0008 Accepted ; 0002–0008 Implemented sauf 0001 Partial ; gates 0009–0011 Proposed.
- Atlas visuel du dépôt (Mermaid, canon code) : `docs/architecture/README.md` — le wiki reste la couche conception.

## Fonctionnalité — Apparatus

- [[projects/manifesto/concepts/apparatus-p0-contracts]] — crates P0 livrées + audit + P0.1 42/42.
- [[projects/manifesto/concepts/apparatus-p1-persistence]] — persistance P1 T1–T7 (commit `7455ee5`).
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]] — ticker in-process **Implemented**.
- [[projects/lazaret/lazaret]] — frontière P3 Implemented (T1–T14b ; A-DEC 2026-09-20).
- [[projects/aiforall/concepts/https-platform-mesh]] — T14b HTTPS IAM/Telegraph/Hive/Manifesto, dual-bind, CA mesh ≠ CA Lazaret.
- [[projects/manifesto/concepts/apparatus-platform]] — vision ; Factory/host encore absents. Alias [[entities/paravretius]]. Moteur P4 : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (Accepted / Implemented). Gates : [[projects/manifesto/decisions/0009-0011-gates-preprod]]. Crate : [[projects/manifesto/concepts/apparatus-p4-operator]].
- [[projects/manifesto/references/apparatus-implementation-plan]] — phases et preuves P0–P3.

## Recent Additions

- [[journal/2026-09-30]] — `trust_envoy` + e2e Compose 48 OK ; rien commité AIForAll.
- [[projects/aiforall/decisions/0308-mesh-authn-jwt]] — 0308 Accepted / Partial, mode passerelle §7.
- [[projects/aiforall/concepts/mesh-gateway-principal-trust]] — rustycog `trusted_gateway_san`, pin `2290d45`.
- [[projects/aiforall/skills/running-mesh-authn-e2e]] — `hivemigration`, FORCE, `build-artifacts` d’abord.
- [[projects/aiforall/references/cursor-chat-mesh-authn-2026-09-30]] — distillat du chat `23daa0aa`.
- [[journal/2026-09-29]] — e2e Envoy 0308 : 403 sans JWT, iss/sub recréés, `x-principal-foo` encore transmis.
- [[projects/aiforall/concepts/mesh-ext-authz-opt-in]] — ext_authz opt-in, Accepted / Partial ; Compose §7 prouvé le 30 sept.
- [[journal/2026-09-27]] — delta 22–27 sept. : Connect IdP, gold path, JWT 0304–0309.
- [[projects/aiforall/decisions/0304-access-jwt-trust]] — RS256 / JWKS. 0304 Partial, 0307 Implemented, 0308 et 0309 Partial.
- [[projects/aiforall/decisions/0605-gold-path-kind]] — HTTP 200 Kind, Proposed / Réalité Implemented.
- [[projects/manifesto/decisions/0009-0011-gates-preprod]] — scale, CA, pod par binding : Proposed / Unimplemented.
- [[journal/2026-09-22]] — P4-core T2–T12 ; A-DEC 0008 Implemented.
- [[projects/manifesto/concepts/apparatus-p4-operator]] — crate `apparatus-operator`.
- [[projects/aiforall/skills/running-apparatus-p4-tests]] — Docker + Kind `--test-threads=1`.
- [[projects/iamrusty/decisions/index]] — 0407–0411 Accepted / Implemented (crates Connect en HEAD).
- [[projects/aiforall/decisions/0602-observabilite-portable]] — OTLP Proposed, pas implémenté.
- [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] — ADR-0008 Accepted / Implemented (Kind V1).
- [[projects/manifesto/references/0007-closeout]] — A-DEC inventaire ; APP-05 reste ouvert.
- [[projects/manifesto/references/0008-app01-reconciliation]] — méthode RATIFIÉE, pas canon.
- [[projects/manifesto/references/apparatus-p4-implementation-prompt]] — contrat TDD P4 historique ; 0008 depuis Implemented (A-DEC).
- [[journal/2026-09-20]] — HEAD `a27ea5b` : T11b–T14b + SQS ; ingest 0008 / closeout.
- [[projects/aiforall/concepts/https-platform-mesh]] — mesh HTTPS T14b (IAM 8443, Telegraph 8444, Hive 8445, Manifesto 8448).
- [[journal/2026-09-18]] — HEAD `9e85edd` : T8 `kv_purge` `f0cf1d2`, T10 enrollment `9e85edd`, flake CI SQS LocalStack.
- [[entities/paravretius]] — nom historique ; canon Apparatus / Lazaret ; absent du code.
- [[projects/lazaret/lazaret]] — P3 Partial T1–T10 (T8 file `lazaret-kv-events`, T10 `apparatus_enrollments`).
- [[journal/2026-09-17]] — HEAD `e978cd0` : P2 Implemented, Architecte, Lazaret P3 Partial.
- [[projects/aiforall/concepts/architecte-agent]] — ADR Proposed + contrat, pas 0100–0502.
- [[journal/2026-09-13]] — P2 alors Partial dans HEAD `7455ee5`.
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
