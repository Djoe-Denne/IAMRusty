# oodhive-monolith

Monolithe modulaire : un listener HTTP, cinq nests IAM / Telegraph / Hive / Manifesto / Lazaret.

- Package : `oodhive-monolith`
- Nests : `/iam`, `/telegraph`, `/hive`, `/manifesto`, `/lazaret`
- `/health` (process) + `/ready` (crate `readiness`)
- Compose les sorties **setup** (`create_router`, background tasks) — jamais `run()` d’un service
- Listener : `0.0.0.0:8080`, `tls_enabled: false` (`monolith/src/config.rs`)

## J1 — hôte (infra compose + binaire)

Le port **8080 doit être libre** (`iam-service` compose le prend). Ne pas utiliser le stub nginx de `deploy/`.

```bash
just up-infra          # postgres, create-databases, openfga, localstack, openbao — pas de *-service
just monolith          # cargo run -p oodhive-monolith (depuis la racine du dépôt) — RESTART si queues étaient off
just sentinel-sync     # cargo run -p sentinel-sync (process à part ; config/sentinel-sync.toml)
just component-catalog # stub GET :9000/api/components (POST .../components après le grant FGA)
just monolith-prove    # GET /health + POST /lazaret/invoke
```

Équivalent cargo (même CWD racine, pour que rustycog lise `config/development.toml`) :

```bash
cargo run -p oodhive-monolith
```

Préférer `just monolith` : il force `RUN_ENV=development` (le loader rustycog ignore `RUST_ENVIRONMENT`) et pose les `*_DATABASE__DB` vers `iam_dev` / `telegraph_dev` / `hive_dev` / `manifesto_dev` / `lazaret_dev`.

### Preuve invoke (handler Lazaret réel)

Sans `Authorization` → **401** JSON `{"error":"unauthorized"}` (pas le `ok` nginx).

```bash
curl.exe -sS -D - http://127.0.0.1:8080/health
curl.exe -sS -D - -X POST "http://127.0.0.1:8080/lazaret/invoke?project_id=00000000-0000-0000-0000-000000000000" -H "Content-Type: application/json" --data-binary "@monolith/prove-j1-body.json"
```

PowerShell : `.\monolith\prove-j1.ps1`. Bearer bidon + JSON minimal → encore une erreur Lazaret (401 / métier), pas `ok`.

## J3 — overlay démo kind (non canon)

`just deploy-j3` charge `aiforall-oodhive-monolith:j3` sur `aiforall-local` et applique `deploy/apps/overlays/kind-demo-monolith/`. Ce n’est **pas** l’unité cluster 0601. Overlay M2 (`deploy/apps/overlays/kind`) reste le stub nginx.

Postgres/OpenFGA restent sur l’hôte (`just up-infra`) : le pod utilise `host.docker.internal` (Docker Desktop). J1 (`just monolith` / `just monolith-prove`) est inchangé et n’a pas besoin de libérer :8080 pour J3.

## Documentation

- [`docs/services/monolith.md`](../docs/services/monolith.md)
- [`docs/platform/runtime.md`](../docs/platform/runtime.md)
