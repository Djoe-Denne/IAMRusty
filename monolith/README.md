# oodhive-monolith

Monolithe modulaire : un listener HTTP, les quatre routeurs IAM / Telegraph / Hive / Manifesto.

- Package : `oodhive-monolith`
- Nests : `/iam`, `/telegraph`, `/hive`, `/manifesto`
- `/health` (process) + `/ready` (crate `readiness`)
- Compose les sorties **setup** (`create_router`, background tasks) — jamais `run()` d’un service

## Lancer

```bash
cargo run -p oodhive-monolith
```

Hors `docker-compose` par défaut. Les chemins sont les mêmes qu’en microservice.

```bash
curl http://localhost:8080/health
curl http://localhost:8080/iam/.well-known/jwks.json
curl http://localhost:8080/hive/api/organizations/search
```

## Documentation

- [`docs/services/monolith.md`](../docs/services/monolith.md)
- [`docs/platform/runtime.md`](../docs/platform/runtime.md)
