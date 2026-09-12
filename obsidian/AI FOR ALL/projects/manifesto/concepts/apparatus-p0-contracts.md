---
title: "Apparatus — contrats P0 livrés"
category: concepts
tags: [components, architecture, testing, visibility/internal]
aliases: [apparatus-contracts, apparatus-reference-kv]
status: partial
feature_status: partial
sources:
  - docs/adr/0002-apparatus-contract-first.md
  - docs/apparatus-p0-implementation-prompt.md
  - apparatus-contracts/src/lib.rs
  - apparatus-reference-kv/apparatus.toml
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/442685dd-30ba-4a9d-b59b-f8ae6aa47b9a/442685dd-30ba-4a9d-b59b-f8ae6aa47b9a.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/688f5240-08ac-4f6e-94ce-323953bc759a/688f5240-08ac-4f6e-94ce-323953bc759a.jsonl
  - C:/Users/djden/.codex/sessions/2026/09/11/rollout-2026-09-11T13-40-54-01a09045-472c-7390-abf3-435e5a6e4c12.jsonl
  - .github/workflows/ci.yml
summary: >-
  P0 livré + audité : crates pures, digest sha256, harness gaté, KV référence, P0.1 42/42 + CI.
provenance:
  extracted: 0.78
  inferred: 0.20
  ambiguous: 0.02
created: 2026-09-11T05:45:00Z
updated: 2026-09-12T09:30:00Z
---

# Apparatus — contrats P0 livrés

Le jalon P0 d’[[projects/manifesto/references/apparatus-implementation-plan]] existe dans le workspace : deux crates top-level, **sans** Axum, SeaORM, OpenFGA, JWT, AWS ni `rustycog-*`. ADR cible : [[projects/manifesto/decisions/index]] (surtout [0002](../../../../../docs/adr/0002-apparatus-contract-first.md)).

## Ce qui est dans le dépôt

- `apparatus-contracts` : `limits` (`MAX_ID_LEN=100`, `MAX_DESCRIPTION_LEN=1024`), `ids`, `manifest` (`deny_unknown_fields`), `capabilities` (`project.read`, `storage.kv.read`, `storage.kv.write`), `ui` (`absent|schema|sandbox`), `protocol` (`manifesto-apparatus/1`), `validation`, `digest`, `error`, port `KvStore` (`src/ports.rs`).
- Digest : `sha256:` + hex manuel sur JSON canonique (`BTreeMap`, `serde_json` sans espaces). `latest` / refs flottantes / `trusted_*` / `credentials` sont des erreurs de validation.
- `apparatus-reference-kv` : `apparatus_id` `io.aiforall.reference-kv`, backend `kv-v1` injecté, UI `schema`, pas de réseau. Idempotence par `operation_id`. `deterministic_operation_id` est FNV-1a, sans RNG.
- Harness in-process **TEST-ONLY** : feature Cargo `test-harness`. Absent des builds prod. Isolation KV par `binding_id`. Ne produit pas `VALID` ni `VERIFIED`.

## Tests

- `apparatus-contracts/tests/contracts_p0.rs` : 27 tests (manifeste, digest, rejets, bornes, UI, DTO `deny_unknown_fields`, harness gaté).
- `apparatus-reference-kv/tests/kv_p0.rs` : 10 tests (idempotence, isolation, delete, bornes KV, configure/invoke inconnus).
- Commande : `cargo test -p apparatus-contracts -p apparatus-reference-kv --features test-harness` (37). Sans feature : 33 (tests harness exclus).
- P0.1 : + `apparatus_p01_micro.rs` (3 contrats + 2 KV) = **42/42** ; CI exécute les deux crates (job `apparatus-p0`).
- Déterministes, sans Docker. `list_keys` n’est pas sur le port `KvStore` ; non testé au niveau référence. ^[inferred]

## Revue et correctifs (10 sept.)

Triple review (deux Grok Extra High, un Muse) : **accepté sous réserves**. Bloquant retenu : harness compilé en prod — corrigé par la feature. Autres : `unbind` validé, `KvStore` déplacé en port, `serde`/`uuid` retirés de `reference-kv`. ^[inferred]

## Audit P0 et fermeture P0.1 (11–12 sept.)

Double audit lecture seule (Cursor `688f5240` + Codex `01a09045`, même verdict) : **P0 terminé avec réserves non bloquantes**. Preuves : 37/37 tests, `fmt`/`check`/`clippy -D warnings`/`doc` verts, 0 `VALID`/`VERIFIED`, 0 `unwrap`/`expect`/`unsafe`, hors-périmètre respecté. Réserves : compteurs `16/16` obsolètes, ADR/README désynchronisés, P0 hors matrice CI.

P0.1 (12 sept., non commité) : compteurs réels 30+12 = **42/42** (`apparatus_p01_micro.rs` 3+2), job CI `apparatus-p0` + coverage Apparatus, 3 non-bugs justifiés par écrit (dont `ReadyResponse`, `configure` persistant, adaptateurs KV).

## Hors P0

Factory, gateway mTLS, host UI, persistance Manifesto, `VALID`/`VERIFIED` persistants. Le monolithe ne charge pas ces crates comme plugin. Voir [[projects/manifesto/concepts/apparatus-platform]].

## Related

- [[projects/manifesto/references/apparatus-ui-and-protocol]] — DTO discovery/health/bind existent ; pas de transport HTTP.
- [[projects/aiforall/skills/running-apparatus-p0-tests]]
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
- [[projects/manifesto/concepts/apparatus-p1-persistence]] — persistance Manifesto au-dessus de ces contrats
