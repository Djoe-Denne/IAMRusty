# sentinel-sync

Worker (pas d’HTTP). Consomme les events domaine, traduit en writes/deletes OpenFGA, idempotence via ledger.

- Package : `sentinel-sync`
- **Absent** du `docker-compose.yml` par défaut — process à part, **non nesté** dans `oodhive-monolith`
- Chaîne hôte : `just up-infra` → files SQS on (`config/development.toml`) → `just monolith` (**restart** si le process a booté queues off) → `just sentinel-sync` (`cargo run -p sentinel-sync`, CWD racine, `config/sentinel-sync.toml`)
- Store OpenFGA : `openfga-migrate` ne crée pas le store ; `openfga/ensure-host-store.ps1` (appelé par `just monolith` / `just sentinel-sync`) réutilise ou crée le store `aiforall` et aligne `MANIFESTO_OPENFGA__STORE_ID` + `SENTINEL_SYNC_OPENFGA__STORE_ID`
- Translators : Hive, Manifesto, IAM, Telegraph (`sentinel-sync/src/translator/`)
- Event inconnu → no-op (pas d’erreur)

Tout nouvel event qui change l’AuthZ **doit** avoir un bras de translator. Voir [../platform/events-outbox.md](../platform/events-outbox.md) et [../guides/nouveau-service.md](../guides/nouveau-service.md).
