# ADR-0101 : Une crate par couche hexagonale, plus le binaire *-service et tests/

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Un hexagone dans un seul crate mélange ports, SQL et Axum. Les guides Manifesto imposent des crates de couche **avant** le premier use case.

## Décision

Chaque service métier a **une crate par couche** :

| Couche | Rôle |
|---|---|
| `domain` | entités, ports (traits), services domaine |
| `application` | use cases, commandes, factory de registre |
| `infra` | adapters (DB, clients HTTP, JWT/OAuth, outbox) |
| `http` | `RouteBuilder`, handlers minces, mapping HTTP |
| `configuration` | config typée `rustycog-config` |
| `migration` | schéma SeaORM |
| `setup` | composition root |

Plus :

- le **package binaire** `{iam,hive,telegraph,manifesto}-service` à la racine du service (`src/main.rs`) ;
- `tests/` d’intégration (hors crate, harness rustycog-testing — 0200).

Noms de crates HTTP **divergents** (`hive-http` vs `*_http_server`) : choix local, pas une quatrième architecture.

Le framework `rustycog/` est feature-gated (`command`, `db`, `events`, `http`, `outbox`, `permission`, `testing`, `logger`, `config`, `core`, `server`) — voir 0502. Ce n’est pas une couche du service.

## Conséquences

- `domain/Cargo.toml` ne dépend pas de `sea-orm` / `axum` / `reqwest` (attesté sur les 4).
- `docs/guides/nouveau-service.md` omet `migration` dans la liste courte ; le code et le wiki l’incluent — suivre le code.
- Crates `*-events` workspace = contrat (0300), pas une 8ᵉ couche du slice.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Hexagone dans un crate | Circularité HTTP↔SQL ; tests et DI illisibles |
| `setup` fusionné dans le binaire | Le monolithe doit extraire `router` sans `run()` |
| Crate `service/` séparée en plus du package racine | Le dépôt utilise déjà le package racine `*-service` |

## Non décidé ici

- Harmoniser `hive-http` vs `*-http_server`.
- Cycle de vie des crates `*-events` (0300).
- Feature `kafka` rustycog hors `full` Windows.

## Références

- `Manifesto/docs/rustycog-hexagonal-web-service-guide.md` §4, `rustycog-service-build-guide.md` §2
- `IAMRusty/Cargo.toml` (`iam-service` + `iam-migration`), idem Hive/Telegraph/Manifesto
- Preuve : dossiers `domain`, `application`, `infra`, `http`, `configuration`, `migration`, `setup`, `tests`, `src/main.rs` dans les 4 services
