# Cheatsheet configuration

Les sections `server`, `database`, `logging`, `queue`, `command` sont des **contrats rustycog**. Ne pas inventer de formes parallèles. `load_config_part("server")` lit les overrides `SERVER_*`, **pas** le préfixe service.

## Préfixes env

| Service | Préfixe | Exemple |
|---|---|---|
| IAMRusty | `IAM_` | `IAM_JWT__SECRET__VALUE`, `IAM_AUTH__JWT__HS256_SECRET` |
| Hive | `HIVE_` | `HIVE_OPENFGA__STORE_ID`, `HIVE_QUEUE__ENABLED` |
| Manifesto | `MANIFESTO_` | `MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID` |
| Telegraph | `TELEGRAPH_` | `TELEGRAPH_QUEUE__ENABLED` |
| sentinel-sync | (config dédiée) | store / queue du worker |

Double underscore = nesting TOML (`auth.jwt.hs256_secret` → `HIVE_AUTH__JWT__HS256_SECRET`).

## Sections partagées

```toml
[server]
host = "0.0.0.0"
port = 8080
tls_enabled = false

[database]
host = "postgres"          # ou localhost
port = 5432                # 0 = port libre (tests)
db = "hive_dev"
[database.creds]
username = "postgres"
password = "postgres"

[logging]
level = "debug"

[command.retry]
max_attempts = 0           # 0 = pas de retry
base_delay_ms = 50
max_delay_ms = 5000
backoff_multiplier = 2.0
use_jitter = false

[queue]
type = "sqs"
enabled = false
# … region, account_id, host, port, creds LocalStack
default_queues = ["…"]
[queue.queues]
# event_type = ["physical-queue", …]
```

## JWT — deux blocs

| Bloc | Qui | Champs |
|---|---|---|
| `[jwt]` + `[jwt.secret]` | IAM **émetteur** | `expiration_seconds`, `issuer`, `audience`, `secret.type/value` |
| `[auth.jwt]` | **tous** les HTTP (extracteur) | `hs256_secret`, `issuer`, `audience` |

IAM a les deux. Les consommateurs n’ont que `[auth.jwt]`. Détail : [authn-jwt.md](authn-jwt.md).

## OpenFGA (consommateurs Check)

```toml
[openfga]
scheme = "http"
host = "openfga"           # compose
port = 8080                # 0 en test (random)
# store_id / authorization_model_id via env
cache_ttl_seconds = 0      # tests : skip cache
```

## Files spécifiques Telegraph

`[queues.telegraph-events]` n’est **pas** le contrat rustycog générique : c’est le routage interne event → mode (`email` / `notification`) + template.

## Pièges

- `config/default.toml` n’est pas toujours mergé automatiquement — vérifier le loader du service.
- Queues `enabled = false` en default/dev/test : un `type = "sqs"` ne démarre pas le transport.
- Secrets HMAC et `postgres:postgres` sont dans Git pour le local — **jamais** en prod (TOML production déjà vidés pour JWT).
