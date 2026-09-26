# oodhive-monolith

Un listener, cinq bounded contexts. Compose les `create_router` + background tasks. Pas d’appel à `run()` des services.

- Package : `oodhive-monolith`
- Nests : `/iam`, `/telegraph`, `/hive`, `/manifesto`, `/lazaret` ([`monolith/src/routes.rs`](../../monolith/src/routes.rs))
- `/health` monolithe + `/ready` (crate `readiness`)
- Listener hôte : `0.0.0.0:8080` (le port doit être libre — pas `iam-service` compose)
- Hors compose apps par défaut ; infra : `just up-infra` puis `just monolith` puis `just sentinel-sync` (worker FGA, process séparé)

Contrats de chemins **identiques** au standalone. Tests IT préfixés. Détail : [../platform/runtime.md](../platform/runtime.md). Atlas visuel : [../architecture/runtime.md](../architecture/runtime.md) (index [../architecture/README.md](../architecture/README.md)).
