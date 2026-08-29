# Revue d’architecture — Manifesto (RustyCog / IAM)

| Champ | Valeur |
|---|---|
| Date | 2026-08-29 |
| Cible | Service **Manifesto** (`manifesto-service`) — orchestration de projets / composants / membres |
| Référentiel | RustyCog (skill + `references/`) + wiki QMD `aiforall-wiki` |
| Code métier | **non modifié** |

## 1. Périmètre

Manifesto est le service de référence RustyCog (pas un crate `iam-manifesto`). Il vit sous `Manifesto/` et s’intègre à l’IAM via le contrat d’auth partagé (`AuthConfig` / JWT), OpenFGA, `sentinel-sync`, et les events domaine.

| Crate | Rôle |
|---|---|
| `manifesto-service` (`Manifesto/`) | Binaire + tests d’intégration |
| `manifesto-domain` | Entités, ports, VO |
| `manifesto-application` | Use cases, commandes, DTO |
| `manifesto-infra` | SeaORM, adapters HTTP, events, outbox UoW |
| `manifesto-http_server` | `RouteBuilder`, handlers, `SERVICE_PREFIX` |
| `manifesto-configuration` | `ManifestoConfig` typé |
| `manifesto-setup` | Composition root `Application` |
| `manifesto-migration` | Migrations SeaORM |
| `manifesto-events` | Contrat `ManifestoDomainEvent` |

Dépendances locales liées IAM / plateforme : `iam-events` (déclarée, **non utilisée** dans le Rust Manifesto), `apparatus-events` (consumer), `openfga/model.fga` (types `project` / `component`).

## 2. Méthode et limites outillage

| Outil | Statut |
|---|---|
| **Serena** | OK — `initial_instructions` + `activate_project` AIForAll ; overview / `find_symbol` / références / patterns |
| **GrepAI** (`user-grepai`) | Index MCP **hors sujet** (corpus Alcoholic Java, ~0,53 similarité). RPG désactivé. Traces `create_router` vides. **Non fiable pour ce repo.** |
| **Context Mode** | **Indisponible** — aucun namespace MCP `context-mode` / `ctx_*` dans le catalogue de session |
| **QMD** (CLI) | OK — collection `aiforall-wiki` (wiki ~118 j., utile mais pas à jour à 100 %) |
| **RustyCog** | Référentiel normatif (skill + `references/using-rustycog-*.md` + `building-rustycog-services.md`) |

## 3. Synthèse

Manifesto est le **gabarit RustyCog le plus aligné** du monorepo : hexagonale, un composition root, `GenericCommandService`, `RouteBuilder` + OpenFGA, prefix `/manifesto`, harness `rustycog-testing` (testcontainers + wiremock + OpenFGA mock). Les écarts sont surtout : logging hand-rolled, `unwrap` prod sur les ACL membres, events publiés « best-effort » (risque de dérive OpenFGA via `sentinel-sync`), outbox limité à la création de projet, `DomainError` local mort, secret HS256 commité, `iam-events` mort, OpenAPI incomplet.

**Verdict global : conforme avec écarts P0/P1 ciblés** — pas une divergence de layout.

## 4. Tableau des 12 axes

| # | Axe | Verdict | Écart vs rustycog / consignes |
|---|---|---|---|
| 1 | Layout crate / boundaries | **conforme** | Slice verticale + ports/adapters. `domain/src/error.rs` mort (réexport rustycog). |
| 2 | Config / env / secrets | **partiel** | Sections rustycog + préfixe `MANIFESTO`. Secrets HS256 + MDP Postgres en git. Wiki : vérifieur HS256 ≠ émission RS256 IAM. |
| 3 | Erreurs | **partiel** | `thiserror` + mapping `CommandError` → HTTP. Pas de `ServiceError` sur le chemin HTTP. `unwrap()` prod. Setup en `anyhow`. |
| 4 | Observabilité | **partiel** | `tracing` partout. Subscriber hand-rolled (pas `rustycog-logger::setup_logging`). Métriques = wrapper permissions seulement. |
| 5 | AuthN / AuthZ IAM | **partiel** | JWT + `with_permission_on` + OpenFGA + cache TTL 0 en test. Routes composant sur `"project"`. Tables ACL SQL encore là. Lecture anonyme publique incomplète (wiki). |
| 6 | Contrats API | **partiel** | OpenAPI 3.0.3 `openspecs.yaml` v1.0.0. Pas de proto/gRPC. Route `/details` absente du spec. Pas de version d’URL. |
| 7 | Persistance | **conforme** | `DbConnectionPool`, read/write ports, 9 migrations, UoW création projet. |
| 8 | Messaging / events | **partiel** | `manifesto-events` + multi-queue + consumer Apparatus. Publish swallow + outbox partiel. `iam-events` unused. Queue `disabled` par défaut (OK). |
| 9 | Tests | **conforme** | Descriptor rustycog, prefix, OpenFGA mock, wiremock catalog, SQS routing, ACL, transactions. |
| 10 | DI / composition root | **conforme** | Un `Application`, `AppState`, `router` / `start|stop_background_tasks` pour le monolithe. |
| 11 | Patterns partagés | **partiel** | `/health`, shutdown `ctrl_c`, retry commandes, no-op queue géré. Pas de `/ready`. Health consumer non exposé HTTP. |
| 12 | Alignement rustycog | **conforme** | Référence de scaffolding. Écarts connus : logging `^[ambiguous]`, `ServiceError` non utilisé côté HTTP. |

---

## 5. Détail par axe

### 5.1 Layout crate / boundaries — **conforme**

**Preuves**

- Bin : `Manifesto/src/main.rs` → `main` charge config, log, `Application::new`, `run`.
- Libs : `domain` (ports `Project*Repository`, `ComponentServicePort`) / `application` (usecases + `ManifestoCommandRegistryFactory`) / `infra` (adapters, repos, `event`, `transaction`) / `http` / `setup` / `configuration` / `migration`.
- Hexagonale : handlers → `command_service.execute` → handlers de commande → usecases → ports. Pas d’I/O dans le domain.
- Monolithe : `Application::router` délègue à `create_router` (non préfixé) ; standalone : `create_prefixed_router` + `serve_router`.

**Écart**

- `Manifesto/domain/src/lib.rs` réexporte `rustycog::core::error::DomainError` ; `Manifesto/domain/src/error.rs` (`ValidationError` / `NotFound` / …) n’a **aucune référence** (code mort).
- `iam-events` est une path-dep de `manifesto-service` sans usage Rust.

### 5.2 Config / env / secrets — **partiel**

**Preuves**

- `ManifestoConfig` : `server`, `auth`, `logging`, `command`, `queue`, `database`, `scaleway`, `service`, `openfga` — traits `HasServerConfig`, `HasLoggingConfig`, `HasQueueConfig`, `HasDbConfig`, `HasOpenFgaConfig`, `HasScalewayConfig`.
- `config_prefix() -> "MANIFESTO"` ; `load_config_fresh::<ManifestoConfig>()`.
- `service.component_service` (`base_url`, `api_key`, timeout) et `service.business.*` (quotas, pagination) consommés (wiki QMD *Runtime and Configuration*).
- Queue checked-in : `type = "disabled"` (volontaire).

**Écart**

- `Manifesto/config/default.toml` : `auth.jwt.hs256_secret = "rustycog-dev-hs256-secret"` et `password = "postgres"` versionnés.
- Wiki : IAM peut émettre en RS256 ; le vérifieur partagé Manifesto est **HS256-only**.
- `configuration` réexporte `rustycog::logger::setup_logging` mais le bin appelle `manifesto_setup::setup_logging` (subscriber local). Conflit rustycog `^[ambiguous]`.

### 5.3 Erreurs — **partiel**

**Preuves**

- `ApplicationError` (`thiserror`) + `From<ApplicationError> for CommandError` (variants rustycog `DomainError`).
- `HttpError` + `error_mapper(CommandError)` : validation 422, authz 403, not_found 404, conflict 409, infra/retry/timeout 500.
- Events : `ManifestoErrorMapper` impl `ErrorMapper<DomainError>` → `ServiceError` (chemin queue, pas HTTP).

**Écart**

- Chemin HTTP : `ApplicationError` → `CommandError` → `HttpError`, **pas** `ServiceError` / `http_status_code()` (consigne `using-rustycog-core`).
- Prod : `MemberUseCaseImpl` — `.id.unwrap()` / `role_permission.id.unwrap()` (`member.rs` L209, L327, L464, L525). Infra : `project_member_role_permission_repository.rs` L47.
- Setup / run : `anyhow::Error` (acceptable en composition root, pas typé).
- `HttpError` sans `thiserror` (enum ad hoc).

### 5.4 Observabilité — **partiel**

**Preuves**

- `tracing::info!` / `warn!` / `error!` dans handlers, usecases, repos, consumer, setup.
- `MetricsPermissionChecker` autour d’OpenFGA.

**Écart**

- `setup/src/config.rs` : `tracing_subscriber` fmt + `EnvFilter` — **pas** `rustycog-logger::setup_logging` (Loki / filter unifié).
- Pas de métriques métier / HTTP / queue hors permissions.
- Logs handlers souvent interpolés (`"Getting project: {}"`) plutôt que champs structurés.

### 5.5 AuthN / AuthZ IAM — **partiel**

**Preuves**

- `UserIdExtractor::new(config.auth)` ; `AppState::new(command_service, user_id_extractor, permission_checker)`.
- Chaîne OpenFGA : `OpenFgaPermissionChecker` → `CachedPermissionChecker` (skip si `cache_ttl_seconds == 0`) → `MetricsPermissionChecker`. Conforme rustycog-permission / tests.
- `RouteBuilder` : `.authenticated()` / `.might_be_authenticated()` puis `.with_permission_on(Permission::{Read,Write,Admin,Owner}, "project")`.
- `openfga/model.fga` : types `user`, `organization`, `project`, `component`.
- Tests : JWTs rustycog, `OpenFgaMockService`, `public_acl_api_tests`, `component_acl_consistency_tests`.

**Écart**

- Routes composant / membre : object type `"project"` (segment `{component_type}` non-UUID) — **documenté** et aligné middleware ; type FGA `component` non utilisé par le HTTP tant que pas de `{component_id}`.
- Persistance SQL permissions / `role_permissions` / `project_member_role_permissions` **en plus** d’OpenFGA — double source, risque de divergence.
- Wiki : `viewer@user:*` pas encore écrit par `sentinel-sync` → lecture anonyme d’un projet **403 en prod**.
- AuthN = HS256 partagé, pas le chemin RS256 IAM.

### 5.6 Contrats API — **partiel**

**Preuves**

- HTTP JSON Axum. `openspecs.yaml` OpenAPI 3.0.3, `info.version: 1.0.0`.
- Paths spec : `/api/projects`, `/{projectId}`, publish, archive, components, members, permissions.
- `SERVICE_PREFIX = "/manifesto"` — contrat standalone / monolithe.

**Écart**

- Pas de proto / gRPC (**N/A** transport, mais pas de contrat IDL hors OpenAPI).
- `GET /api/projects/{id}/details` dans `create_router` + tests, **absent** de `openspecs.yaml`.
- Pas de `/v1` ; version seulement dans le spec.

### 5.7 Persistance — **conforme**

**Preuves**

- `DbConnectionPool::new(&config.database)` + replicas logguées.
- Ports read/write séparés (projects, components, members, permissions, resources, roles).
- Migrations `m20241015_000001` … `000009` (tables + seed + drop unique index).
- `ProjectCreationUnitOfWorkImpl` + `OutboxRecorder` ; tests `transaction_readiness_tests.rs`.
- Harness : `run_migrations_up` / `down`, `has_db() == true`.

**Écart**

- `setup_database` ne joue pas les migrations (crate `manifesto-migration` à part) — pattern rustycog, pas un défaut.

### 5.8 Messaging / events — **partiel**

**Preuves**

- `ManifestoDomainEvent` impl `DomainEvent` (`project` / `component` / `member`).
- Publisher : `create_multi_queue_event_publisher` (injectable en test).
- Consumer : `ApparatusEventConsumer` + `ComponentStatusProcessor` ; no-op → pas de tâche de fond.
- `OutboxDispatcher` toujours démarré ; UoW outbox sur **création de projet** seulement.
- Tests : `event_runtime_tests`, `sqs_event_routing_tests` (référence rustycog-testing).

**Écart**

- Usecases : `publish(...).await` + `tracing::warn!` — **échec ignoré**. Pitfall rustycog : event sans bras `sentinel-sync` → OpenFGA silencieux ; ici l’event peut même ne pas partir.
- Outbox non généralisé (update/delete/member/permission = fire-and-forget).
- `iam-events` : dep morte (contrairement à Telegraph / IAMRusty / Hive / sentinel-sync).
- Health `ApparatusEventConsumer::health_check` non branché sur `/health`.

### 5.9 Tests — **conforme**

**Preuves**

- `ManifestoTestDescriptor` : `ServiceTestDescriptor`, `build_and_run`, migrations, OpenFGA JSON.
- `setup_test_server` → URL déjà suffixée `SERVICE_PREFIX` ; `test.toml` `openfga.cache_ttl_seconds = 0`.
- Suites : `project_api_tests`, `component_api_tests`, `member_api_tests`, `public_acl_api_tests`, `component_acl_consistency_tests`, `component_service_client_tests` (wiremock), `event_runtime_tests`, `sqs_event_routing_tests`, `transaction_readiness_tests`.
- Fixtures : `DbFixtures`, `ComponentServiceMockService` (skill wiremock), OpenFGA mock.

**Écart**

- Peu de tests unitaires domain purs hors `#[cfg(test)]` sur les VO. Acceptable : le contrat est surtout HTTP + ACL.

### 5.10 DI / composition root / lifecycle — **conforme**

**Preuves**

- Un root : `Application::new_with_maybe_event_publisher` — DB, publisher, UoW, usecases, registry, `GenericCommandService`, extractor, checker, `AppState`.
- `run` : HTTP + `JoinSet` background + `ctrl_c` + `stop_background_tasks` (outbox + consumer).
- APIs monolithe : `router()`, `start_background_tasks()`, `stop_background_tasks()` — pas de `run()` imbriqué.

**Écart**

- Aucun écart rustycog majeur.

### 5.11 Patterns partagés — **partiel**

**Preuves**

- `RouteBuilder::health_check()`.
- Shutdown gracieux (abort des tasks).
- `CommandRegistryBuilder::with_config(RegistryConfig::from_retry_config(&command_config.retry))` — `max_attempts = 0` désactive (consigne).
- Consumer no-op logué, pas de crash au boot.

**Écart**

- Pas de readiness (DB / queue / OpenFGA).
- Health HTTP ≠ `consumer.health_check()` (pitfall « factory no-op »).

### 5.12 Alignement rustycog — **conforme** (avec dettes listées)

**Aligné**

- `rustycog-framework` `features = ["full"]` / `test-utils`.
- `SERVICE_PREFIX`, `create_router` / `create_prefixed_router` / `create_app_routes` → `serve_router`.
- Une surface commande ; permission centralisée ; cache TTL 0 honoré.
- Tests préfixés ; SQS routing dédié.

**Anti-patterns / dettes**

| Item | Consigne rustycog |
|---|---|
| `tracing_subscriber` maison | `setup_logging` unique (`using-rustycog-logger`) |
| HTTP sans `ServiceError` | `using-rustycog-core` |
| Publish warn-only | events + sync OpenFGA |
| `unwrap` IDs ACL | pas d’unwrap prod |
| `DomainError` local mort | un seul type domaine |
| Secret HS256 git | config/secrets |

---

## 6. Écarts priorisés

### P0

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P0-1 | `unwrap()` sur IDs de permissions en usecase membre (+ repo) | `application/src/usecase/member.rs` L209/327/464/525 ; `infra/.../project_member_role_permission_repository.rs` L47 | Panic 500 sur grant/revoke si `id` absent |
| P0-2 | Échec de `event_publisher.publish` seulement `warn!` | usecases `project` / `component` / `member` | `sentinel-sync` / OpenFGA désynchronisés sans erreur API |

### P1

| ID | Écart | Preuve | Risque |
|---|---|---|---|
| P1-1 | Logging hors `rustycog-logger` | `setup/src/config.rs` `setup_logging` | Double init, pas de Loki/`logging.filter` |
| P1-2 | Outbox seulement à la création projet | `ProjectCreationUnitOfWorkImpl` vs publish direct ailleurs | Perte d’events si broker down |
| P1-3 | Double autorisation SQL + OpenFGA | tables `*_permissions` + `PermissionChecker` | Incohérence ACL |
| P1-4 | Authn HS256 + secret commité ; IAM RS256 à part | `config/default.toml` ; wiki runtime | Prod / interop tokens IAM |
| P1-5 | Lecture anonyme publique incomplète | wiki API : pas de `viewer@user:*` sentinel-sync | 403 vs intention produit |
| P1-6 | HTTP n’utilise pas `ServiceError` | `http/src/error.rs` vs `using-rustycog-core` | Mapping status dupliqué |

### P2

| ID | Écart | Preuve |
|---|---|---|
| P2-1 | `domain/src/error.rs` mort | zéro référence Serena |
| P2-2 | `iam-events` unused | seul `Manifesto/Cargo.toml` |
| P2-3 | OpenAPI sans `GET .../details` | `openspecs.yaml` vs `create_router` |
| P2-4 | Type FGA `component` non gardé HTTP | routes `"project"` |
| P2-5 | Pas de `/ready` ; health queue absent | `RouteBuilder.health_check` only |
| P2-6 | Métriques limitées au checker | `MetricsPermissionChecker` |
| P2-7 | Logs handlers peu structurés | `handlers/*.rs` |

---

## 7. Forces

1. **Référence RustyCog** — layout, `AppState`, `RouteBuilder`, prefix, monolithe (`router` + background) exemplaires.
2. **Une surface commande** — tous les handlers HTTP passent par `GenericCommandService`.
3. **AuthZ centralisée** — OpenFGA + cache opt-out test + métriques ; plus de fetcher par route.
4. **Tests d’intégration riches** — testcontainers, wiremock catalog, OpenFGA mock, ACL fail-closed, SQS routing, transactions.
5. **Ports hexagonaux clairs** — read/write repos + `ComponentServicePort`.
6. **Lifecycle mature** — no-op queue, shutdown, outbox dispatcher, consumer Apparatus optionnel.

---

## 8. Dépendances IAM (hors code métier)

| Artefact | Lien Manifesto | Note |
|---|---|---|
| `AuthConfig` / JWT rustycog | `UserIdExtractor` | HS256 ; pas le issuer IAM RS256 |
| `openfga/model.fga` | `with_permission_on(..., "project")` | `component` prévu plus tard |
| `sentinel-sync` | consumers d’events Manifesto (hors crate) | critique si P0-2 |
| `iam-events` | path-dep `manifesto-service` | **morte** |
| `apparatus-events` | consumer infra | OK |

IAMRusty n’est pas dans le périmètre d’édition ; cité uniquement comme producteur de tokens / events IAM.
