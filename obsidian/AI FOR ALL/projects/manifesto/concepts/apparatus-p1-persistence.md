---
title: >-
  Apparatus P1 — persistance et catalogue Manifesto
category: concepts
tags: [components, architecture, testing, visibility/internal]
sources:
  - docs/apparatus-p1-implementation-prompt.md
  - docs/adr/0001-apparatus-binding-owned-by-manifesto.md
  - docs/services/manifesto.md
  - Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs
  - Manifesto/infra/src/apparatus_backfill.rs
  - Manifesto/infra/src/apparatus_mapping.rs
  - Manifesto/infra/src/apparatus_outbox.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/f5e5a1e2-20b9-4794-95b0-c75b32a52fb4/f5e5a1e2-20b9-4794-95b0-c75b32a52fb4.jsonl
summary: >-
  P1 livré 2026-09-12 : extension 1:1 apparatus_bindings, backfill legacy, T1-T7 verts 36 tests, zéro nouveau type FGA.
provenance:
  extracted: 0.85
  inferred: 0.13
  ambiguous: 0.02
created: 2026-09-12T09:30:00Z
updated: 2026-09-13T10:25:00Z
---

# Apparatus P1 — persistance et catalogue Manifesto

P1 implémente la persistance du binding [[projects/manifesto/decisions/index|ADR-0001]] en TDD strict (tranches T1-T7), sans workload, sans gateway, sans Factory. Commit `7455ee5` (working tree propre le 13 sept.). Suite : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

## Extension 1:1

- Table `apparatus_bindings` : `id` BIGSERIAL interne, `component_id` UUID UNIQUE FK → `project_components.id` CASCADE, `digest` VARCHAR(128) NULL, `source` CHECK `legacy|managed`.
- Migration `m20260912_000012` additive réversible (up/down/up verts).
- `component_type` non réécrit ; mapping legacy → Apparatus **injectif** (`check_pairs_injective`, erreur versionnée `APPARATUS_MAPPING_COLLISION` v1, conflit 23505 → 409).
- Fichiers : `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs`, `Manifesto/infra/src/apparatus_backfill.rs`, `apparatus_mapping.rs`, `apparatus_outbox.rs`, `transaction.rs`, `Manifesto/http/src/handlers/components.rs`.

## Tranches et preuves

| Tranche | Contenu | Tests |
|---|---|---|
| T1 migration | Table 1:1, up/down/up, unicité intacte | 8/8 (`apparatus_p1_t1_migration`) |
| T2 backfill | `backfill_apparatus_legacy` explicite idempotent, `source=legacy`, release non résolue, contrôleur off | 5/5 |
| T3 collisions | Mapping injectif, double ajout concurrent refusé par contrainte DB, concurrence [201,409] stable | 4/4 + mapping 5/5 |
| T4 outbox | `persist_binding_atomically` : échec → rollback total (0 ligne), succès → 1 ligne managed ; faux broker in-memory, 0 polling/worker | 3/3 |
| T5 ACL | Binding managed commité avec composant+ACL+outbox, grants inchangés, revoke→403 TTL0, **zéro nouveau type FGA** (FGA 5 types) | 4/4 |
| T6 HTTP | Alias `?binding` = même `component_id` (404 sinon), contrats sérialisés gelés, catalogue wiremock, 5 routes `/components` | 4/4 |
| T7 gate | 0 token P2 dans le prod scanné (7 crates src), 0 `VALID`/`VERIFIED`, routes/FGA/UUID intacts | 3/3 |

Total : **P1 36** (T1-T6 28 + mapping 5 + T7 3) + **P0.1 42** = **78**. Voir [[projects/aiforall/skills/running-apparatus-p1-tests]].

## Conventions figées (résolution unique P1)

- Harness unique `Manifesto/tests/common.rs`, `#[serial]`, `cache_ttl_seconds = 0`, `port = 0` ; pas de harness parallèle.
- OpenFGA réel en testcontainer pour T5 ; arranger `project` uniquement, ne jamais écrire `component:{id}` ad hoc.
- Consentement et génération **non ajoutés** (sans spec ADR) ; ADR dédiée génération/lease/fencing exigée avant P2.
- Méthode d'exécution : duel gather (agent + contre-agent isolés) → résolution unique → socle TDD RED → tranches → reviews docs. Voir [[projects/aiforall/concepts/orchestrator-agent-harness]].

## Related

- [[projects/manifesto/references/apparatus-implementation-plan]] — preuve P1 détaillée
- [[projects/manifesto/concepts/apparatus-p0-contracts]] — socle P0/P0.1
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]] — desired/observed (P2)
- [[projects/aiforall/skills/running-apparatus-p1-tests]]
