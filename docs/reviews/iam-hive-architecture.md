# Revue d’architecture — Hive (RustyCog / IAM)

| Champ | Valeur |
|---|---|
| Date | 2026-08-29 |
| Cible | Service **Hive** (`hive-service`) — organisations / membres / invitations / liens externes / sync |
| Référentiel | RustyCog (skill + `references/`) + wiki QMD `aiforall-wiki` |
| Code métier | **non modifié** |

## 1. Périmètre

Hive n’est pas IAMRusty : c’est le service d’**organisation management** de la plateforme Rusty. Il vit sous `Hive/` et s’intègre à l’IAM via JWT partagé, OpenFGA (`organization`), `hive-events` + `sentinel-sync`, et un publisher multi-queue (outbox).

| Crate | Rôle |
|---|---|
| `hive-service` (`Hive/`) | Binaire + tests d’intégration |
| `hive-domain` | Entités, ports R/W, services domaine |
| `hive-application` | Use cases, commandes, DTO |
| `hive-infra` | SeaORM, client HTTP providers, outbox, event adapter |
| `hive-http` | `RouteBuilder`, handlers, `SERVICE_PREFIX=/hive` |
| `hive-configuration` | `AppConfig` typé, préfixe `HIVE` |
| `hive-setup` | Composition root `Application` / `AppBuilder` |
| `hive-migration` | 9 migrations SeaORM |
| `hive-events` | Contrat `HiveDomainEvent` (crate workspace) |

Dépendances locales liées IAM / plateforme : `hive-events` (publié, consommé par `sentinel-sync`), `openfga/model.fga` (type `organization`). Pas de dépendance crate vers `IAMRusty`. `iam_service` est déclaré en config et **jamais lu** hors `AppConfig`.

## 2. Méthode et limites outillage

| Outil | Statut |
|---|---|
| **Serena** | OK — `initial_instructions` + `activate_project` AIForAll ; overview / `find_symbol` / références / patterns. L’instance MCP a parfois basculé vers Alcoholic ; réactivation requise. |
| **GrepAI** (`user-grepai`) | Index MCP **hors sujet** (corpus Alcoholic Java, ~0,53 similarité). RPG off. **CLI locale** `grepai search` / `trace graph create_router` depuis AIForAll : utile (setup, router, sentinel-sync). |
| **Context Mode** | **Indisponible** — aucun namespace MCP `context-mode` / `ctx_*` dans le catalogue de session (pourtant déclaré dans `.cursor/mcp.json`) |
| **QMD** (CLI) | OK — collection `aiforall-wiki` (`projects/hive/*`, rustycog) ; wiki ~118 j., encore pertinent (drift routes/registry déjà noté) |
| **RustyCog** | Référentiel normatif (skill + `building-rustycog-services.md` + `using-rustycog-*.md`) |

## 3. Synthèse

Hive suit le **gabarit hexagonal RustyCog** (slice verticale, un composition root, `GenericCommandService`, `SERVICE_PREFIX=/hive`, APIs monolithe, `setup_logging`). C’est le producteur d’events organisations pour `sentinel-sync` / OpenFGA. Les écarts graves sont : **routes `/roles` live sans commandes enregistrées ni `RoleUseCase`**, **traduction sentinel-sync limitée à 3 events** (`OrganizationCreated`, `MemberJoined`, `MemberRemoved` — delete/update/roles = no-op), **outbox hors transaction métier**, et un **OpenAPI / handlers / registry** qui ne décrivent pas la même API. Contrairement à Manifesto, le bin utilise bien `rustycog-logger::setup_logging`.

**Verdict global : conforme sur le layout, partiel sur AuthZ/events/erreurs, divergent sur les contrats API** — pas une divergence de boundaries.

## 4. Tableau des 12 axes

| # | Axe | Verdict | Écart vs rustycog / consignes |
|---|---|---|---|
| 1 | Layout crate / boundaries | **conforme** | Hexagonale + ports R/W. Domain `pub use hive_events::*`. `domain/src/error.rs` mort (pas même `mod`). `_role_service` jeté. |
| 2 | Config / env / secrets | **partiel** | Préfixe `HIVE`, sections rustycog + `command` + `queue`. `iam_service` mort. Secrets HS256 / Postgres / AWS en git. Queue `sqs` par défaut. |
| 3 | Erreurs | **partiel** | `thiserror` + mappers `CommandError` aplatis. HTTP = match `"not found"`. `ServiceError` seulement events. `unwrap` prod. |
| 4 | Observabilité | **partiel** | `setup_logging` rustycog. Logs interpolés, pas de `#[instrument]`. Métriques = checker OpenFGA seulement. |
| 5 | AuthN / AuthZ IAM | **partiel** | JWT + `with_permission_on(..., "organization")` + cache TTL 0. GET/search/list sans FGA. Delete org sans tuple delete. Tables SQL rôles encore là. |
| 6 | Contrats API | **divergent** | OpenAPI 3.0.3 beaucoup plus large que `create_router`. Handlers invitations/update_member morts. `/roles` live mais registry vide. Pas de `/v1`. |
| 7 | Persistance | **partiel** | `DbConnectionPool` R/W, 9 migrations. Outbox = **seconde** txn après le write métier (pas d’UoW atomique). |
| 8 | Messaging / events | **partiel** | `hive-events` + publisher + outbox dispatcher. **Pas de consumer**. sentinel-sync ignore delete/update/roles. Queue `sqs` / no-op invisible. |
| 9 | Tests | **conforme** | Descriptor rustycog, prefix, OpenFGA **testcontainer réel**, `has_sqs=false` + suite SQS dédiée, outbox, wiremock fixture. Pas de tests roles/invitations HTTP. |
| 10 | DI / composition root | **conforme** | Un `Application::new` / `AppBuilder`, `AppState`, `router` / `start\|stop_background_tasks`. Monolithe compose sans `run()`. |
| 11 | Patterns partagés | **partiel** | `/health`, shutdown `ctrl_c`, cache TTL 0 honoré. Pas de `/ready`. Pas de `RegistryConfig`. Health queue absente. |
| 12 | Alignement rustycog | **partiel** | Shell HTTP/setup/logger OK. Dettes : roles registry, `ServiceError` HTTP, retry `command`, unwrap, events sortants incomplets. |

---

## 5. Détail par axe

### 5.1 Layout crate / boundaries — **conforme**

**Preuves**

- Bin : `Hive/src/main.rs` → `main` : `load_config`, `setup_logging`, `AppBuilder::new(config).build().run(server_config)`.
- Libs : `domain` (ports `Organization*Repository`, `OrganizationMember*`, invitations, providers, sync + `ExternalProviderClient`) / `application` (`HiveCommandRegistryFactory`, usecases) / `infra` (repos, `HttpExternalProviderClient`, `HiveOutboxUnitOfWorkImpl`, `HiveErrorMapper`) / `http` / `setup` / `configuration` / `migration`.
- Hexagonale : handlers → `command_service.execute` → handlers de commande → usecases → ports. Pas d’I/O dans le domain (hors `pub use hive_events::*`).
- Monolithe : `Application::router` → `create_router` (non préfixé) ; standalone : `create_prefixed_router` + `serve_router`.

**Écart**

- `Hive/domain/src/lib.rs` réexporte `hive_events::*` (et donc `rustycog::events`) — leaky domain, même motif Telegraph/`iam_events`.
- `Hive/domain/src/error.rs` (`use thiserror::Error` seul) n’est **pas** `mod` dans `lib.rs` — fichier mort.
- `setup_application` crée `_role_service` et le jette ; pas de crate/usecase `RoleUseCase` branché.

### 5.2 Config / env / secrets — **partiel**

**Preuves**

- `AppConfig` : `server`, `auth`, `database`, `iam_service`, `external_provider_service`, `logging`, `scaleway`, `command`, `queue`, `openfga` — traits `HasServerConfig`, `HasLoggingConfig`, `HasDbConfig`, `HasQueueConfig`, `HasScalewayConfig`, `HasOpenFgaConfig`.
- `config_prefix() -> "HIVE"` ; `load_config()` → `load_config_fresh::<AppConfig>()`.
- `external_provider_service` consommé (`HttpExternalProviderClient::new` dans `setup_infra`).
- `test.toml` : `openfga.cache_ttl_seconds = 0` (consigne rustycog-permission / tests).

**Écart**

- `Hive/config/default.toml` : `auth.jwt.hs256_secret = "rustycog-dev-hs256-secret"`, `password = "postgres"`, `secret_access_key = "test"` versionnés.
- `IamServiceConfig` (`base_url`, `api_key`, `timeout_seconds`) : **zéro lecture runtime** (Serena : seulement `AppConfig` + `Default`).
- Pas de `HasCommandConfig` ; `[command] max_attempts` n’est pas passé à `CommandRegistryBuilder` (wiki QMD *Command Execution* `^[ambiguous]`).
- Queue checked-in : `type = "sqs"` (pas `disabled`) — pitfall rustycog : factory peut dégrader en no-op sans health.

### 5.3 Erreurs — **partiel**

**Preuves**

- `ApplicationError` (`thiserror`) wrappe `rustycog::core::error::DomainError`.
- Mappers par agrégat (`OrganizationErrorMapper`, `MemberErrorMapper`, …) : Domain → `CommandError::business`, validation → `validation`, reste → `infrastructure` / `business`.
- `SyncJobErrorMapper` : **tout** en `infrastructure` (même motif Telegraph).
- `HiveErrorMapper` (`infra/src/event/event_adapter.rs`) : `DomainError` ↔ `ServiceError` (chemin events seulement).
- `HttpError` + `error_mapper(CommandError)` dans organizations / sync_jobs : validation 400, business + `"not found"` → 404 sinon 400, infra/retry → 500.

**Écart**

- Chemin HTTP : `CommandError` → `HttpError`, **pas** `ServiceError` / `http_status_code()` (consigne `using-rustycog-core`).
- `HttpError::Application` + mapping fin `DomainError` dans `IntoResponse` : **non utilisé** par les handlers (ils passent par `error_mapper` / `HttpError::Internal`).
- Invitation stubs : `list_*` / `cancel` / `get_by_token` / `resend` → `CommandError::business("…_not_implemented")`.
- Prod : `MemberUseCaseImpl::member_to_response` — `member.id.unwrap()` ; `ExternalLinkUseCaseImpl` — `provider_source.clone().unwrap()` ; domain `member_service` / `invitation_service` / `sync_service` / `external_provider_service` ; repos SeaORM `result.*.unwrap()` systématiques.
- Setup / run : `anyhow::Error` (acceptable en composition root).

### 5.4 Observabilité — **partiel**

**Preuves**

- Bin : `hive_configuration::setup_logging` = réexport `rustycog::logger::setup_logging` (mieux que Manifesto).
- `tracing::info!` / `error!` dans handlers, setup, outbox rollback, client provider.
- `MetricsPermissionChecker` autour d’OpenFGA.

**Écart**

- Pas de `#[instrument]`.
- Logs handlers interpolés (`"Getting organization: {}"`) plutôt que champs structurés.
- Pas de métriques métier / HTTP / queue / outbox hors permissions.

### 5.5 AuthN / AuthZ IAM — **partiel**

**Preuves**

- `UserIdExtractor::new(config.auth)` ; `AppState::new(command_service, user_id_extractor, permission_checker)`.
- Chaîne OpenFGA : `OpenFgaPermissionChecker` → skip `CachedPermissionChecker` si `cache_ttl_seconds == 0` → `MetricsPermissionChecker`. Conforme rustycog-permission / tests.
- `RouteBuilder` : `.authenticated()` / `.might_be_authenticated()` puis `.with_permission_on(Permission::{Read,Write,Admin}, "organization")` pour update/delete/members/invitations/links/sync/roles.
- `openfga/model.fga` : `type organization` (`owner` / `admin` / `member` / `viewer` / `read` / `write` / `administer` / `own`).
- Tests : JWTs rustycog, **vrai** testcontainer OpenFGA (`TestOpenFga`), tuples arrangés dans `organization_api_tests` / `members_api_tests`.
- Wiki : plus de `permissions/*.conf` ni `ResourcePermissionFetcher` — AuthZ HTTP = Check OpenFGA uniquement.

**Écart**

- `GET /search` et `GET /{id}` : `might_be_authenticated` **sans** `with_permission_on` — lecture publique (test `search_organizations_is_public_and_returns_results`).
- `GET /api/organizations` (liste) : authenticated **sans** FGA — pas d’object UUID (middleware skip).
- `POST /api/organizations` : authenticated seulement (création = premier write) ; l’owner OpenFGA dépend de `OrganizationCreated` + sentinel-sync.
- `sentinel-sync/src/translator/hive.rs` : seuls `OrganizationCreated`, `MemberJoined`, `MemberRemoved` écrivent/suppriment des tuples. `OrganizationDeleted` / `Updated` / invitations / links / sync / `MemberRolesUpdated` → `TupleDelta::default()` (**no-op**). Pitfall rustycog : event sans bras → OpenFGA silencieux.
- Persistance SQL `permissions` / `role_permissions` / `organization_member_role_permissions` **en plus** d’OpenFGA — RBAC métier SQL vs AuthZ HTTP.
- AuthN = HS256 partagé, pas le chemin RS256 IAM.

### 5.6 Contrats API — **divergent**

**Preuves**

- HTTP JSON Axum. `openspecs.yaml` OpenAPI 3.0.3, `info.version: 1.0.0`.
- Live `create_router` : search/get/create/update/delete/list orgs, start sync-job, list/get roles, add/remove/list/get members, create invitation, create external-link.
- `SERVICE_PREFIX = "/hive"` — contrat standalone / monolithe.
- GrepAI `trace graph create_router` : `health_check` → routes → `create_prefixed_router` / `Application::router`.

**Écart**

- Pas de proto / gRPC (**N/A** transport).
- Spec vs live : `{orgId}/external-links/{linkId}`, `…/sync`, `sync-jobs/{jobId}`, `/api/invitations/{token}` (+ accept/decline), CRUD rôles — **absents** de `create_router` (wiki *HTTP API and OpenAPI Drift*).
- Handlers présents mais **non routés** : `list_invitations`, `get_invitation`, `accept_invitation`, `get_invitation_by_token`, `update_member`.
- Inverse : `GET …/roles` **routé** mais `ListRolesCommand` / `GetRoleCommand` **non enregistrés** dans `create_hive_registry` et **pas** de `RoleUseCase` application — 500 garanti (wiki *Command Execution*).
- Tests : create → 200 (spec 201), delete → 200 (spec 204), pagination `page`/`page_size` vs spec `cursor`/`limit`.
- Pas de `/v1` ; version seulement dans le spec.

### 5.7 Persistance — **partiel**

**Preuves**

- `DbConnectionPool::new` + replicas logguées (`setup_database`).
- Ports read/write séparés (orgs, members, invitations, providers, links, sync, roles, permissions, resources).
- Migrations `m20240101_000001` … `000012` (orgs, members, invitations, providers, links, sync_jobs, permissions, resources, role_permissions, member_role_permissions).
- `HiveOutboxUnitOfWorkImpl` + `OutboxRecorder` ; tests `outbox_tests.rs`.
- Harness : `run_migrations_up` / `down`, `has_db() == true`.

**Écart**

- `record_event` ouvre sa **propre** write txn **après** `create_organization` (et équivalents). Pas d’UoW unique agrégat + outbox (contrairement à Manifesto `ProjectCreationUnitOfWorkImpl`). Dual-write : org persistée puis échec outbox → 500 + pas d’event / pas de tuple owner.
- `setup_database` ne joue pas les migrations (crate `hive-migration` à part) — pattern rustycog, pas un défaut.

### 5.8 Messaging / events — **partiel**

**Preuves**

- `HiveDomainEvent` impl `DomainEvent` (org / member / invitation / external_link / sync).
- Publisher : `create_multi_queue_event_publisher` + `HiveErrorMapper`.
- Usecases : `record_or_publish_event` — outbox si branché, sinon `publish` avec `map_err` (**pas** swallow warn-only comme Manifesto).
- `OutboxDispatcher` toujours démarré (`start_background_tasks`).
- Tests : `outbox_tests`, `sqs_event_routing_tests` (`HiveSqsTestDescriptor`, `has_sqs() == true`).
- `sentinel-sync` dépend de `hive-events`.

**Écart**

- Pas de consumer Hive (wiki : contrairement à Telegraph).
- sentinel-sync : 8/11 variants = no-op — delete org / update rôles ne nettoient pas OpenFGA.
- Queue `sqs` par défaut ; pas de `health_check` publisher/dispatcher sur `/health` (pitfall « factory no-op »).
- `MemberRolesUpdated` dans `hive-events` : pas de chemin de publish trouvé dans les usecases application.

### 5.9 Tests — **conforme**

**Preuves**

- `HiveTestDescriptor` : `ServiceTestDescriptor`, `build_app` / `run_app`, migrations, `openfga_authorization_model_json` = `openfga/model.json`.
- `setup_test_server` → URL suffixée `SERVICE_PREFIX` ; OpenFGA testcontainer **réel** (pas seulement mock).
- `has_db() == true`, `has_sqs() == false`, `has_openfga() == true` — SQS isolé dans `sqs_event_routing_tests` (mieux que Telegraph `has_sqs==true` partout).
- Suites : `organization_api_tests`, `members_api_tests`, `external_link_api_tests`, `outbox_tests`, `sqs_event_routing_tests`.
- Fixtures : `DbFixtures`, `ExternalProviderFixtures` (`MockServerFixture` / wiremock), JWT rustycog.
- Unitaires : `#[cfg(test)]` VO org/sync_job + factory.

**Écart**

- Aucun `roles_api_tests` / `invitation_api_tests` — le P0 registry `/roles` n’est pas couvert.
- `tests/common.rs` : `unsafe { APP.replace(app) }` static globale (harness rustycog, mais fragile).
- Wiremock provider : fixture présente, les API tests externes s’appuient surtout sur la DB seed.

### 5.10 DI / composition root / lifecycle — **conforme**

**Preuves**

- Un root : `Application::new` — DB, publisher, outbox dispatcher, usecases+UoW, registry, `GenericCommandService`, extractor, checker, `AppState`.
- `run` : HTTP + outbox `JoinSet` + `ctrl_c` + `stop_background_tasks`.
- APIs monolithe : `router()`, `start_background_tasks()`, `stop_background_tasks()` — `monolith/src/runtime.rs` compose `hive_setup::AppBuilder` sans `run()`.

**Écart**

- `_role_service` non injecté dans le registry (voir 5.6).
- Pas d’écart rustycog majeur sur le lifecycle monolithe.

### 5.11 Patterns partagés — **partiel**

**Preuves**

- `RouteBuilder::health_check()`.
- Shutdown gracieux (abort HTTP + outbox).
- Cache OpenFGA skip si TTL 0.
- Commandes `validate()` + un `GenericCommandService`.

**Écart**

- Pas de readiness (DB / queue / OpenFGA).
- `CommandRegistryBuilder::new()` **sans** `RegistryConfig::from_retry_config` — `max_attempts` TOML inerte (`max_attempts = 0` en dev/test n’a aucun effet ; default `3` non appliqué).
- Health HTTP ≠ publisher / outbox / SQS.

### 5.12 Alignement rustycog — **partiel** (avec dettes listées)

**Aligné**

- `rustycog-framework` `features = ["full"]` / `test-utils`.
- `SERVICE_PREFIX`, `create_router` / `create_prefixed_router` / `create_app_routes` → `serve_router`.
- Une surface commande HTTP ; permission centralisée ; cache TTL 0 honoré.
- `setup_logging` au boot standalone (référentiel `^[ambiguous]` vs Manifesto).
- Tests préfixés ; monolithe compose setup, pas `run()`.

**Anti-patterns / dettes**

| Item | Consigne rustycog |
|---|---|
| `/roles` sans `register` | une surface commande ; `command_type()` = clé registry |
| HTTP sans `ServiceError` | `using-rustycog-core` |
| Pas de `RegistryConfig` | `using-rustycog-command` / `[command.retry]` |
| Outbox hors txn métier | events + outbox atomique |
| sentinel-sync no-op sur delete | event sans bras → OpenFGA silencieux |
| `unwrap` IDs / `Option` | pas d’unwrap prod |
| `pub use hive_events::*` dans domain | boundaries hexagonales |
| Secret HS256 git | config/secrets |
| Queue `sqs` sans health | factory no-op |

---

## 6. Écarts priorisés

### P0

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P0-1 | Routes `GET …/roles` live, commandes **non** dans `create_hive_registry`, pas d’impl `RoleUseCase` | `http/src/lib.rs` L49–58 ; `command/factory.rs` `create_hive_registry` ; `usecase/mod.rs` sans `role` ; wiki *Command Execution* | **500** sur list/get roles ; CI ne le voit pas (pas de `roles_api_tests`) |
| P0-2 | `OrganizationDeleted` (et update/roles/invites/links) = `TupleDelta` vide dans sentinel-sync | `sentinel-sync/src/translator/hive.rs` L114–122 ; `openfga/model.fga` | Tuples owner/member **orphelin** après delete/changement de rôle ; AuthZ OpenFGA diverge |
| P0-3 | Dual-write : persist métier **puis** outbox dans une autre txn | `OrganizationUseCaseImpl::create_organization` ; `HiveOutboxUnitOfWorkImpl::record_event` | Org créée + API 500 + pas d’`OrganizationCreated` → pas de `#owner` → **403** sur toutes les routes Admin/Write |

### P1

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P1-1 | OpenAPI / handlers / router / registry désalignés | `openspecs.yaml` vs `create_router` vs `handlers/invitations.rs` | Contrat client mensonger ; accept invitation implémenté mais **non exposé** |
| P1-2 | HTTP sans `ServiceError` + match de strings `"not found"` | `handlers/organizations.rs` `error_mapper` | 400/404/500 fragiles ; mapping `HttpError::Application` mort |
| P1-3 | `unwrap` usecase/domain/repos | `usecase/member.rs` L117 ; `usecase/external_link.rs` L136 ; `invitation_service` / repos `result.id.unwrap()` | Panic 500 |
| P1-4 | `[command]` non branché (`RegistryConfig`) | `factory.rs` `CommandRegistryBuilder::new()` ; `AppConfig.command` | Retry inerte ; `max_attempts = 0` en test n’a pas le sens rustycog |
| P1-5 | Secrets HS256 + MDP + AWS keys en git ; IAM RS256 à part | `config/default.toml` ; wiki runtime | Fuite / interop tokens IAM |
| P1-6 | Queue `sqs` par défaut + pas de health | `default.toml` `[queue]` ; `/health` liveness only | Publisher no-op invisible ; outbox « OK » sans delivery |
| P1-7 | Double source SQL rôles + OpenFGA | migrations `*_role_permissions` + Check FGA | Incohérence ACL métier vs HTTP |

### P2

| ID | Écart | Preuve |
|---|---|---|
| P2-1 | `domain/src/error.rs` mort (pas de `mod`) | zéro export `lib.rs` |
| P2-2 | `iam_service` config morte | seulement `AppConfig` / TOML |
| P2-3 | Handlers invitations / `update_member` non routés | `handlers/*.rs` vs `create_router` |
| P2-4 | Domain `pub use hive_events::*` | `domain/src/lib.rs` |
| P2-5 | Pas de `/ready` ; health queue/outbox absent | `RouteBuilder.health_check` only |
| P2-6 | Métriques limitées au checker | `MetricsPermissionChecker` |
| P2-7 | Logs handlers peu structurés | `handlers/*.rs` |
| P2-8 | `unsafe` static `APP` dans les tests | `tests/common.rs` |
| P2-9 | Invitation commands stub (`*_not_implemented`) | `command/invitation.rs` |
| P2-10 | Lecture publique org/search sans FGA | `might_be_authenticated` seul |

---

## 7. Forces

1. **Shell RustyCog solide** — crates hexagonaux, `AppState`, `RouteBuilder`, prefix `/hive`, monolithe (`router` + background) alignés Manifesto / Telegraph / IAMRusty.
2. **Une surface commande HTTP** — handlers passent par `GenericCommandService` (quand la commande est bien enregistrée).
3. **`setup_logging`** — mieux que le gabarit Manifesto (pas de subscriber maison).
4. **AuthZ wiring moderne** — checker unique, cache TTL 0 honoré, `MetricsPermissionChecker`, object type `"organization"` cohérent avec le middleware UUID.
5. **Events typés + outbox dispatcher** — `hive-events` vivant (contrairement à `iam-events` mort chez Manifesto) ; publish **ne swallow pas** l’erreur.
6. **Tests d’intégration sérieux** — Postgres + OpenFGA **réel** + JWT préfixé ; SQS isolé (`has_sqs=false`) ; outbox dédié.
7. **Ports hexagonaux clairs** — read/write repos + `ExternalProviderClient`.

---

## 8. Dépendances IAM (hors code métier)

| Artefact | Lien Hive | Note |
|---|---|---|
| `AuthConfig` / JWT rustycog | `UserIdExtractor` | HS256 ; pas l’issuer IAM RS256 |
| `openfga/model.fga` | `with_permission_on(..., "organization")` | membres / links / sync = relations dérivées sur l’org |
| `sentinel-sync` | `translator/hive.rs` | critique si P0-2 / P0-3 |
| `hive-events` | usecases + sentinel-sync | contrat **utilisé** |
| `iam_service` TOML | `AppConfig` | **mort** — pas de client IAMRusty |

IAMRusty n’est pas dans le périmètre d’édition ; cité uniquement comme producteur de tokens / (non) consommateur `iam_service`.
