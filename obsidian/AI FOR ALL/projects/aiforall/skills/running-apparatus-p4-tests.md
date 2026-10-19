---
title: Lancer les tests Apparatus P4 (operator)
category: skills
tags: [testing, rust, kubernetes]
sources:
  - _raw/opencode-m6-proof-hardening-20261018/inventory.json
  - .agents/skills/aiforall-test-evidence/SKILL.md
  - _raw/opencode-g17-m6-20261018/inventory.json
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
  - workers/apparatus-operator/Cargo.toml
  - workers/apparatus-operator/tests/fixtures/kind/mod.rs
summary: >-
  Windows E:, IT9+1 et m6 réel1 avec H1–H4 ; préflight transitif des remplacements, captures sans secrets et limites d'attestation réseau.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-22T06:55:00Z
updated: 2026-10-18
---

# Lancer les tests Apparatus P4 (operator)

> [!warning] Actualisation — 18 octobre 2026
> Les commandes Kind historiques ci-dessous ne sont plus un runbook autorisé. `kind-apparatus-p4-it` est protégé ; les IT passent par testcontainers, jamais Kind. Seul l'E2E m6 explicitement autorisé a été exécuté sur `kind-aiforall-local`. Fixture désormais réutilisation/fail-loud, jamais create/delete/install Calico. Voir [[projects/aiforall/references/mission-g17-m6-20261018]].

## Preuves récentes et procédure actuelle

### Campagne de fiabilisation terminée le 18 octobre 2026
- [[projects/aiforall/references/mission-m6-proof-hardening-20261018]] : codefd9c412,
  oracles10+11, ITt6 9/9+m1 1/1, m6réel1/1 en51.07s surE:, zéro skip.
- H1–H4 autonomes archivés et14empreintes vérifiées. Attentes BusyBox404 et GNU
  timeout fermées par assertions runtime réelles ; pas certification DNS/mesh/Netpol globale.
- **Fixtureensure non destructif ≠ scénarioentier non destructif** : m6 remplace
  Podplugin/AdmissionRecord déterministes, comportement local accepté après
  explication (O-LIVE-1), paspréservationglobale ni droitproduction.
- Charger skillprojet `aiforall-test-evidence` + lifecycle avant futur préflight.
  Kubeconfigdédié, registrestrictUTF8/CP1252/UTF16, IDs/UIDavantaprès, allow-list
  d'archive sanscredentials ; pas de reprise automatique des scripts `.tmp`.
- StrictClippym6prod rouge différé explicitement ; verts unit/IT/E2E ne l'effacent pas.

### Photographie G-17 antérieure — ne pas confondre les deux runs

- Hôte Windows, un seul Cargo ; target E: exigé pour les prochains runs, env-only sans éditer la config. Le run historique m6 de cette mission a utilisé C: (écart documenté).
- G-17 t6 et m1 : `--features admit`, `--locked`, `--test-threads=1`, vrais binaires ORAS/Cosign ou images pinées ; 9/9 et 1/1.
- m6 : `cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --test-threads=1` ; 1/1, 311,79 s, kubeconfig exporté pour local.
- Préconditions : GO de la mission, lifecycle chargé, Calico Ready et pas kindnet, registre contemporain, inventaire avant/après ; cluster absent/non conforme = STOP, pas de reconstruction.
- Captures autonomes par hop manquantes dans cette preuve : tout prochain run doit réellement les produire sans secret et les archiver ; durcir readiness/endpoints du canary AVANT la sonde.

## Historique septembre (preuves passées, pas des commandes à rejouer)

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
