# ADR-0404 : Dual runtime — standalones et `oodhive-monolith` préfixé ; le monolithe ne remplace pas

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Quatre vertical slices (IAM, Telegraph, Hive, Manifesto) doivent tourner en compose (un process par service) **et** sous un listener unique pour le dev / un déploiement compact. La tentation est de déclarer le monolithe « la » runtime, ou d’y embarquer `sentinel-sync`.

`docs/platform/runtime.md` et `oodhive-monolith` composent déjà les routeurs préfixés sans appeler `run()` des services.

## Décision

1. **Dual runtime** : les **standalones** (`iam-service`, `telegraph-service`, `hive-service`, `manifesto-service`) **et** le package **`oodhive-monolith`** sont tous deux des modes supportés.
2. Le monolithe **compose** `create_router` + `start_background_tasks` + `compose_routes` (`monolith/src/routes.rs`). Il **n’appelle pas** les `run()` standalone. Il **ne remplace pas** les microservices : le compose par défaut reste les quatre services + infra.
3. **Mêmes contrats de chemins** grâce aux `SERVICE_PREFIX` : `/iam`, `/telegraph`, `/hive`, `/manifesto`. Les IT doivent utiliser une base URL **déjà préfixée**.
4. Santé : `/health` monolithe + `/ready` (crate `readiness`, 0405) sur le routeur composé.
5. **`sentinel-sync` est hors dual** : worker événements → tuples (0303), pas un routeur HTTP, **absent** du compose par défaut, **non nesté** dans le monolithe.

Hors compose par défaut : monolithe et sentinel-sync (`cargo run -p oodhive-monolith` / `cargo run -p sentinel-sync`).

## Conséquences

- Un nouveau service métier expose `SERVICE_PREFIX` + `create_prefixed_router` s’il doit vivre dans les deux modes.
- On ne documente pas le monolithe comme successeur des standalones.
- Un event AuthZ n’est pas « inclus » parce que le monolithe tourne : il faut encore lancer `sentinel-sync` après bootstrap OpenFGA.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Monolithe seul | Le compose et les crates `*-service` sont le défaut documenté |
| Standalones seuls | `oodhive-monolith` existe et partage les préfixes |
| Nest sentinel-sync dans le monolithe | Ce n’est pas une slice HTTP (0303, 0100) |

## Non décidé ici

- Déploiement Kubernetes / un seul vs quatre Deployments.
- Inclusion future d’un host Apparatus dans le dual (P5, 0002 / 0406).
- Queues no-op au boot vs `/ready` (0405).

## Références

- Handbook : `docs/platform/runtime.md`, `docs/services/monolith.md`, `docs/services/sentinel-sync.md`, `README.md`
- Code : `monolith/src/routes.rs`, `*/http/src/lib.rs` (`SERVICE_PREFIX`, `create_prefixed_router`)
- Preuve : compose démarre les 4 services ; `oodhive-monolith` hors compose ; sentinel-sync hors les deux
