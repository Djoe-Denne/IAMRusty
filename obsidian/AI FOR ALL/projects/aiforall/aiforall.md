---
title: >-
  AIForAll
category: project
tags: [platform, microservices, rust, visibility/internal]
sources:
  - docs/missions/20261031-k6-auth-handover.md
  - .opencode/missions/team_mvtz6p47_000q_4wmht0/reports/99-cas-minimal-windows-verifie.md
  - .opencode/missions/team_mvtz6p47_000q_4wmht0/reports/107-nettoyage-et-archivage-verifies.md
  - docs/contracts/20261029-auth-security-qualified-state.md
  - docs/missions/20261029-k6-recherche-et-enchainement.md
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - _raw/opencode-m6-proof-hardening-20261018/inventory.json
  - _raw/opencode-g17-m6-20261018/inventory.json
  - README.md
  - Cargo.toml
  - runtime/monolith/Cargo.toml
  - runtime/monolith/src/runtime.rs
  - docs/reviews/iam-architecture-comparison.md
  - .agents/skills/rustycog-submodule/SKILL.md
  - C:/Users/djden/source/repos/AIForAll/.env
  - docs/adr/README.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/adr/0007-closeout.md
summary: >-
  Plateforme RustyCog : OAuth historique qualifié et socle k6 Windows PASS ; portage auth intégré mais non exécuté, mission arrêtée. Linux différé.
provenance:
  extracted: 0.87
  inferred: 0.10
  ambiguous: 0.03
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-10-31
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
- Sous OpenCode, la racine travaille directement ou devient lead Ensemble, sans orchestrateur intermédiaire ; l'historique Cursor reste dans [[projects/aiforall/concepts/orchestrator-agent-harness]]. Architecture vivante : [[projects/aiforall/concepts/architecte-agent]].
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

- [[projects/aiforall/references/auth-security-closure-20261029]] — correctif d'autorité OAuth, Chrome réel et refus Envoy qualifiés localement ; limites production/HA/HSM conservées.
- [[projects/aiforall/concepts/k6-functional-linux]] — socle k6/Chrome Windows PASS ; portage auth intégré et inspecté, mais aucune campagne auth exécutée. Arrêt utilisateur après échec de déploiement de cause inconnue ; trois nœuds arrêtés, données et arbres conservés. Source : `docs/missions/20261031-k6-auth-handover.md`. Linux reste différé.
- Clôture du socle k6 mission1 vérifiée : dix copies de travail retirées après sauvegardes, équipe archivée et onze références de sauvegarde conservées. Ce bilan ne décrit pas la mission auth arrêtée avec ses arbres conservés. Le nettoyage n'est pas une qualification HTTP/Linux ; enseignements dans [[projects/aiforall/concepts/orchestrator-agent-harness]].

- [[projects/aiforall/references/mission-telegraph-outbox-20261027]] — transaction writer commune, NoOp suspendu, owner HTTP, reprise après lease puis verticaleSQS/OpenFGA/replay/403/200 ; commit39024b2 et SDKd60 publiés avec gitlink exact. Clippy antérieur/Sonar/CI restent des réserves, statuts ADR non promus.
- [[projects/telegraph/concepts/durable-notification-publication]] et [[projects/aiforall/skills/proving-leased-outbox-recovery]] — contrat livré et protocole de preuve distinct du SLA produit.

- [[projects/aiforall/references/mission-m6-proof-hardening-20261018]] — suite exécutée :21oracles, IT9+1 et m6réel1 surE:, H1–H4 archivés, revuesPASS bornés ; remplacementsK8slocaux acceptés et pièges promus en skill/règles, detteClippy conservée.

- [[projects/aiforall/references/mission-g17-m6-20261018]] — photographie antérieure : G-17/m6 verts avec commentaires,19sessions archivées ; fiabilisation alors préparée, exécutée dans la référence suivante ci-dessus.

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
