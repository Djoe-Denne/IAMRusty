# Revue d’architecture — Telegraph (RustyCog / IAM)

| Champ | Valeur |
|---|---|
| Date | 2026-08-29 |
| Cible | Service **Telegraph** (`telegraph-service`, dit « telégraf ») — communication / notifications alimentées par les events IAM |
| Référentiel | RustyCog (skill + `references/`) + wiki QMD `aiforall-wiki` |
| Code métier | **non modifié** |

## 1. Périmètre

Telegraph n’est pas IAMRusty : c’est le service de communication de la plateforme Rusty (emails + notifications in-app) qui **consomme** le contrat `iam-events`. Il vit sous `Telegraph/` et s’intègre à l’IAM via JWT partagé, OpenFGA (`notification`), SQS, et `sentinel-sync` (tuples, **si** Telegraph publie `NotificationCreated`).

| Crate | Rôle |
|---|---|
| `telegraph-service` (`Telegraph/`) | Binaire + tests d’intégration |
| `telegraph-domain` | Entités, ports, `DomainError` local, services |
| `telegraph-application` | Use cases, commandes (`ProcessEvent`, notifications) |
| `telegraph-infra` | SMTP, Tera, SeaORM, consumer SQS, processors |
| `telegraph-http_server` | `RouteBuilder`, handlers, `SERVICE_PREFIX` |
| `telegraph-configuration` | `TelegraphConfig` typé |
| `telegraph-setup` | Composition root `TelegraphApp` / `AppBuilder` |
| `telegraph-migration` | Migration SeaORM notifications |
| `iam-events` | Contrat IAM **utilisé** (`UserSignedUp`, `UserEmailVerified`, `PasswordResetRequested`, `UserLoggedIn`) |

Pas de dépendance crate vers `IAMRusty`. OpenFGA partagé : `openfga/model.fga` type `notification`.

## 2. Méthode et limites outillage

| Outil | Statut |
|---|---|
| **Serena** | OK — `initial_instructions` + `activate_project` AIForAll ; overview / `find_symbol` / références / patterns |
| **GrepAI** (`user-grepai`) | Index MCP **hors sujet** (racine Alcoholic). RPG off. **CLI locale** `grepai search` / `trace` depuis AIForAll : utile (factory, `create_router`, processors) |
| **Context Mode** | **Indisponible** — aucun namespace MCP `context-mode` / `ctx_*` dans le catalogue de session (pourtant déclaré dans `.cursor/mcp.json`) |
| **QMD** (CLI) | OK — collection `aiforall-wiki` (`projects/telegraph/*`, skills rustycog) ; wiki ~118 j., encore pertinent |
| **RustyCog** | Référentiel normatif (skill + `building-rustycog-services.md` + `using-rustycog-*.md`) |

## 3. Synthèse

Telegraph suit le **gabarit hexagonal RustyCog** (slice verticale, un composition root, `GenericCommandService` partagé HTTP + queue, `SERVICE_PREFIX=/telegraph`, APIs monolithe). C’est le complément event-driven d’IAMRusty : descriptors TOML + Tera, consumer SQS, read-model notifications. Les écarts graves sont : **flattening** de toutes les erreurs queue en `ServiceError::infrastructure`, **`unwrap` au boot** sur le mapping `queues.*.event_configs`, **OpenFGA `with_permission_on` sans publisher `NotificationCreated`**, et des mappers notification qui ignorent `DomainError`. Contrairement à Manifesto, le bin utilise bien `setup_logging`.

**Verdict global : conforme sur le layout, partiel sur erreurs / AuthZ / events** — pas une divergence de boundaries.

## 4. Tableau des 12 axes

| # | Axe | Verdict | Écart vs rustycog / consignes |
|---|---|---|---|
| 1 | Layout crate / boundaries | **conforme** | Hexagonale + ports. Domain réexporte `iam_events::*` (et donc `rustycog::events`). Handlers `communication` morts. |
| 2 | Config / env / secrets | **partiel** | Préfixe `TELEGRAPH`, sections rustycog sauf **`command`**. Descriptors hardcodés. Secrets HS256 / Postgres / AWS en git. SMS doc ≠ struct. |
| 3 | Erreurs | **divergent** | `DomainError` local `thiserror`, pas `ServiceError` HTTP. Mappers notif = tout infra. Queue wrappe tout en infra. `unwrap` prod. `HttpError` mort. |
| 4 | Observabilité | **partiel** | `setup_logging` + champs structurés consumer. Emojis. Pas de `#[instrument]`. Métriques = checker OpenFGA seulement. |
| 5 | AuthN / AuthZ IAM | **partiel** | JWT + `AuthUser`. OpenFGA seulement sur mark-read. Tuples `notification#recipient` non produits par Telegraph. Liste filtrée par `user_id`. |
| 6 | Contrats API | **partiel** | OpenAPI 3.1 aligné sur les 3 routes live. Pas de proto. Pas de `/v1`. DTOs send-message hors contrat. |
| 7 | Persistance | **conforme** | `DbConnectionPool` R/W, 1 migration + index, txn create+delivery. Pas d’outbox. |
| 8 | Messaging / events | **partiel** | Consumer rustycog + `iam-events` + command path. **Pas de publisher**. Health consumer non branché. Erreurs queue perdues. |
| 9 | Tests | **conforme** | Descriptor rustycog, prefix, DB/SQS/SMTP/OpenFGA testcontainers, `cache_ttl=0`. `has_sqs()==true` pour toute la suite. Pas de wiremock. |
| 10 | DI / composition root | **conforme** | Un `TelegraphApp::new`, `AppState`, `router` / `start\|stop_background_tasks`. Monolithe compose sans `run()`. |
| 11 | Patterns partagés | **partiel** | `/health`, shutdown `ctrl_c`, cache TTL 0 honoré. Pas de `/ready`. Pas de `RegistryConfig` retry. Health queue morte. |
| 12 | Alignement rustycog | **partiel** | Shell HTTP/setup/logger OK. Dettes : `command`, `ServiceError`, unwrap, events sortants, domain leaky. |

---

## 5. Détail par axe

### 5.1 Layout crate / boundaries — **conforme**

**Preuves**

- Bin : `Telegraph/src/main.rs` → `main` : `load_config`, `setup_logging`, `AppBuilder::new(config).build().run(server_config)`.
- Libs : `domain` (ports `Notification*Repository`, `TemplateService`, `EventExtractor`, `EventHandler`, communication) / `application` (`TelegraphCommandRegistryFactory`, usecases) / `infra` (email, tera, repos, `EventConsumer`, processors) / `http` / `setup` / `configuration` / `migration`.
- Hexagonale : HTTP et SQS → `GenericCommandService` → handlers commande → usecases → ports. GrepAI CLI : factory + `command/mod.rs` au centre.
- HTTP rustycog : `SERVICE_PREFIX = "/telegraph"`, `create_router`, `create_prefixed_router`, `create_app_routes` → `serve_router`. Trace GrepAI : même triangle que Hive / IAMRusty / Manifesto (`create_prefixed_router` L52, `TelegraphApp::router` L286).
- Monolithe : `monolith/src/runtime.rs` `AppBuilder` + `start_background_tasks` + `router()` ; `routes.rs` nest `SERVICE_PREFIX`.

**Écart**

- `Telegraph/domain/src/lib.rs` : `pub use iam_events::*;` (réexport `rustycog::events::*`) — le domain dépend de `rustycog-framework` `full` + du contrat IAM.
- `http/src/handlers/communication.rs` (`SendMessageRequest`, SMS, email) **non routé** (wiki + `create_router`).
- SMS : README / TOML historique vs `CommunicationConfig` (email + notification + template seulement).

### 5.2 Config / env / secrets — **partiel**

**Preuves**

- `TelegraphConfig` : `server`, `auth`, `logging`, `scaleway`, `queue`, `queues` (routing events), `communication`, `database`, `openfga`.
- Traits : `HasServerConfig`, `HasLoggingConfig`, `HasScalewayConfig`, `HasQueueConfig`, `HasDbConfig`, `HasOpenFgaConfig`. `config_prefix() -> "TELEGRAPH"`. `load_config()` → `load_config_fresh`.
- Split rustycog : transport `queue` vs métier `queues.*` + `event_configs.*.modes` (wiki *Runtime and Configuration*).
- `test.toml` : `openfga.cache_ttl_seconds = 0` (opt-out cache, consigne permission).

**Écart**

- **Pas de section `command`** / `HasCommandConfig` — rustycog-config + rustycog-command exigent `RegistryConfig` depuis `[command.retry]`. Factory : `CommandRegistryBuilder::new()` sans retry.
- `TelegraphApp::new` hardcode `resources/communication_descriptor` alors que `TemplateConfig.dir` est configurable.
- Secrets versionnés : `hs256_secret`, `password = "postgres"`, `secret_access_key = "test"` (`config/default.toml`, `development.toml`, `test.toml`).
- Wiki : README `8081` / `user-events` vs compose `8080` / files `telegraph-events` / `test-user-events`.

### 5.3 Erreurs — **divergent**

**Preuves**

- Domain : `DomainError` `thiserror` + helpers `is_recoverable` / `is_validation_error` (`domain/src/error.rs`). **Pas** `rustycog::core::DomainError` / `From` → `ServiceError`.
- `ProcessEventErrorMapper` : mapping partiel validation / business / infra ; `_` → infra (`unknown_domain_error`). `Unauthorized` / `NotificationNotFound` tombent dans `_`.
- Notification : `GetNotificationsErrorMapper` / `GetUnreadCountErrorMapper` / `MarkNotificationReadErrorMapper` → **toujours** `CommandError::infrastructure("INFRASTRUCTURE_ERROR", …)`.
- HTTP : handlers mappent `CommandError` à la main (`Validation` → 400, `Business` si `"Unauthorized"` / `"not found"` → 403/404). `HttpError` (`http/src/error.rs`) **zéro usage** hors définition.
- Queue : `TelegraphEventHandler::handle_event` wrappe **tout** `CommandError` en `ServiceError::infrastructure`.
- Setup / bin : `anyhow::Error`.

**Écart**

- Contredit `using-rustycog-core` : pas de `ServiceError::http_status_code()` / `is_retryable()` sur HTTP ; catégorie perdue sur le consumer (retries poison sur erreurs métier).
- Matching de strings HTTP (`message.contains`) — fragile.
- Prod `unwrap` : `setup/src/app.rs` L122/L129 (`event_configs.get`); `infra` email / processors / `notification_write` (`id.unwrap()`, `get().unwrap()`).

### 5.4 Observabilité — **partiel**

**Preuves**

- Bin : `config::setup_logging(&config)` — **conforme** `using-rustycog-logger` (un singleton, contrairement à Manifesto).
- `tracing` info/error/debug dans setup, consumer, processors, repos, templates, handlers.
- Consumer : champs `event_id`, `event_type`, `queue_name`.
- `MetricsPermissionChecker` autour du cache OpenFGA.

**Écart**

- Logs avec emojis (`📋`, `🎯`, `✅`, `❌`) — bruit, pas un schéma d’événements.
- Handlers : `tracing::error!("Failed to get notifications: {:?}", error)` interpolé, pas de `#[instrument]`.
- Pas de métriques HTTP / queue / SMTP hors checker.

### 5.5 AuthN / AuthZ IAM — **partiel**

**Preuves**

- `UserIdExtractor::new(config.auth)` ; `AppState::new(command_service, extractor, permission_checker)`.
- Chaîne OpenFGA : `OpenFgaPermissionChecker` → skip cache si `cache_ttl_seconds == 0` sinon `CachedPermissionChecker` (défaut 15s) → `MetricsPermissionChecker`. Conforme rustycog-permission.
- `create_router` : 3 routes `.authenticated()` ; mark-read `.with_permission_on(Permission::Write, "notification")`.
- `openfga/model.fga` : `type notification` — `recipient: [user]`, `read`/`write`/`administer`/`own` = recipient. UUID `{id}` = plus profond → OK middleware.
- Liste / unread : scoped `auth_user.user_id` dans la commande (pas de check FGA objet).
- Mark-read domaine : `Unauthorized` si pas owner (double check **si** la requête passe le middleware).

**Écart**

- rustycog : « every protected route » + `with_permission_on`. GET list/unread authentifiés **sans** permission FGA (filtre user only).
- Commentaire `create_router` + wiki : tuples `notification:{id}#recipient@user:{user_id}` écrits par **sentinel-sync** sur `NotificationCreated`. **Aucun publisher / outbox** dans Telegraph (recherche `NotificationCreated` = commentaire seulement). Middleware **fail-closed 403** si store vide → le fallback domaine n’est jamais atteint en prod.
- Tests : vrai testcontainer OpenFGA (pas `OpenFgaFixtures` wiremock recommandé) ; `cache_ttl=0` OK.
- AuthN HS256 partagé (`rustycog-dev-hs256-secret`) — même dette interop RS256 IAM que Manifesto.

### 5.6 Contrats API — **partiel**

**Preuves**

- HTTP JSON Axum. `Telegraph/openspecs.yaml` OpenAPI **3.1.0** : `GET /api/notifications`, `GET /api/notifications/unread-count`, `PUT /api/notifications/{id}/read`, `GET /health` — aligné `create_router`.
- Prefix `/telegraph` standalone + monolithe.

**Écart**

- Pas de proto / gRPC (**N/A** transport).
- Pas de version d’URL (`/v1`).
- Surface live = read-model only ; DTOs `communication.rs` hors spec (wiki : ne pas les prendre pour le chemin d’extension).
- OpenAPI décrit « SQS real-time » ; le contrat HTTP n’expose pas le processing.

### 5.7 Persistance — **conforme**

**Preuves**

- `DbConnectionPool::new(&config.database)` ; read repo / write repo / `CombinedNotificationRepositoryImpl`.
- Ports : `NotificationReadRepository`, `NotificationWriteRepository`.
- `NotificationWriteRepositoryImpl` : `begin` / `exec_with_returning` / `commit` / `rollback` sur create+delivery.
- Migration `m20250201_000001` : tables `notifications`, `notification_deliveries` + FK + index (`user_id`, `user_is_read`, `created_at`, `expires_at`, delivery status).
- Tests : `has_db()`, fixtures `notifications` / `notification_deliveries`, `run_migrations_up` / `down`.

**Écart**

- Pas d’UoW / outbox (pas de events sortants). `delete_expired_notifications` côté port, pas de job visible dans setup.
- Migrations hors process `run()` — pattern rustycog.

### 5.8 Messaging / events — **partiel**

**Preuves**

- `iam-events` : `IamDomainEvent` taggé `user_signed_up` / `user_email_verified` / `user_logged_in` / `password_reset_requested`, impl `DomainEvent` + `version()`.
- `EventConsumer::new` : `create_event_consumer_from_queue_config(&config.queue)` ; `start` enregistre `TelegraphEventHandler`.
- Handler : `ProcessEventCommand::new` → `command_service.execute` (une surface, consigne rustycog).
- `supports_event_type` : filtre `queues.*.events` ; sinon discard + log (pas de DLQ dédiée).
- `CompositeEventProcessor` : mapping config → modes `email` / `notification`. Dev : signup / password_reset → email ; `user_email_verified` → notification.
- Tests : `user_signup_event_test`, `user_email_verified_event_test` (publish SQS réel).
- IAMRusty produit ces events (`IAMRusty/infra/src/event_adapter.rs`, tests `signup_sqs`).

**Écart**

- `using-rustycog-events` : health transport au startup. `EventConsumer::health_check` **zéro référence**.
- Factories peuvent no-op : boot « OK » ≠ SQS live.
- Toutes les erreurs commande → infra (retries / ack faux).
- Pas de publisher Telegraph → pas de `NotificationCreated` → OpenFGA / sentinel-sync (P0 AuthZ).
- SMS / multi-channel annoncé, processors live = email + notification.

### 5.9 Tests — **conforme**

**Preuves**

- `TelegraphTestDescriptor` : `ServiceTestDescriptor`, `has_db` / `has_sqs` / `has_openfga` / `has_smtp`, migrations, modèle OpenFGA JSON.
- `setup_test_server` + `prefixed_url` = origin + `SERVICE_PREFIX` (consigne rustycog-testing).
- Suites : `notification_http_endpoints_test`, `notification_business_logic_test`, `notification_error_scenarios_test`, `user_signup_event_test`, `user_email_verified_event_test`.
- Fixtures : DB + **MailHog testcontainer** (skill testcontainers) + SQS + `TestOpenFga`.
- Unitaires locaux : `command/factory.rs`, `template_env_service.rs`.

**Écart**

- `has_sqs() == true` **toujours** — rustycog : HTTP-only devrait rester `false` + suite routing dédiée.
- OpenFGA = vrai container, pas `OpenFgaFixtures` wiremock (plus lourd, plus fidèle).
- Pas de fixture wiremock SMTP (MailHog à la place — acceptable).
- Peu de tests domain purs.

### 5.10 DI / composition root / lifecycle — **conforme**

**Preuves**

- Un root : `TelegraphApp::new` — SMTP, `DbConnectionPool`, repos, Tera, factory descriptors, processors, usecases, registry, `GenericCommandService`, `EventConsumer`, extractor, checker, `AppState`.
- `AppBuilder::build` → `new`. `run` : consumer + HTTP en parallèle, `select!` `ctrl_c` / fail, `stop_background_tasks` + abort.
- Monolithe : `router()` = `create_router` non préfixé ; `start_background_tasks` spawn `event_consumer.start` ; `stop` → `event_consumer.stop`.

**Écart**

- `unwrap` dans le root si `event_configs` incomplet (P0).
- Chemin descriptors non injectable.

### 5.11 Patterns partagés — **partiel**

**Preuves**

- `RouteBuilder::health_check()` → `/health` (OpenAPI).
- Shutdown gracieux consumer + abort HTTP.
- Cache OpenFGA skip si TTL 0.
- Commandes `validate()` + un `GenericCommandService`.

**Écart**

- Pas de `/ready` (DB / SQS / SMTP / OpenFGA).
- Health HTTP ≠ `EventConsumer::health_check` / `EmailAdapter::health_check` (port existe, non exposé).
- Pas de `RegistryConfig` / `[command.retry]` — `max_attempts = 0` inexploitable.
- Retry queue faussé par le wrap infra (axe 3).

### 5.12 Alignement rustycog — **partiel**

**Aligné**

- Umbrella `rustycog-framework` `full` / `test-utils`.
- `SERVICE_PREFIX`, `create_router` / `create_prefixed_router` / `create_app_routes` → `serve_router`.
- Une surface commande HTTP + queue ; checker unique ; TTL 0 honoré.
- `setup_logging` au boot standalone.
- Tests préfixés ; monolithe compose setup, pas `run()`.

**Anti-patterns / dettes**

| Item | Consigne rustycog |
|---|---|
| `DomainError` local + HTTP ad hoc | `using-rustycog-core` (`ServiceError`) |
| Mappers notif + wrap queue → infra | catégories / `is_retryable` |
| Pas de section `command` | `using-rustycog-command` / config |
| `unwrap` setup / infra | pas d’unwrap prod |
| Health queue non branchée | `using-rustycog-events` |
| Pas de `NotificationCreated` | permission + sentinel-sync |
| `pub use iam_events::*` dans domain | boundaries hexagonales |
| `has_sqs` toujours true | rustycog-testing (suites séparées) |
| Secret HS256 git | config/secrets |

---

## 6. Écarts priorisés

### P0

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P0-1 | Toute erreur commande queue → `ServiceError::infrastructure` | `TelegraphEventHandler::handle_event` (`infra/src/event/consumer.rs`) | Retry infini / poison sur validation ; ack/nack faux |
| P0-2 | `unwrap()` sur `event_configs.get(event_name)` au boot | `TelegraphApp::new` (`setup/src/app.rs` ~L122–129) | Panic si `queues.*.events` sans `event_configs` |
| P0-3 | `with_permission_on(Write, "notification")` sans events `NotificationCreated` | `http/src/lib.rs` ; aucun publisher ; wiki HTTP API | Mark-read **403 fail-closed** en prod ; fallback domaine injoignable |

### P1

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P1-1 | Mappers notification = toujours infra | `application/src/command/notification.rs` | 500 au lieu de 403/404 ; retry HTTP/command absurde |
| P1-2 | HTTP sans `ServiceError` + match de strings | `handlers/notification.rs` vs `using-rustycog-core` | Mapping dupliqué / fragile |
| P1-3 | `unwrap` infra (email, processors, `notification.id`) | `email.rs`, `processors/mod.rs`, `notification_write.rs` | Panic 500 runtime |
| P1-4 | Pas de `[command]` / `RegistryConfig` | `TelegraphConfig` ; factory `CommandRegistryBuilder::new()` | Retry non configurable (`max_attempts`) |
| P1-5 | Descriptors hardcodés | `TelegraphApp::new` `resources/communication_descriptor` | Casse hors CWD / monolithe |
| P1-6 | Secrets HS256 + MDP + AWS keys en git | `config/*.toml` | Fuite / tokens IAM non RS256 |
| P1-7 | `EventConsumer::health_check` mort | zéro référence Serena | No-op SQS invisible |

### P2

| ID | Écart | Preuve |
|---|---|---|
| P2-1 | `HttpError` mort | seulement `http/src/error.rs` |
| P2-2 | Handlers `communication` non routés | `handlers/communication.rs` vs `create_router` |
| P2-3 | SMS annoncé, pas dans `CommunicationConfig` / processors | wiki + struct |
| P2-4 | Domain `pub use iam_events::*` + rustycog `full` | `domain/src/lib.rs`, `domain/Cargo.toml` |
| P2-5 | `has_sqs()==true` pour toute la suite | `tests/common.rs` |
| P2-6 | Pas de `/ready` ; `/health` liveness only | `RouteBuilder.health_check` |
| P2-7 | Logs emoji / peu d’`instrument` | consumer, handlers |
| P2-8 | OpenAPI sans version d’URL ; drift ports/queues doc | `openspecs.yaml` ; wiki Open Questions |
| P2-9 | `ProcessEventErrorMapper` : `Unauthorized` / not_found → infra | `process_event.rs` `_` arm |

---

## 7. Forces

1. **Shell RustyCog solide** — crates hexagonaux, `AppState`, `RouteBuilder`, prefix, monolithe (`router` + background) alignés Manifesto / Hive / IAMRusty.
2. **Une surface commande** — HTTP notifications **et** consumer SQS passent par `GenericCommandService`.
3. **`setup_logging`** — mieux que le gabarit Manifesto (pas de subscriber maison).
4. **Contrat IAM vivant** — `iam-events` réellement consommé (signup, verify, password reset) + tests e2e SQS.
5. **AuthZ wiring moderne** — checker unique, cache TTL 0 honoré, `MetricsPermissionChecker`, type FGA `notification` correct.
6. **Persistance propre** — split R/W, transaction create+delivery, migration indexée.
7. **Tests d’intégration riches** — MailHog + LocalStack + Postgres + OpenFGA + JWT préfixé.

---

## 8. Dépendances IAM (hors code métier)

| Artefact | Lien Telegraph | Note |
|---|---|---|
| `iam-events` | `ProcessEventCommand`, consumer, tests | **vivant** (contrairement à Manifesto) |
| IAMRusty | producteur SQS (hors crate) | `user_signed_up`, `user_email_verified`, `password_reset_requested` |
| `AuthConfig` / JWT rustycog | `UserIdExtractor` | HS256 ; pas l’issuer IAM RS256 |
| `openfga/model.fga` | `with_permission_on(..., "notification")` | tuples attendus de sentinel-sync |
| `sentinel-sync` | sync FGA sur `NotificationCreated` | **bloqué** tant que Telegraph ne publie pas (P0-3) |

IAMRusty n’est pas dans le périmètre d’édition ; cité uniquement comme producteur d’events / tokens.
