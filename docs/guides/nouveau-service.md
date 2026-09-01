# Ajouter un service RustyCog sur AIForAll

Deux couches. Ne pas s’arrêter à la première.

1. **Forme crate** (Manifesto) — skill [building-rustycog-services.md](../../.agents/skills/rustycog/references/building-rustycog-services.md) + [`Manifesto/docs/`](../../Manifesto/docs/).
2. **Branchement plateforme** (cette page) — events, OpenFGA, sentinel-sync, compose, monolithe, préfixe.

Skill agent : [`.agents/skills/aiforall-new-service/SKILL.md`](../../.agents/skills/aiforall-new-service/SKILL.md).

## 1. Vertical slice Manifesto

Crates : `domain`, `application`, `infra`, `http`, `setup`, `configuration`, `tests`. Un use case de bout en bout avant de tout scaffolder.

- Config typée d’abord (`using-rustycog-config`).
- Un `DbConnectionPool`, registries, `GenericCommandService`.
- `AppState::new(command_service, user_id_extractor, permission_checker)`.
- HTTP : `SERVICE_PREFIX`, `create_router(state)`, `create_prefixed_router(state, probe)`, `create_app_routes` → `serve_router`.
- Setup : `router()` (unprefixed) + `start_background_tasks` / `stop_background_tasks` pour le monolithe.
- Logging : un seul `setup_logging` **ou** tracing maison, pas les deux. Manifesto utilise encore le tracing maison (écart à trancher).

## 2. JWT consommateur

TOML `[auth.jwt]` aligné sur IAM (`hs256_secret`, `issuer = "iamrusty"`, `audience = "aiforall"`). Recette : [jwt-consommateur.md](jwt-consommateur.md).

## 3. OpenFGA

- Si nouveau type : l’ajouter dans [`openfga/model.fga`](../../openfga/model.fga) **et** republier `model.json`.
- Routes protégées : `.authenticated()` puis `.with_permission_on` / `.with_permission_on_param` ([permissions.md](permissions.md)).
- Tests : `has_openfga() == true`, `include_str!("../../openfga/model.json")`, `allow` explicite.

## 4. Crate d’événements

Workspace member `foo-events` (comme `hive-events`). Le service publie ; les consommateurs dépendent du crate, pas de l’inverse.

Si l’event change l’AuthZ : bras dans `sentinel-sync/src/translator/` + enregistrement dans `main.rs`. Sans ça, le store FGA ne bouge pas.

## 5. Outbox / queue

`[queue.queues]` : chaque `event_type` → file(s) physiques. Consommateurs AuthZ → `sentinel-sync-events`. Notifications IAM-like → file Telegraph. Default TOML : `enabled = false`.

## 6. Monolithe

[`monolith/src/routes.rs`](../../monolith/src/routes.rs) + runtime : extraire le router, **ne pas** appeler `run()`. Ajouter le champ dans `MonolithRouters` et `.nest(foo_http::SERVICE_PREFIX, routers.foo)`.

## 7. Compose et workspace

- Member dans le `Cargo.toml` racine.
- Service dans [`docker-compose.yml`](../../docker-compose.yml) : port hôte libre (8084+), `DATABASE_URL`, volume `config`, `depends_on` postgres / localstack / openfga / `create-databases`.
- Étendre `create-databases` avec `foo_dev`.
- README service + fiche [`docs/services/`](../services/).

## 8. Tests

`tests/common.rs` wrappe l’origin rustycog avec **le même** `SERVICE_PREFIX`. Bodies : `{server_url}/api/...` sans re-préfixer. Voir [tests-integration.md](tests-integration.md).

## Ordre recommandé

Config JWT → composition root (DB, commands, checker) → une route authentifiée + test → events + translator si AuthZ → nest monolithe → compose.
