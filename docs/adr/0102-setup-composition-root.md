# ADR-0102 : setup (setup/src/app.rs) est le seul composition root

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Si les handlers construisent des pools SQL ou si le domaine reçoit des `Arc` concrets, l’hexagone est fictif. Le blueprint Manifesto fixe un seul endroit de câblage.

## Décision

1. **`setup/src/app.rs`** (type `Application` / `build_and_run`) est le **seul** composition root : pool DB, publishers, repos, use cases, factory de commandes, `UserIdExtractor`, `PermissionChecker`, `AppState`.
2. Les **handlers HTTP sont minces** : parse → commande → `state.command_service.execute` → map d’erreur locale. Pas de SQL.
3. Surface runtime partagée : **`GenericCommandService`** (wrap du `CommandRegistry`) + **`AppState`** + **`RouteBuilder`** dans la crate `http` (`create_router` / `create_prefixed_router` / `create_app_routes`).
4. Le binaire (`src/main.rs`) charge la config, initialise le logger, appelle `Application::new` / `run` — il ne câble pas les adapters.
5. Le probe `readiness` est **assemblé dans setup** et passé au routeur préfixé ; ce n’est pas un second root (0405).

`RouteBuilder` vit dans `http/src/lib.rs`, pas dans `app.rs`. C’est le contrat HTTP, pas de la DI.

## Conséquences

- Le monolithe compose `create_router` / `create_prefixed_router` + tâches de fond, **jamais** `run()` du service (0404, `nouveau-service.md` §6).
- IAMRusty injecte `InMemoryPermissionChecker` pour satisfaire `AppState::new` (IdP, 0400) — exception AuthZ, pas un second root.
- Manifesto câble en plus client catalogue, outbox, consommateur Apparatus optionnel — toujours dans `setup/src/app.rs`.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Handlers qui parlent SQL / clients HTTP | Casse les tests et le prefix monolithe |
| DI dans le domaine | Le domaine ne connaît que des ports |
| Deux roots (tests vs prod) | `build_and_run` / `maybe_event_publisher` sont le même `Application` |

## Non décidé ici

- Stratégie OpenFGA du checker (0100 / 0302).
- Retry `[command.retry]` (IAM `0` = off ; Hive factory parfois inerte — revue, pas cette ADR).
- `setup_logging` vs subscriber historique (wiki caduc côté Manifesto).

## Références

- `Manifesto/setup/src/app.rs` (`GenericCommandService::new`, `AppState::new`)
- `Hive/setup/src/app.rs`, `Telegraph/setup/src/app.rs`, `IAMRusty/setup/src/app.rs`
- `Manifesto/http/src/lib.rs` (`RouteBuilder::new(state)`), `Manifesto/http/src/handlers/members.rs`
- `Manifesto/src/main.rs` ; skill wiki `building-rustycog-services`
- Preuve : les 4 `setup` créent `GenericCommandService` ; les 4 `http` exposent `RouteBuilder` + `SERVICE_PREFIX`
