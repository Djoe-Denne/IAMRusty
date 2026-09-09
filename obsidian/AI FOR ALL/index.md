---
title: Wiki Index
category: navigation
tags: [index, navigation, wiki]
summary: "Index du wiki AIForAll : services actuels, corrections AuthZ et dossier Apparatus comme fonctionnalité future."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
updated: 2026-09-09T17:50:00Z
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
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

## Fonctionnalité future — Apparatus

- [[projects/manifesto/concepts/apparatus-platform]] — vision et décisions proposées, distinctes de l’existant.
- [[projects/manifesto/references/apparatus-source-reconciliation]] — audit du dépôt et écarts avec le document utilisateur.
- [[projects/manifesto/references/apparatus-implementation-plan]] — migration, étapes et critères d’acceptation.

## Recent Additions

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
