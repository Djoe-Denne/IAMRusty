# sentinel-sync

Worker (pas d’HTTP). Consomme les events domaine, traduit en writes/deletes OpenFGA, idempotence via ledger.

- Package : `sentinel-sync`
- **Absent** du `docker-compose.yml` par défaut — `cargo run -p sentinel-sync` après bootstrap store/model
- Translators : Hive, Manifesto, IAM, Telegraph (`sentinel-sync/src/translator/`)
- Event inconnu → no-op (pas d’erreur)

Tout nouvel event qui change l’AuthZ **doit** avoir un bras de translator. Voir [../platform/events-outbox.md](../platform/events-outbox.md) et [../guides/nouveau-service.md](../guides/nouveau-service.md).
