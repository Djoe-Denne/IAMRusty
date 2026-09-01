# Runtime : compose, standalone, monolithe

## Microservices (défaut)

```bash
git clone --recurse-submodules <url>
cd AIForAll
docker compose up -d
```

Le compose démarre postgres, LocalStack, OpenFGA (migrate + run), crée les bases, puis `iam-service`, `telegraph-service`, `hive-service`, `manifesto-service`.

Outils (profil `tools`, hors `up` par défaut) :

```bash
docker compose --profile tools run --rm truncate-db
docker compose --profile tools run --rm verify-emails
docker compose --profile tools run --rm list-databases
```

`truncate-db` / `verify-emails` ciblent `iam_dev` par défaut (`TARGET_DB`).

## Standalone (dev crate)

Chaque service lit `RUST_ENVIRONMENT` (`development` / `test` / …) et le TOML sous `<Service>/config/`. Préfixe env : `IAM_`, `HIVE_`, `TELEGRAPH_`, `MANIFESTO_`.

Exemples :

```bash
cargo run -p manifesto-service
cargo run -p sentinel-sync
```

Migrations : `cargo run -p manifesto-migration -- up` (même schéma pour `iam-migration`, `hive-migration`, `telegraph-migration` selon les crates du service).

## Monolithe modulaire

Package `oodhive-monolith`. Il **ne** appelle **pas** les `run()` des services : setup → extract `create_router(state)` → `start_background_tasks()` → un seul `compose_routes` ([`monolith/src/routes.rs`](../../monolith/src/routes.rs)).

Chemins identiques au standalone grâce aux `SERVICE_PREFIX` :

- `/iam/...`
- `/telegraph/...`
- `/hive/...`
- `/manifesto/...`
- `/health` (monolithe) et `/ready` (crate `readiness`, attaché sur le routeur composé)

Les tests d’intégration de chaque service doivent renvoyer une base URL **déjà préfixée** (`/hive`, …) pour rester valides dans les deux modes.

## Readiness

La crate [`readiness/`](../../readiness/) expose `/ready`. Les factories de queue rustycog peuvent **réussir en no-op** : un boot « OK » ne prouve pas que SQS/Kafka est live. Les probes doivent classer ce cas (voir le concept wiki `queue-readiness-signaling`).

`OpenFgaClientConfig.port = 0` : le client choisit un port libre (fixture test). En compose, OpenFGA écoute 8080 **dans** le réseau ; l’hôte mappe 8090.

## OpenFGA local

1. Compose démarre migrate puis `openfga run`.
2. Créer un store et pousser [`openfga/model.fga`](../../openfga/model.fga) (CLI `fga`).
3. Injecter `HIVE_OPENFGA__STORE_ID`, `HIVE_OPENFGA__AUTHORIZATION_MODEL_ID` (idem `MANIFESTO_`, `TELEGRAPH_`, config sentinel-sync).

Sans store_id, les Check HTTP fail-closed (403 + log upstream).

## Suite

- [overview.md](overview.md)
- [config-cheatsheet.md](config-cheatsheet.md)
- Wiki : `projects/aiforall/skills/running-aiforall-runtime-modes`
