---
title: >-
  Mission ops — séparation runtime/validation du 19 octobre 2026
category: references
tags: [testing, docker, kubernetes, orchestration, visibility/internal]
sources:
  - docs/missions/20261018-ops-kind-runtime-validation-separation.md
  - .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md
  - docs/contracts/ops-kind-runtime-validation-separation.md
  - docs/contracts/ops-kind-runtime-validation-manifest.md
  - .cursor/qa-plans/20261019-ops-kind-runtime-validation-qa-plan.md
  - .cursor/review-briefings/20261019T0956Z-correctness-ops-kind-runtime-validation-separation-5c5d6ca.md
  - .cursor/review-briefings/20261019T1007Z-tests-ops-kind-runtime-validation-5c5d6ca.md
summary: >-
  Mission Ensemble 10 workers close : contrat+manifeste 150 fichiers, 3 lots I intégrés, IT 5/5 packages, revues BLOCK corrigées ; flakes fixture documentés, dette churn ratifiée.
provenance:
  extracted: 0.9
  inferred: 0.1
  ambiguous: 0.0
created: 2026-10-19
updated: 2026-10-19
---

# Mission ops — séparation runtime/validation (19 octobre 2026)

Exécution complète de l'ordre
`docs/missions/20261018-ops-kind-runtime-validation-separation.md` par une équipe
Ensemble de 10 workers sous grille de routage approuvée (rev 1 : luna medium/high
light, glm-5.3-flash max standard, gpt-6.1-sol high strong, xhigh maximum, grok
4.7 alternatif non automatique). Concept distillé :
[[projects/aiforall/concepts/runtime-validation-separation]].

## Chaîne de commits (base 80918ca → 40e759d)

| Commit | Contenu |
|---|---|
| `427416b` | Contrat Proposed + manifeste de tri (150 fichiers) + plan QA |
| `3aee24b` | Intégration atomique I-runtime + I-validation (15 fichiers) |
| `2625171` | Salvage local-full README + labels H4 + amendement manifeste |
| `5c5d6ca` | I-références : justfile pinné, archives, retrait .pyc |
| `eac9be9` | Fix-batch post-revues (C-1/C-2, T-1..T-5) |
| `40e759d` | Amendements plan QA + flips COVERED |

## Phases et gouvernance

- **Audits RO A/B/C** (explore ×3, sans worktree, dépôt canonique partagé) :
  graphe E1–E7, 16 drivers cartographiés, consommateurs inverses des 150 fichiers.
- **D contrat** (architecte, gpt-6.1-sol high) : écriture bloquée par les
  permissions scoped du profil × worktree hors racine (globs non matchés →
  deny `*`) — preuve JSON conservée ; livraison en 7 chunks ≤10KB, intégration
  lead (auteur documenté). Vérifications lead : couverture 150/150 mécanique,
  11 chemins du checker, contradictions justfile:199/ADR0606:26.
- **Q plan QA** (qa-planner, glm max) : 18 risques FMEA, registre ~28
  obligations COVERED/GAP, 6 gates statiques + IT natives.
- **Arbitrages utilisateur** : T1 migré (fixtures IAMRusty), .pyc retirés,
  archives H1–H4 — puis variation restrictive (enroll-job.yaml conservé :
  consommateur vivant `schedule-reference-kv.ps1:423` trouvé par pool-10,
  manqué par l'audit C — exemple vécu de « aucune déduction inutilisé »).
- **3 lots I en parallèle** (2 max simultanés + I-références après intégration) ;
  spawns parasites auto-scale fermés (pool-8/9) ; doublon pool-8 non mergé
  (salvage d'un seul fichier).

## Validation

- **IT natives 5/5 packages** : iam 290/0/1 opt-in (24m53) · hive exit 0 ·
  manifesto 12/12 après flake (RUST_TEST_THREADS=2) · telegraph exit 0 ·
  lazaret 2/2 après flake. Détail :
  [[projects/aiforall/concepts/testcontainers-resource-churn]].
- **model.json régénéré** (CLI pinnée `31a565d1…`) accepté par de vrais
  serveurs OpenFGA — neutralité sémantique confirmée (reviewer : rewrites/
  restrictions inchangés + IT p1–p6 verts).
- **Statiques** : checker 15 chemins, S-11 18/18, source-guard 22/22,
  infra-static 8 rendus (+ fix BOM UTF-8/PYTHONUTF8), fmt, py_compile,
  gitignore actif.

## Revues (BLOCK → corrigé)

- correctness : C-1 HIGH (matrice curl attendait 303 mais full a
  `connectors=[]` → handler 422) ; C-2 MEDIUM (positifs authentifiés
  inexpressibles). Arbitrage lead : canal e2e `--inputs` pour l'authentifié,
  route-matrix rapporte NOT RUN authentifié, jamais PASS implicite.
- test-reviewer : T-1 HIGH (matrice livrée sans aucun test, oracle
  tautologique) ; T-3/T-4 gardes manquantes ; T-5 gardes statiques. Fix-batch
  audité PASS (mutations mortelles démontrées, baseline AST intacte) ;
  résidus LOW nommés (T-6, T-7 traité).

## Déferts nommés

T-6 (garde rendu-vs-source), T-5 restants (G-A8a/A9a/B4/B5/B6/C5/C6), IT SQS
opt-in NOT RUN, live full NOT RUN jusqu'au GO runtime séparé, E2E authentifié
BLOCKED sans setup approuvé.
