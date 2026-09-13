---
title: >-
  Lancer les tests Apparatus P2
category: skills
tags: [testing, rust, components]
sources:
  - docs/apparatus-p2-implementation-prompt.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - Manifesto/tests/apparatus_p2_t2_migration.rs
  - Manifesto/tests/apparatus_p2_t5_tick.rs
  - Manifesto/tests/apparatus_p2_t7_cleanup.rs
summary: >-
  Tests P2 T1–T7 : unit sans Docker + DB --test-threads=1. Compteurs cités :
  t2 12, t4 4, t5 5, t7_cleanup 2. ADR-0006 Accepted / Partial.
provenance:
  extracted: 0.92
  inferred: 0.08
  ambiguous: 0.00
created: 2026-09-12T13:20:00Z
updated: 2026-09-13T10:25:00Z
---

# Lancer les tests Apparatus P2

ADR-0006 Accepted, Réalité Partial. Harness `Manifesto/tests/common.rs`. Concept : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

T4/T5 **injectent `digest` en SQL** après le create HTTP : le chemin prod ne pose pas le digest, donc un create seul ne suffit pas à exercer `bind`. ^[extracted]

## Commandes

```text
cargo test -p manifesto-service --test apparatus_p2_t1_events -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t1_lookup -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t2_migration -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t3_persist -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t4_resume -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t5_tick -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t6_runtime -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t7_cleanup -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p2_t7_gate -- --test-threads=1
cargo test -p manifesto-service --test apparatus_p1_t7_gate -- --test-threads=1
```

`t1_events` / `t6_runtime` / gates : sans Docker. Les autres : Postgres (+ OpenFGA via `setup_test_server`), `#[serial]`.

## Compteurs cités (2026-09-12 / t5 poison 2026-09-13)

t1 4+1, t2 **12**, t3 5, t4 **4**, t5 **5**, t6 3, t7 cleanup **2** + gate 5. Suite Docker non relancée à l’ingest wiki. Clippy OK cité dans le jalon.
