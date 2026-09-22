---
title: Lancer les tests Apparatus P4 (operator)
category: skills
tags: [testing, rust, kubernetes]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
  - apparatus-operator/Cargo.toml
  - apparatus-operator/tests/fixtures/kind/mod.rs
summary: >-
  IT operator : features admit,controller obligatoires pour T6/T10/T11 ;
  --test-threads=1 ; Docker + Kind fail-loud. Kind Calico v3.29.7
  (disableDefaultCNI). T11 vert 22 sept. Cosign fail-closed. 0008 Implemented (A-DEC) ; Kind V1 ; dette TCB.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-22T06:55:00Z
updated: 2026-09-22T14:00:00Z
---

# Lancer les tests Apparatus P4 (operator)

Crate `apparatus-operator`. Canon : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]. Concept : [[projects/manifesto/concepts/apparatus-p4-operator]]. Harness : [[projects/aiforall/concepts/orchestrator-agent-harness]].

Fail-loud si Docker ou Kind absents. **Pas** `#[ignore]`. Kind n’est pas « Docker manquant » : LocalStack tourne déjà ; le binaire Kind doit être sur le PATH (fallback fixture `C:\Users\djden\bin\kind.exe` / `APPARATUS_KIND_BIN`).

## Commandes

M1 reference-kv pin zot (sans Kind) :

```text
cargo test -p apparatus-operator --test apparatus_m1_reference_kv_pin --features admit -- --nocapture --test-threads=1
```

Pin unitaire (sans Kind) :

```text
cargo test -p apparatus-operator --features admit,controller --test apparatus_p4_t6_sign_registry t6_cri_pin_rejects_short_or_non_hex -- --exact
```

IT Docker T6 + Kind T10/T11 (17 tests au 22 sept. 2026) :

```text
cargo test -p apparatus-operator --features controller,admit --test apparatus_p4_t6_sign_registry --test apparatus_p4_t10_platform_workload --test apparatus_p4_t11_runtime -- --test-threads=1
```

T12 (gates fichiers, pas Docker) : `--test apparatus_p4_t12_gate_regression`.

T2–T5 / T7–T9 : features selon `Cargo.toml` (`admit` pour T6/T9 ; `controller` pour T10/T11).

## Pièges

- `cargo test -p apparatus-operator` **sans** `--features admit,controller` n’exécute **pas** T6/T10/T11 (tests derrière `required-features` dans `Cargo.toml`) — ce n’est **pas** la preuve du lot.
- T11 exige un CNI qui enforce les NetworkPolicy : cluster Kind IT avec **Calico v3.29.7** (`disableDefaultCNI: true`), **pas** kindnet seul — T11 `t11_plugin_exec_cannot_reach_transit_or_system` **vert** au 22 sept. 2026. Cosign admission **fail-closed** (tag `.sig` ≠ preuve). ADR-0008 **Implemented** (A-DEC 2026-09-22) ; Kind = V1 ; dette **D-TRANSIT-TCB** / **D-ADMB** / **D-PROD** (Transit `-dev` IT ≠ exigence TCB prod).
- Cluster Kind **réutilisé** (`apparatus-p4-it`). Le miroir HTTP zot (`certs.d` / `hosts.toml`) est un patch **runtime**, pas `extraPortMappings`.
- T10 historique charge l’image via `kind load`. Le test pull zot **ne doit pas** appeler `build_and_load_platform_plugin`. Unicité : LABEL docker + `crictl inspecti` absent avant schedule.
- Push CRI vers zot HTTP : skopeo/crane (`--dest-tls-verify=false`), pas `insecure-registries` Docker Desktop. Fail-loud si les deux manquent.
- `cargo fmt --all` peut échouer sur des fichiers Manifesto hors lot (ex. `apparatus_p4_t1_absence.rs` untracked).
