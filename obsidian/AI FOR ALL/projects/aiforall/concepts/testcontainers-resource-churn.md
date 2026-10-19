---
title: >-
  Churn des fixtures testcontainers — mesure, flake prouvé, pistes de correction
category: concepts
tags: [testing, docker, rust, debt, visibility/internal]
sources:
  - .serena/memories/architecture/it-testcontainers-resource-churn.md
  - .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md
  - .cursor/qa-plans/20261019-ops-kind-runtime-validation-qa-plan.md
summary: >-
  Un cargo test IT spawn des dizaines de containers simultanés (35–97 mesurés), flake de setup corrélé à la charge du daemon ; teardown 2 temps, pistes bornées.
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
**À corriger dans une mission dédiée au framework** — le contrat X3 de la
mission ops interdisait d'écrire dans `rustycog-testing`.

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
ps` tronque les IDs, utiliser `--no-trunc`). Chaque campagne doit inclure son
stop possédé (règle registre : `.cursor/rules/docker-resource-registry.mdc`).

## Pistes de correction (mission framework dédiée)

1. **Plafonner le parallélisme** : `RUST_TEST_THREADS` / sérialisation des
   binaires IT — réduit le pic au prix de la durée.
2. **Mutualisation par binaire** : passer l'acquisition par-test à par-binaire
   là où l'isolation le permet (singletons `OnceLock<Arc<Mutex<...>>>`
   existent déjà pour certains types) — réduit fortement le churn.
3. **Stop possédé en fin de campagne** : intégrer au lanceur IT
   (`it-native.ps1`) l'arrêt gracieux des IDs créés + nettoyage des `Exited(0)`
   possédés (jamais par nom, jamais prune).
4. **Profil opt-in court/long** : bornes de suites pour les gates fréquentes
   vs campagne complète.

## Contraintes non négociables

- La **herméticité** (état neuf par test, pas de pollution croisée) ne se
  négocie pas — l'objectif est le coût, pas l'isolation.
- Toute modification passe par le pin du submodule rustycog (workflow
  `rustycog-submodule`) après publication côté repo rustycog.
- Les suites IT ne tournent jamais sous Kind (décision 2026-10-07).
