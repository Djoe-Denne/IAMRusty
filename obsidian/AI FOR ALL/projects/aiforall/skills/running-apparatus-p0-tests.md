---
title: "Lancer les tests Apparatus P0"
category: skills
tags: [testing, rust, components]
sources:
  - apparatus-contracts/Cargo.toml
  - apparatus-reference-kv/Cargo.toml
  - apparatus-contracts/tests/contracts_p0.rs
  - apparatus-reference-kv/tests/kv_p0.rs
summary: >-
  Tests P0/P0.1 déterministes : feature test-harness, 42 tests, CI apparatus-p0, clippy et doc sans warning.
provenance:
  extracted: 0.92
  inferred: 0.08
  ambiguous: 0.00
created: 2026-09-11T05:45:00Z
updated: 2026-09-12T09:30:00Z
---

# Lancer les tests Apparatus P0

Les crates [[projects/manifesto/concepts/apparatus-p0-contracts]] n’ont pas besoin de Docker, WireMock, JWT ni OpenFGA.

## Commandes

```text
cargo test -p apparatus-contracts -p apparatus-reference-kv --features test-harness
cargo test -p apparatus-contracts -p apparatus-reference-kv
cargo clippy -p apparatus-contracts -p apparatus-reference-kv --all-targets -- -D warnings
cargo clippy -p apparatus-contracts -p apparatus-reference-kv --all-targets --features test-harness -- -D warnings
cargo doc -p apparatus-contracts -p apparatus-reference-kv --no-deps
cargo fmt --all -- --check
```

Sans `--features test-harness`, le module `harness` n’est pas compilé ; les tests gatés sont exclus (33 au lieu de 37).

P0.1 ajoute `apparatus_p01_micro.rs` (3 contrats + 2 KV) : total **42/42**. La CI exécute les deux crates (job `apparatus-p0`).

## Interdit

- `unwrap` / `expect` / `unsafe` dans `src/` (Mutex : `PoisonError::into_inner`).
- Introduire `axum`, `sea-orm`, `jsonwebtoken`, `openfga`, `reqwest`, `tokio` dans ces crates.
- Absorber `apparatus-events/` (hors `members`) ou toucher le submodule `rustycog/`.

## Related

- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/aiforall/skills/fixing-sonar-clippy-in-services]]
- [[projects/aiforall/skills/running-apparatus-p1-tests]]
