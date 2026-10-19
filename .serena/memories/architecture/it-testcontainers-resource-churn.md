# Dette structurelle — churn des fixtures IT testcontainers (ratifiée utilisateur 2026-10-19)

Statut : dette ratifiée par l'utilisateur — **à corriger dans une mission dédiée au
framework**, pas dans la mission ops-kind-runtime-validation (X3 + §9 interdisent
d'y toucher rustycog-testing).

## Faits observés (mission ops 2026-10-19, run T1 iam-service)

- Design hermétique par test (ADR 0200–0202) : chaque test acquiert ses fixtures
  réelles (postgres/LocalStack/Kafka…) sur ports aléatoires, état neuf.
- `cargo test -p iam-service --tests` = ~49 binaires de test, tests internes en
  parallèle → pic mesuré **~35 containers simultanés** (postgres ×N + LocalStack).
- Coût hôte réel : RAM libre au pic **3,5–4 Go / 32 Go** ; run complet 24m53
  (compilation 11m31) pour un seul package.
- Teardown en deux temps, par design : `stopped` propre à la fin du test (36/36
  exit 0), puis containers `Exited(0)` conservés jusqu'au preflight du run
  suivant (`cleanup_existing_*`) — pas une fuite, mais des restes persistants
  entre runs ; registre : 180 events append-only (`.tmp/docker-resources-20261019.log`).

## Pistes de correction (arbitrage utilisateur + architecte)

1. **Plafonner le parallélisme** : `RUST_TEST_THREADS` / config `test-threads`
   par binaire, ou sérialiser les binaires IT (`--test-threads` global) —
   réduit le pic au prix de la durée.
2. **Mutualisation des fixtures par binaire** : passer d'acquisition par-test à
   par-binaire là où l'isolation le permet (skill `creating-testcontainer-fixtures`
   : fixtures singleton `OnceLock<Arc<Mutex<...>>>` existent déjà pour certains
   types) — réduit fortement le churn.
3. **Nettoyage des `Exited(0)` en fin de run** plutôt qu'au preflight suivant
   (possédé, par IDs retournés — jamais par nom, jamais prune).
4. **Profil opt-in "long" vs "rapide"** : bornes de suites pour les gates
   fréquentes vs campagne complète (complète la politique opt-in des queues).

## Interdits actuels (à rappeler)

- Aucune écriture dans `rustycog/rustycog-testing` sans lot framework dédié.
- Aucun changement du design hermétique qui affaiblirait l'isolation d'état
  entre tests (l'objectif est le coût, pas la herméticité).
- Toute modification passe par le pin du submodule (skill `rustycog-submodule`)
  après publication côté repo rustycog.

Sources : ledger mission `.cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md`
(section « Run T1 terminé »), preuves `.tmp/ival-a0eb35a507884366af4a-pool7/`.
