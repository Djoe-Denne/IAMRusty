# Hive

Organisations, membres, invitations, liens externes et jobs de sync.

- Préfixe HTTP : `/hive` (compose : port hôte **8082**)
- JWT consommateur : `[auth.jwt]` — `hs256_secret`, `issuer = "iamrusty"`, `audience = "aiforall"`
- AuthZ : OpenFGA type `organization` ; routes imbriquées via `with_permission_on_param`
- Events : `organization_*` / `member_*` → `sentinel-sync-events`

## Lancer

```bash
cargo run -p hive-migration -- up
cargo run -p hive-service
```

Config : `Hive/config/` (`HIVE_*`). Queues `enabled = false` par défaut.

## Documentation

- Handbook : [`docs/services/hive.md`](../docs/services/hive.md), [`docs/functional/organisation.md`](../docs/functional/organisation.md)
- JWT : [`docs/guides/jwt-consommateur.md`](../docs/guides/jwt-consommateur.md)
- Permissions : [`docs/guides/permissions.md`](../docs/guides/permissions.md)
