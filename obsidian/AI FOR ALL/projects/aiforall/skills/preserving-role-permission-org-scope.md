---
title: >-
  role_permissions : unicité par organisation
category: skills
tags: [skill, hive, postgres, migration, visibility/internal]
sources:
  - services/Hive/migration/src/m20261003_000014_role_permission_organization_scope.rs
  - services/Hive/migration/src/lib.rs
  - services/Hive/tests/organization_api_tests.rs
  - conversation opencode 2026-10-03 (clôture 0308)
summary: >-
  Migration qui remplace l'unicité globale (permission, resource) par
  (organization, permission, resource) : restaure la création de la
  2e organisation ; up/down sans perte, refus propre sous collision.
provenance:
  extracted: 0.85
  inferred: 0.12
  ambiguous: 0.03
created: 2026-10-03T15:10:00Z
updated: 2026-10-03T15:10:00Z
---

# role_permissions : unicité par organisation

Correctif de bug préexistant découvert le 2026-10-03 pendant l'e2e Kind ([[projects/aiforall/references/opencode-close-0308-2026-10-03]]) : `POST /hive/api/organizations` échouait en 400 avec `duplicate key value violates unique constraint idx_role_permissions_unique_combo`, parce que l'ancien index était global `(permission_id, resource_id)` alors que chaque organisation sème les mêmes combinaisons par défaut.

## Quelle était l'intention initiale

[[projects/manifesto/manifesto|Manifesto]] fait la même chose : `m20241015_000006_create_role_permissions_table.rs:101-111` scopa l'unicité par projet, pas globale. Le schéma Hive avait dérivé ; le fix le raccorde à l'invariant établi par le code, pas à une nouvelle décision (validation [[projects/aiforall/concepts/architecte-agent|architecte]] du 2026-10-03 : aucun besoin d'ADR, l'invariant est déjà le modèle).

## Ce que fait la migration

- `up` : crée `unicité (organization_id, permission_id, resource_id)` **avant** de drop l'ancienne globale (aucun moment sans contrainte d'unicité).
- `down` : recrée d'abord l'index global ; moderne et sûr : **échoue** franchement si des combinaisons cross-organisation existent désormais, **sans supprimer de données** et en laissant l'index org-scopé en place (SeaORM Migration 1.1.20, transactionnelle).

`RolePermission` est le rôle/template : `id` sert de `role_id`. resource_id est NOT NULL (catalogue) — pas de sémantique « NULL = global ». Ne pas aligner silencieusement sur `hive_database_schema.sql` (descriptif, périmé : il décrit encore une jonction `organization_role_id`).

## Tests de conservation (services/Hive/tests/organization_api_tests.rs)

- `create_two_organizations_preserves_scoped_default_role_permissions`
- `role_permission_scope_migration_round_trip_preserves_populated_rows` — upgrade depuis l'ancien schéma peuplé conserve IDs + grants owner
- `role_permission_scope_migration_rejects_colliding_downgrade_without_loss`
- Roulés en Docker avec testcontainers Postgres+OpenFGA (19/19), puis l'e2e Kind 35/35 le prouvé au runtime (`create_organization` + membre owner avec `issuer` du JWT).

## Related

- [[projects/hive/hive]] — hub Hive.
- [[projects/aiforall/skills/running-it-tests-docker]] — comment roulés ces tests.
- [[projects/aiforall/references/opencode-close-0308-2026-10-03]] — d'où vient le fix.
