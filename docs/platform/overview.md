# Vue d’ensemble de la plateforme

AIForAll est un workspace Rust de bounded contexts hexagonaux, tous câblés via le SDK [rustycog](https://github.com/Djoe-Denne/rustycog) (git submodule `rustycog/`). Deux modes d’exécution : microservices autonomes, ou monolithe modulaire `oodhive-monolith`.

## Catalogue

| Composant | Rôle | Préfixe HTTP | Port compose (hôte) | Crate binaire |
|---|---|---|---|---|
| **IAMRusty** | Identité, OAuth, JWT, refresh | `/iam` | 8080 | `iam-service` (via `IAMRusty`) |
| **Telegraph** | Emails et notifications in-app | `/telegraph` | 8081 → 8080 | `telegraph-service` |
| **Hive** | Organisations, membres, invitations | `/hive` | 8082 → 8080 | `hive-service` |
| **Manifesto** | Projets, composants, membres projet | `/manifesto` | 8083 → 8080 | `manifesto-service` |
| **sentinel-sync** | Worker : événements → tuples OpenFGA | (aucun HTTP) | — | `sentinel-sync` |
| **oodhive-monolith** | Compose les 4 routeurs sous un listener | `/iam` … `/manifesto` | (hors compose) | `oodhive-monolith` |
| **readiness** | Crate `/ready` partagée | `/ready` sur chaque service | — | `readiness` |

Préfixes : constantes `SERVICE_PREFIX` dans chaque crate HTTP (`IAMRusty/http`, `Hive/http`, …). Le monolithe les réutilise via `.nest(...)` — même contrat de chemins en standalone et en monolithe.

## Infra locale (`docker-compose.yml`)

| Service | Port hôte | Usage |
|---|---|---|
| PostgreSQL 15 | 5432 | `iam_dev`, `telegraph_dev`, `hive_dev`, `manifesto_dev`, `openfga_dev` |
| LocalStack 3 | 4566 | SQS |
| OpenFGA | 8090 (HTTP), 8091 (gRPC), 3000 (playground) | Check / Write d’autorisation |

`sentinel-sync` **n’est pas** dans le compose par défaut : le worker se lance à part (`cargo run -p sentinel-sync`) une fois store + model OpenFGA bootstrappés.

## Crates d’événements

Contrats de payload partagés, pas des services :

- `iam-events` — `user_signed_up`, `user_email_verified`, `password_reset_requested`, …
- `hive-events` — cycle de vie org / membres
- `manifesto-events` — projets, composants, permissions
- `telegraph-events` — `notification_created`

## Flux inter-services (résumé)

```
IAM  --user_* events-->  Telegraph (email / notification)
Hive / Manifesto / Telegraph  --domain events-->  sentinel-sync  --> OpenFGA
Hive / Manifesto / Telegraph  --Check-->  OpenFGA (via rustycog-permission)
Tous les HTTP métier  --Bearer HS256-->  rustycog-http UserIdExtractor
IAM  --émet le JWT-->  même secret HMAC + iss=iamrusty / aud=aiforall
```

Détail : [authn-jwt.md](authn-jwt.md), [authz-openfga.md](authz-openfga.md), [events-outbox.md](events-outbox.md).

## SDK

`rustycog/` est un **gitlink** (mode `160000`) vers `Djoe-Denne/rustycog`. Cargo patche `rustycog-framework` depuis ce chemin. Voir [`.agents/skills/rustycog-submodule/SKILL.md`](../../.agents/skills/rustycog-submodule/SKILL.md).

## Suite

- Runtime : [runtime.md](runtime.md)
- Nouveau service : [../guides/nouveau-service.md](../guides/nouveau-service.md)
- Fiches : [../services/](../services/)
