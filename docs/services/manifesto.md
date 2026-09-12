# Manifesto

Projets, composants, membership projet. Service de référence pour scaffolder.

- Préfixe : `/manifesto` — compose : **8083**
- JWT : `[auth.jwt]` consommateur
- OpenFGA : types `project` / `component` ; `user:*` via sentinel-sync sur création `public` ou flip `ProjectVisibilityChanged`, jamais sur `publish`. GET/details/composants et liste anonyme fail-closed si la ligne n’est plus world-readable (`public` + `draft|active`). Flip impliquant `public` = `Admin`. Pas de partenariat / join. Détail : [../functional/projet.md](../functional/projet.md).
- Events : `project_created` / `project_visibility_changed` / `project_published` / `project_archived` / membre / permission → `sentinel-sync-events` ; conso optionnelle `component_status_changed`
- Collaborateur HTTP : catalogue composants (`service.component_service`)

## Apparatus P1 — persistance (2026-09-12)

Extension 1:1 `apparatus_bindings` sur `project_components.id` (`component_id` UUID UNIQUE FK CASCADE, `digest` VARCHAR(128) NULL, `source` CHECK legacy|managed). Fichiers : `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs`, `Manifesto/infra/src/apparatus_backfill.rs`, `apparatus_mapping.rs`, `apparatus_outbox.rs`, `transaction.rs`, `Manifesto/http/src/handlers/components.rs`.

- up/down : `cargo run -p manifesto-migration -- up` / `cargo run -p manifesto-migration -- down` (migration additive réversible, up/down/up verts, T1 8/8).
- Backfill explicite : `backfill_apparatus_legacy` (`INSERT...SELECT` legacy `ON CONFLICT DO NOTHING`), idempotent 2 runs, `component_type` non réécrit (T2 5/5). Consentement/génération non ajoutés (ADR dédiée avant P2).

Tests reproductibles (P1 36 = T1-T6 28/28 + mapping 5/5 + T7 3/3 ; P0.1 42/42) :

- Hygiène : `cargo fmt --all -- --check` ; `cargo check -p manifesto-migration` ; `cargo check -p manifesto-infra` ; `cargo clippy` ciblé.
- Contract/sans infra (P0.1) : `cargo test -p apparatus-contracts --features test-harness` (30) ; `cargo test -p apparatus-reference-kv` (12).
- Contract/unit (P1) : `cargo test -p manifesto-infra apparatus_mapping` (mapping injectif 5/5, `check_pairs_injective`, `APPARATUS_MAPPING_COLLISION` v1).
- DB Postgres testcontainer : `cargo test -p manifesto-service --test apparatus_p1_t1_migration -- --test-threads=1` ; `apparatus_p1_t2_backfill` ; `apparatus_p1_t3_collisions` (0 second UUID) ; `apparatus_p1_t4_outbox`.
- AuthZ OpenFGA réel : `cargo test -p manifesto-service --test apparatus_p1_t5_acl -- --test-threads=1` (grants inchangés, revoke→403 TTL0, 0 nouveau type FGA, FGA 5 types).
- HTTP live + wiremock : `cargo test -p manifesto-service --test apparatus_p1_t6_http -- --test-threads=1` (alias `?binding`, POST 7 clés, GET==POST, list `{data}`, 0 second UUID).
- Gate : `cargo test -p manifesto-service --test apparatus_p1_t7_gate -- --test-threads=1` + `grep` bloquants T7 (0 token P2, 0 `VALID`/`VERIFIED` quotés, 5 routes).

## Docs

- [`Manifesto/README.md`](../../Manifesto/README.md), [`Manifesto/docs/`](../../Manifesto/docs/)
- [../functional/projet.md](../functional/projet.md)
- [../guides/nouveau-service.md](../guides/nouveau-service.md)
