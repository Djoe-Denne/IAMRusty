---
title: >-
  Séparation runtime/validation — le contrat ops Kind
category: concepts
tags: [architecture, ops, kubernetes, contracts, visibility/internal]
sources:
  - docs/contracts/ops-kind-runtime-validation-separation.md
  - docs/contracts/ops-kind-runtime-validation-manifest.md
  - .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md
summary: >-
  Un seul chemin nominal (local-full) sous bail opérateur, validations explicites séparées (smoke/curl/E2E/IT), manifeste de tri 150 fichiers à ownership unique.
provenance:
  extracted: 0.93
  inferred: 0.07
  ambiguous: 0.0
created: 2026-10-19
updated: 2026-10-19
---

# Séparation runtime/validation — contrat ops Kind

Décisions D1–D6 du contrat `docs/contracts/ops-kind-runtime-validation-separation.md`
(Proposed, lien secondaire vers l'ADR 0606 sans la consommer). Mission d'origine :
[[projects/aiforall/references/mission-ops-kind-separation-20261019]].

## D1 — Un nominal : local-full

Cluster `aiforall-local-full` (1 control-plane + 2 workers), namespaces dédiés.
Séquence : artefacts Linux (compose build **sans** up) → certificats → bail
opérateur neuf → bootstrap → images épinglées → déploiement → validation
séparée. Les 5 autres chemins (M1/M2/M3, J2 hybride, J3/gold, mesh, dette
schedule-reference-kv) restent **non nominaux avec cibles explicites** —
conservés, jamais rebaptisés full, jamais supprimés sans arbitrage.

## D6 — Le bail est produit par l'opérateur (premier maillon)

JSON v2 minimal : `version=2`, paire cluster/context **fermée**, `task_id`
(32-hex minuscule), `anchor_state=pending_creation`, `nodeContainerIds=[]`,
`kubeconfig_path` sous `approved_root/cluster-anchor-<task_id>/` — sur Windows
`Path.home()/AppData/Local/Temp/opencode` (`local_full_target.py:128–129`).
Aucun champ préfabriqué (UID, ID, mapping, hash, endpoint) : le bootstrap
produit les attestations, jamais l'inverse. Le writer leasev2 parent n'existe
PAS sous ops — la faille du premier maillon de l'audit A est traitée par la
procédure opérateur documentée au README runtime.

## Interface validation — quatre canaux séparés

- **Smoke** : readiness/get/wait uniquement, zéro mutation (pas de Pod, seed,
  set-env, rollout, NP).
- **Curl métier (route-matrix)** : accès contractualisé `127.0.0.1:18080`,
  positifs/négatifs discriminants, **matrice cohérente avec le profil déployé**
  (full : `connectors=[]` → login OAuth attendu **422** `connector_not_configured`,
  pas 303 ; voir `mesh-public-route-cases-local-full.json`) ; la couverture
  authentifiée est rapportée **NOT RUN**, jamais PASS implicite.
- **E2E** : état métier **déclaré** par `--inputs` privé, prepare-probes/
  cleanup-probes séparés (cleanup = seules ressources enregistrées, jamais par
  nom), BLOCKED explicite tant que le setup full approuvé est absent.
- **IT natives Windows** : `it-native.ps1 -Suite <svc>` → `cargo test -p <pkg>
  --tests`, un package à la fois, testcontainers, jamais Kind.

Code0 seulement si toutes les obligations sont exécutées ; skip ⇒ non-zéro
avec raison non secrète ; rapport LIVE/SIMULÉ/NOT RUN.

## Manifeste de tri — ownership unique

150 fichiers suivis, chaque chemin = enregistrement individuel (rôle,
producteur/consommateurs, dépendances, effets, preuve SOURCE jamais LIVE,
destination, décision, justification, owner). Catégories : KEEP-RUNTIME,
KEEP-VALIDATION, SHARED-CONTRACT (OpenFGA model.fga/json, copie unique dans
ops/), MOVE-TEST-INTERNAL, ARCHIVE-HISTORY (H1–H4 → `docs/history/ops/`, H4
archivé 2/3 : `enroll-job.yaml` conservé, consommateur vivant), REMOVE-GENERATED
(.pyc), OTHER-OPS/DEFER-UNKNOWN (« pas de consommateur » = hypothèse documentée,
jamais inutilisé ; ≥2 recherches indépendantes avant toute conclusion).
« Moins de fichiers » n'est pas un oracle ; **moins de couplages** en est un.

## Pièges persistants

- `offline-no-kubeconfig` inexistant **par design** — ne pas le créer.
- Marqueur racine `Cargo.toml` + `ops/deploy/kind/cluster.yaml` (le déplacer
  casse toutes les entrées).
- `infra-static.sh` exige `PYTHON` (Windows : shim WSL sur `bash` du PATH —
  Git Bash explicite) et `PYTHONUTF8=1` (BOM).
- Cycle de vie runtime : [[projects/aiforall/concepts/local-runtime-lifecycle]].
