---
title: >-
  Churn des fixtures testcontainers — mesure, flake prouvé, correction engagée
category: concepts
tags: [testing, docker, rust, debt, visibility/internal]
sources:
  - .serena/memories/architecture/it-testcontainers-resource-churn.md
  - .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md
  - .cursor/qa-plans/20261019-ops-kind-runtime-validation-qa-plan.md
  - docs/missions/20261019-it-fixtures-lifecycle.md
summary: >-
  Un cargo test IT spawn des dizaines de containers simultanés (35–97 mesurés), flake de setup corrélé à la charge du daemon ; teardown 2 temps. Correction engagée 2026-10-19 (stop de campagne obligatoire, cap de parallélisme, guard exit-process) — comportements cibles marqués en attente d'intégration F1/F2/F3.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.0
created: 2026-10-19
updated: 2026-10-19
---

# Churn des fixtures testcontainers — dette ratifiée

Dette structurelle ratifiée par l'utilisateur le 2026-10-19 (mission
[[projects/aiforall/references/mission-ops-kind-separation-20261019]]) ;
mémoire canonique : `.serena/memories/architecture/it-testcontainers-resource-churn.md`.
**Mission de correction dédiée lancée le 2026-10-19** :
`docs/missions/20261019-it-fixtures-lifecycle.md` (F1 stop de campagne
lanceur, F2 cycle de vie fixtures `rustycog-testing`, F3 cap de parallélisme,
F4 règles et wiki — dont la présente note). Le contrat X3 de la mission ops
interdisait d'écrire dans `rustycog-testing` ; ce verrou n'est levé que par
cette nouvelle mission.

## Mesures observées (run réel iam 24m53 + finales 5 packages)

- **Pic 35 containers simultanés** pour UN binaire de test (postgres ×N +
  LocalStack à ports aléatoires) ; jusqu'à **97 running** accumulés après une
  campagne complète (45 postgres + 39 openfga + 9 sqs + singletons nommés).
- RAM hôte au pic : **3,5–4 Go libres / 32 Go**.
- `cargo test -p <pkg> --tests` = ~20–49 binaires, tests internes en
  parallèle, chaque binaire acquiert ses propres fixtures réelles
  (herméticité par test, ADR 0200–0202).

## Flake prouvé (corrélation charge ↔ échec de setup)

2 événements identiques le 2026-10-19 : manifesto `member_api_tests` (×2) et
lazaret `health` (×2) — `Failed to create test OpenFGA: opaque fixture start
failure; identity UNKNOWN`, **panic de setup, zéro assertion**. Cause mesurée
au ledger de fixtures : ~28 paires openfga+postgres running simultanément →
daemon Docker sous pression. **Discriminant : reruns ciblés verts**
(12/12 en 24,6s avec `RUST_TEST_THREADS=2` ; 2/2 en 7s). Le `model.json`
régénéré était disculpé par construction (les autres fixtures du même run avec
le même JSON ont démarré).

## Teardown en 2 temps — pas une fuite, mais pas de stop automatique

1. Fin de test : stop propre (exit 0) tracé au registre ;
2. Containers `Exited(0)` **conservés** jusqu'au preflight du run suivant
   (`cleanup_existing_*_container` du harness).

MAIS les singletons nommés (`lazaret_test-*`, `telegraph_test-smtp`) et les
fixtures des packages n'ont **aucun stop automatique à la fin de la campagne**
: 93 containers restés running après les IT finales, arrêtés manuellement par
le lead (IDs complets depuis les ledgers `fixture-*.jsonl` — piège : `docker
ps` tronque les IDs, utiliser `--no-trunc`).

**Règle adoptée le 2026-10-19** : chaque campagne IT inclut désormais son stop
possédé — zéro container possédé running en fin de campagne, y compris sur
échec (canon : `.cursor/rules/docker-resource-registry.mdc`, section
« Campagnes IT testcontainers » ; rappel dans `AGENTS.md`).
**Intégré et prouvé le 2026-10-19** : lanceur `it-native.ps1` (témoin Engine +
propriété par ledger) et guard exit-process `rustycog-testing` ; campagne de
preuve Manifesto+Hive : 74 créés / 74 retirés, pic 4, 0 flake.

## Corrections engagées (mission dédiée 20261019-it-fixtures-lifecycle)

1. **Plafonner le parallélisme** → lot **F3** : cap reproductible du
   parallélisme des tests (config `test-threads`/env dans le lanceur ou config
   Cargo), documentée au README runtime. **Livré** : `RUST_TEST_THREADS=2`
   par défaut, override `AIFORALL_IT_TEST_THREADS` ; re-corrélation : 0 flake
   sur 74 créations (contre 2-3 avant mission).
2. **Mutualisation par binaire** → lot **F2** (`rustycog-testing`, submodule) :
   guard dédié à l'exit du process plutôt que Drop de static, mutualisation
   par-binaire là où l'herméticité le permet ; singletons
   `OnceLock<Arc<Mutex<...>>>` et `port = 0` + publication d'env conservés ;
   bump du pin après publication côté repo rustycog. **Livré** : guard
   exit-process (thread/runtime dédié, stop vérifié exact-ID, sans rm),
   singletons nommés câblés ; pin `rustycog` `2f78d54` publié sur
   Djoe-Denne/rustycog (GO utilisateur) puis bumpé.
3. **Stop possédé en fin de campagne** → lot **F1** (`it-native.ps1`) :
   collecte des IDs créés à la création, stop gracieux en fin de batch et sur
   échec, retrait des `Exited(0)` possédés, rapport d'inventaire avant/après ;
   règle canon adoptée en parallèle (voir section teardown). **Livré et
   prouvé** : 74/74 retirés en campagne réelle, nœud Kind intact.
4. **Profil opt-in court/long** : hors périmètre des lots F1–F4 ; piste
   ouverte, non arbitrée.

## Contraintes non négociables

- La **herméticité** (état neuf par test, pas de pollution croisée) ne se
  négocie pas — l'objectif est le coût, pas l'isolation.
- Toute modification passe par le pin du submodule rustycog (workflow
  `rustycog-submodule`) après publication côté repo rustycog.
- Les suites IT ne tournent jamais sous Kind (décision 2026-10-07).
