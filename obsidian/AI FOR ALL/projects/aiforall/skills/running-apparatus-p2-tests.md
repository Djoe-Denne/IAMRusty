---
title: >-
  Lancer les tests Apparatus P2
category: skills
tags: [testing, rust, components]
sources:
  - docs/apparatus-p2-implementation-prompt.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - Manifesto/tests/apparatus_p2_t1_events.rs
  - Manifesto/tests/apparatus_p2_t1_lookup.rs
summary: >-
  Tests P2 T1–T7 : unit sans Docker + DB testcontainer --test-threads=1. ADR-0006 Accepted, Réalité Partial.
provenance:
  extracted: 0.92
  inferred: 0.08
  ambiguous: 0.00
created: 2026-09-12T13:20:00Z
updated: 2026-09-12T13:20:00Z
---

# Lancer les tests Apparatus P2

ADR-0006 Accepted. Harness unique `Manifesto/tests/common.rs`. Concept : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

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
