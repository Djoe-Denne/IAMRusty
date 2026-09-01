# sentinel-sync

Worker d’autorisation : consomme les événements domaine (Hive, Manifesto, IAM, Telegraph), les traduit en writes/deletes OpenFGA, avec ledger d’idempotence.

- Pas d’HTTP
- **Pas** dans `docker-compose.yml` par défaut
- Translators : `src/translator/{hive,manifesto,iam,telegraph}.rs`

## Lancer

1. Store + model (`openfga/model.fga`) déjà poussés.
2. Config queue + `store_id` / `authorization_model_id`.
3. `cargo run -p sentinel-sync`

Un event sans bras de translator est un no-op (le store FGA ne bouge pas).

## Documentation

- [`docs/services/sentinel-sync.md`](../docs/services/sentinel-sync.md)
- [`docs/platform/events-outbox.md`](../docs/platform/events-outbox.md)
- [`docs/guides/nouveau-service.md`](../docs/guides/nouveau-service.md) (étape translator)
