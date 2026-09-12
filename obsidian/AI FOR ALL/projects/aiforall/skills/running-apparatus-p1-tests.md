---
title: >-
  Lancer les tests Apparatus P1
category: skills
tags: [testing, rust, components]
sources:
  - docs/services/manifesto.md
  - docs/apparatus-p1-implementation-prompt.md
  - Manifesto/tests/apparatus_p1_t1_migration.rs
  - Manifesto/tests/apparatus_p1_t7_gate.rs
summary: >-
  Tests P1 : unit/contract sans infra, DB/AuthZ/HTTP sur testcontainers avec --test-threads=1, gate T7 anti-P2.
provenance:
  extracted: 0.90
  inferred: 0.10
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# Lancer les tests Apparatus P1

Conventions : harness unique `Manifesto/tests/common.rs`, `#[serial]`, `--test-threads=1` pour tout test live, `cache_ttl_seconds = 0`. Concept : [[projects/manifesto/concepts/apparatus-p1-persistence]].

## Commandes

```text
cargo fmt --all -- --check
cargo check -p manifesto-migration
cargo check -p manifesto-infra
cargo test -p manifesto-infra apparatus_mapping
cargo test -p manifesto-service --test apparatus_p1_t1_migration -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t2_backfill -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t3_collisions -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t4_outbox -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t5_acl -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t6_http -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t7_gate -- --test-threads=1
```

- Mapping seul (`apparatus_mapping`, 5/5) : sans infra.
- T1-T4 : PostgreSQL testcontainer réel. T5 : OpenFGA réel. T6 : serveur live + wiremock typée (`MockService` + `reset()`, premier-match-gagne).
- Migrations : `cargo run -p manifesto-migration -- up` / `down`.

## Interdit

- Nouveau harness parallèle ou nouveau `#[path fixtures]` ad hoc.
- Nouveau type FGA pour le binding (escalader au lieu d'inventer).
- Tout token P2 (polling, worker, lease, fencing, controller, Factory, gateway, host, workload) — le gate T7 échoue sinon.

## Related

- [[projects/aiforall/skills/running-apparatus-p0-tests]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
