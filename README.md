# AIForAll

Plateforme Rust de bounded contexts (identité, organisations, projets, notifications) sur le SDK [rustycog](https://github.com/Djoe-Denne/rustycog) (git submodule).

**Handbook (implémentation + parcours métier) :** [`docs/README.md`](docs/README.md)

## Services

| Service | Préfixe | Port compose | Rôle |
|---|---|---|---|
| [IAMRusty](IAMRusty/README.md) | `/iam` | 8080 | Identité, OAuth, JWT |
| [Telegraph](Telegraph/README.md) | `/telegraph` | 8081 | Emails et notifications in-app |
| [Hive](Hive/README.md) | `/hive` | 8082 | Organisations, membres, invitations |
| [Manifesto](Manifesto/README.md) | `/manifesto` | 8083 | Projets, composants, membership |
| [sentinel-sync](sentinel-sync/README.md) | — | (hors compose) | Events → tuples OpenFGA |
| [oodhive-monolith](monolith/README.md) | `/iam`…`/manifesto` | (hors compose) | Un listener, quatre routeurs |

Infra compose : PostgreSQL **5432**, LocalStack SQS **4566**, OpenFGA **8090** (HTTP) / **8091** (gRPC) / **3000** (playground).

Crates d’événements : `iam-events`, `hive-events`, `manifesto-events`, `telegraph-events`. Readiness : crate `readiness` (`/ready`).

## Démarrage

```bash
git clone --recurse-submodules <url>
cd AIForAll
docker compose up -d
```

Logs : `docker compose logs -f`. Stop : `docker compose down`.

Outils (profil `tools`) :

```bash
docker compose --profile tools run --rm truncate-db
docker compose --profile tools run --rm verify-emails
```

JWT local : HMAC partagé, `iss=iamrusty`, `aud=aiforall`. Détail : [`docs/platform/authn-jwt.md`](docs/platform/authn-jwt.md).

## Flux

- IAM publie `user_signed_up` / `password_reset_requested` / `user_email_verified` vers **`telegraph-events`**.
- Hive, Manifesto, Telegraph publient vers **`sentinel-sync-events`** (AuthZ).
- `sentinel-sync` n’est **pas** démarré par le compose — le lancer à part après bootstrap du store OpenFGA.

## Monolithe

```bash
cargo run -p oodhive-monolith
```

Mêmes préfixes qu’en standalone. Voir [`docs/platform/runtime.md`](docs/platform/runtime.md) et [`monolith/README.md`](monolith/README.md).

## Tests et format

```bash
cargo test -p hive-service
cargo fmt
```

Hook fmt : [`docs/CARGO_FMT_PRE_COMMIT.md`](docs/CARGO_FMT_PRE_COMMIT.md).

Nouveau service : [`docs/guides/nouveau-service.md`](docs/guides/nouveau-service.md). Submodule rustycog : [`.agents/skills/rustycog-submodule/SKILL.md`](.agents/skills/rustycog-submodule/SKILL.md).
