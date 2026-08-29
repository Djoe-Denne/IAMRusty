# Revue d’architecture — IAM Rusty

**Service :** `IAMRusty` (binaire `iam-service`, crates `iam-*`)  
**Date :** 2026-08-29  
**Référentiel :** RustyCog (skill `.cursor/skills/rustycog` + wiki QMD `aiforall-wiki/projects/iamrusty`)  
**Périmètre :** `IAMRusty/**`, `iam-events/**`. Manifesto / Telegraph cités seulement comme consommateurs / consigne partagée.

## Méthode et limites d’outillage

| Outil | Statut |
|---|---|
| Serena | OK — projet `AIForAll` activé ; exploration symbolique (`get_symbols_overview`, `find_symbol`, `find_referencing_symbols`, `search_for_pattern`). |
| GrepAI MCP (`user-grepai`) | **Index MCP hors repo** (hits Alcoholic / Minecraft). RPG désactivé. Relais : CLI `grepai` local (821 fichiers, watcher OK) + traces `build_and_run` / `create_router`. |
| Context Mode MCP | **Indisponible** dans cette session (aucun namespace `ctx_*`). Analyse via Serena + GrepAI CLI + QMD. |
| QMD CLI | OK — collection `aiforall-wiki` (pages `projects/iamrusty/*`). Pas de collection `projects/rustycog` indexée ; consignes RustyCog lues depuis le skill repo. |

## Carte du module

| Crate | Rôle |
|---|---|
| `iam-service` (`IAMRusty/`) | Bin standalone (`src/main.rs`) + tests d’intégration. |
| `iam-domain` | Entités, `DomainError`, ports, services métier. |
| `iam-application` | Use cases + commandes + `CommandRegistryFactory`. |
| `iam-infra` | Repos SeaORM, OAuth GitHub/GitLab, JWT, outbox UoW, `IAMErrorMapper`. |
| `iam-http_server` | `RouteBuilder`, `SERVICE_PREFIX = "/iam"`, handlers, `ApiError`. |
| `iam-configuration` | `AppConfig` prefix `IAM`, secrets JWT, traits `Has*`. |
| `iam-setup` | Composition root `IAMRustyApp` / `build_app_state*`. |
| `iam-migration` | SeaORM + `rustycog::outbox::outbox_migration()`. |
| `iam-events` | Contrat `IamDomainEvent` (`DomainEvent`). |

Consommateurs hors périmètre : `monolith` (`iam-setup`, `iam-http_server`, `iam-configuration`) ; `sentinel-sync` (`IamDomainEvent`).

---

## Synthèse des 12 axes

| # | Axe | Verdict | Écart vs RustyCog |
|---|---|---|---|
| 1 | Layout / boundaries | **conforme** | Slice hexagonale + prefix `/iam` alignés. |
| 2 | Config / env / secrets | **partiel** | Loader rustycog OK ; Vault/GCP stubs ; `[kafka]` legacy ; `default.toml` HS256 + queue SQS enabled. |
| 3 | Erreurs | **partiel** | `thiserror` + mapping HTTP maison ; `ServiceError` surtout events ; `unwrap` composition root. |
| 4 | Observabilité | **partiel** | `setup_logging` + tracing ; pas de metrics IAM ; pas de readiness. |
| 5 | AuthN / AuthZ | **partiel** | AuthN riche ; AuthZ OpenFGA N/A (IdP) ; HS256 extractor vs RS256 ; OAuth CSRF/redirects. |
| 6 | Contrats API | **partiel** | OpenAPI 3.1 présent ; pas de proto/gRPC ni `/v1`. |
| 7 | Persistance | **conforme** | Pool R/W, migrations, outbox, transactions signup. |
| 8 | Messaging / events | **partiel** | rustycog queues + outbox ; pas de health transport ; translator sentinel no-op. |
| 9 | Tests | **conforme** | rustycog-testing, fixtures, wiremock, testcontainers. |
| 10 | DI / lifecycle | **conforme** | Un composition root ; APIs monolith `router` / background tasks. |
| 11 | Patterns partagés | **partiel** | Health + shutdown OK ; retries `max_attempts=0` en test/dev ; pas de readiness. |
| 12 | Alignement rustycog | **partiel** | Framework umbrella + shell OK ; écarts `ServiceError`, JWT middleware, leftovers. |

---

### 1. Layout crate / boundaries — **conforme**

| Critère rustycog | Preuve | Écart |
|---|---|---|
| Slice `domain` / `application` / `infra` / `http` / `setup` / `configuration` / `migration` | Crates ci-dessus ; `IAMRusty/domain/src/lib.rs` (`entity`, `error`, `port`, `service`) | Aucun |
| Ports vs adapters | Ports `UserRepository`, `JwtTokenEncoder`, `ProviderOAuth2Client`, … dans `domain/src/port/` ; impls `infra/` | Aucun |
| HTTP : `SERVICE_PREFIX`, `create_router`, `create_prefixed_router` | `iam_http_server::SERVICE_PREFIX = "/iam"` ; nest dans `create_prefixed_router` ; `create_app_routes` → `rustycog::http::serve_router` | Aucun |
| Setup expose `router()` non préfixé pour le monolith | `IAMRustyApp::router` → `create_router` | Aucun |
| Une surface commande | `CommandRegistryFactory::create_iam_registry` → `GenericCommandService` | Services domaine `_user_service` / `_refresh_token_service` créés puis ignorés (`setup/src/app.rs`) |

**Constat :** c’est le gabarit Manifesto / RustyCog appliqué à un IdP (OAuth / JWT / tokens isolés par flux).

---

### 2. Config / env / secrets — **partiel**

| Critère | Preuve | Écart |
|---|---|---|
| Config typée + prefix service | `AppConfig::config_prefix() -> "IAM"` ; `load_config_with_cache` | Docs wiki : mélange `APP_` / `IAM_` |
| Sections rustycog `server`, `database`, `logging`, `queue`, `command` | `HasServerConfig`, `HasDbConfig`, `HasLoggingConfig`, `HasQueueConfig`, `HasScalewayConfig` ; champ `command` | Aucun sur les traits |
| Secrets JWT abstraits | `SecretStorage` : `PlainText`, `PemFile`, `Vault`, `GcpSecretManager` | Vault / GCP : `warn!` + erreur « not yet implemented » |
| Pas de secrets en dur | `config/default.toml` : `[jwt.secret] type = "plain"`, `value = "rustycog-dev-hs256-secret"` ; `[auth.jwt] hs256_secret` | Secret de dev commité ; prod RS256 non bootable sans PEM |
| Queue unique | `QueueConfig` + `create_multi_queue_event_publisher` | Champ legacy `kafka: KafkaConfig` encore sur `AppConfig` ; `default.toml` `queue.enabled = true` (SQS) |

**Symboles :** `AppConfig`, `SecretStorage::resolve`, `JwtConfig::create_jwt_algorithm`, `load_config`.

---

### 3. Erreurs — **partiel**

| Critère | Preuve | Écart |
|---|---|---|
| `thiserror` domaine | `iam_domain::error::DomainError` | Pas les constructeurs `rustycog-core::DomainError` |
| Mapping transport via `ServiceError::http_status_code` | Mapping **maison** `ApiError::into_response` (`http/src/error.rs`) : 404/401/400/500 par variante | `rustycog::core::error::ServiceError` surtout `IAMErrorMapper` (events) |
| `anyhow` composition root | `build_app_state*`, `run_server`, `main` | Acceptable au bord ; pas le chemin handler |
| Pas d’`unwrap` prod | `RegistrationTokenServiceImpl::new(...).unwrap()` (`setup/src/app.rs`) ; `maybe_event_publisher.unwrap()` après `is_some` ; `Mutex::lock().unwrap()` cache config | **Divergent** sur le JWT d’inscription |

Handlers passent par `CommandError` → `ApiError`. Pas de gRPC.

---

### 4. Observabilité — **partiel**

| Critère | Preuve | Écart |
|---|---|---|
| `setup_logging` une fois, tôt | `IAMRusty/src/main.rs` → `config::setup_logging(&config)` | Conforme rustycog-logger |
| Tracing structuré | `tracing::{info,warn,error,debug}` setup / domain / http / infra | Peu d’`#[instrument]` ; logs JWT (longueurs de clés) en `info!` |
| Metrics | — | Pas de `metrics` / Prometheus / `MetricsPermissionChecker` côté IAM |
| Health | `RouteBuilder::health_check()` | Pas d’endpoint readiness / live vs ready |

---

### 5. AuthN / AuthZ IAM — **partiel**

IAM **est** l’IdP : pas de `with_permission_on` — commenté dans `build_app_state_with_event_publisher`. `InMemoryPermissionChecker::new()` vide pour satisfaire `AppState::new`. `tests/common.rs` : `has_openfga() -> false`.

| Flux | Preuve | Écart |
|---|---|---|
| AuthN publique | signup, login, verify, OAuth login/callback, refresh, JWKS | Redirects OAuth hardcodés `http://127.0.0.1:8081/...` (`handlers/auth.rs`) |
| AuthN JWT | `JwtTokenService`, `RegistrationTokenServiceImpl`, `UserIdExtractor` | Wiki : extractor rustycog-http **HS256 only** vs RS256 prod (Phase B) |
| CSRF OAuth | `OAuthState { operation, nonce }` encode/decode base64 | **Pas d’expiry** (docs : state horodaté) |
| RS256 prod | `assert!` RS256 si pas `test-relaxed-jwt` (`jwt_encoder.rs`) | Feature activée en tests via `IAMRusty/Cargo.toml` dev-dep |

**Verdict AuthZ OpenFGA :** **N/A** pour les routes IAM (juste). Le trou réel est **AuthN middleware / déploiement** (HS256 vs RS256, redirects, state).

---

### 6. Contrats API — **partiel**

| Critère | Preuve | Écart |
|---|---|---|
| OpenAPI | `IAMRusty/openspecs.yaml` OpenAPI 3.1 : signup, login, verify, reset, OAuth, complete-registration, username/check, `/api/me`, refresh, JWKS, `/internal/{provider}/token|revoke` | Server example `https://iam.example.com` ; pas de nest `/iam` dans les paths spec (le prefix est runtime) |
| Proto / gRPC | — | **N/A** (HTTP only) |
| Versioning | Paths `/api/auth/...` | Pas de `/v1` |
| Surface live | `create_router` : table ci-dessus + relink | Wiki : drift historique `/start` vs `/login` |

---

### 7. Persistance — **conforme**

| Critère | Preuve |
|---|---|
| `DbConnectionPool` R/W | `build_app_state_with_event_publisher` : `get_read_connection` / `get_write_connection` |
| Repos combinés | `CombinedUserRepository`, emails, tokens, refresh, verification, password-reset |
| Migrations | `iam-migration` : `m20220101_000001_create_table` + `outbox_migration()` |
| Transactions | `SignupTransactionImpl`, `IamOutboxUnitOfWorkImpl` / `OutboxRecorder` |
| Tests DB | `IAMRustyTestDescriptor::has_db() -> true` ; `DbFixtures` |

---

### 8. Messaging / events — **partiel**

| Critère | Preuve | Écart |
|---|---|---|
| Contrat `DomainEvent` | `iam-events` : `UserSignedUp`, `UserEmailVerified`, `UserLoggedIn`, `PasswordResetRequested` | — |
| Transport rustycog | `create_multi_queue_event_publisher(&config.queue, None, IAMErrorMapper)` | Factories peuvent **no-op** sans health check (consigne rustycog) |
| Outbox | `OutboxDispatcher` + start/stop sur `IAMRustyApp` | — |
| Consommation plateforme | `sentinel-sync/src/translator/iam.rs` : match events → `TupleDelta::default()` | **Pas de tuples OpenFGA** (no-op) |
| Tests transport | `signup_kafka.rs`, `signup_sqs.rs`, `sqs_event_routing_tests.rs` | Kafka souvent `#[ignore]` ; suite HTTP : `has_sqs() -> false` |

---

### 9. Tests — **conforme**

| Critère rustycog-testing | Preuve |
|---|---|
| `ServiceTestDescriptor` + `setup_test_server` | `IAMRusty/tests/common.rs` : `IAMRustyTestDescriptor`, `IAMRustyTestDescriptorWithMockEvents` |
| Base URL déjà préfixée `/iam` | `prefixed_url` = `{server_url}{SERVICE_PREFIX}` |
| Fixtures DB fluides | `tests/fixtures/db/*` |
| Wiremock collaborateurs | `tests/fixtures/github/service.rs`, `gitlab/service.rs` |
| Testcontainers Kafka/SQS | `signup_kafka.rs`, `signup_sqs.rs`, routing SQS |
| JWT tests | `test-relaxed-jwt` ; helpers `tests/utils/jwt.rs` |

Couverture large : email/password, OAuth, registration, reset, tokens internes, outbox.

---

### 10. DI / composition root / lifecycle — **conforme**

| Critère | Preuve | Écart |
|---|---|---|
| Un composition root | `iam-setup::app::{build_app_state, build_app_state_with_event_publisher}` | — |
| `AppState::new(command, extractor, checker)` | Fin de `build_app_state_with_event_publisher` | Checker in-memory vide (voulu) |
| Monolith : pas de `run()` | `router()`, `state()`, `start_background_tasks()`, `stop_background_tasks()` | — |
| Standalone | `main` → `build_and_run` → `run_server` (HTTP + outbox, `ctrl_c`) | — |
| Publisher injectable (tests) | `Option<Arc<MultiQueueEventPublisher<_>>>` | `unwrap()` après `is_some` |

Instances OAuth / token repos **dupliquées par flux** (login vs link vs provider) — choix documenté, pas une fuite hexagonale.

---

### 11. Patterns partagés — **partiel**

| Pattern | Statut | Preuve |
|---|---|---|
| Health | **conforme** | `.health_check()` |
| Readiness | **divergent** | Absent |
| Shutdown | **conforme** | `tokio::signal::ctrl_c` ; `stop_background_tasks` ; abort handles |
| Retries commandes | **partiel** | Registry depuis `CommandConfig` ; `test.toml` / `development.toml` : `max_attempts = 0` (désactive les retries — piège rustycog) ; `production.toml` : 3–5 |
| Queue health | **divergent** | Pas de check transport au boot |

---

### 12. Alignement rustycog — **partiel**

**Aligné**

- Umbrella `rustycog = { package = "rustycog-framework", features = ["full"] }` sur tous les crates IAM.
- `RouteBuilder`, `AppState`, `GenericCommandService`, `DbConnectionPool`, `setup_logging`, `QueueConfig`, outbox rustycog.
- Prefix `/iam` (consigne standalone = monolith).
- Tests : descriptor + URL préfixée + wiremock typé.

**Anti-patterns / leftovers**

- Erreurs domaine + HTTP **hors** `ServiceError::http_status_code` / `is_retryable`.
- `iam-http_server` (underscore) vs convention `iam-http`.
- Services domaine construits puis `_` ignorés.
- `[kafka]` legacy parallèle à `queue`.
- JWT : secret HS256 par défaut + extractor rustycog HS256 vs garde RS256 prod.
- AuthZ OpenFGA volontairement absente (IdP) — OK, mais `sentinel-sync` ne matérialise pas non plus l’identité.

---

## Écarts priorisés

### P0

1. **Vérification JWT HTTP vs algo d’émission** — `UserIdExtractor` (rustycog-http) HS256 vs tokens IAM RS256 en prod. Les routes `.authenticated()` cassent dès que l’émission passe en RSA.  
   *Preuve :* wiki `iamrusty-runtime-and-security` ; `UserIdExtractor::new(config.auth)` dans `setup/src/app.rs` ; `JwtTokenService::with_refresh_expiration` + `assert!` RS256.

2. **Redirect OAuth hardcodés localhost** — `http://127.0.0.1:8081/api/auth/{github,gitlab}/callback` et relink. Inutilisable hors machine locale.  
   *Preuve :* `IAMRusty/http/src/handlers/auth.rs` (~459–461, ~1063–1064).

3. **`unwrap` sur `RegistrationTokenServiceImpl::new`** — le constructeur retourne `Result` ; le composition root unwrap. Panic au boot si RS256 manquant (hors `test-relaxed-jwt`).  
   *Preuve :* `setup/src/app.rs` ; `RegistrationTokenServiceImpl::new` → `Result<Self, DomainError>`.

### P1

4. **OAuth state sans expiry** — `OAuthState { operation, nonce }` seulement. Replay CSRF possible.  
   *Preuve :* `http/src/oauth_state.rs`.

5. **Pas de health check transport queue** — rustycog : factory peut no-op. Boot « OK » ≠ Kafka/SQS live.  
   *Preuve :* `create_event_publisher_from_config` ; consigne `using-rustycog-events`.

6. **Secrets distants non implémentés** — Vault / GCP : stub. Prod réelle = PEM fichier ou plain.  
   *Preuve :* `SecretStorage::resolve`.

7. **`sentinel-sync` IAM → `TupleDelta` vide** — events IAM n’alimentent pas OpenFGA.  
   *Preuve :* `sentinel-sync/src/translator/iam.rs`.

8. **Erreurs hors contrat rustycog-core** — pas de `is_retryable()` unifié sur le chemin HTTP.  
   *Preuve :* `DomainError` + `ApiError::into_response` vs `using-rustycog-core`.

9. **`max_attempts = 0` en development** — retries coupés (même sémantique que `test.toml`).  
   *Preuve :* `IAMRusty/config/development.toml`.

### P2

10. Leftovers `_user_service` / `_refresh_token_service` dans le composition root.  
11. Section `[kafka]` legacy + docs Kafka vs `QueueConfig`.  
12. Pas de readiness ni metrics IAM.  
13. `CONFIG_CACHE.lock().unwrap()`.  
14. Tests Kafka `#[ignore]` ; wiki / OpenAPI encore partiellement décalés.  
15. Nom de crate `iam-http_server`.

---

## Forces

1. **Hexagone + shell rustycog** : crates, ports, `SERVICE_PREFIX`, dual mode micro / monolith.  
2. **Une surface commande** : registry IAM complet depuis `CommandConfig` + `GenericCommandService`.  
3. **Identité outbox-first** : UoW + `OutboxDispatcher` + contrat `iam-events`.  
4. **Suite d’intégration de référence** : rustycog-testing, fixtures, wiremock providers, URL `/iam`.  
5. **JWT conscient prod vs test** : `SecretStorage` + feature `test-relaxed-jwt` (garde RS256 réelle en build prod).

---

## Comparaison rapide (pour le parent)

IAMRusty est le **gabarit le plus spécialisé** des trois : même ossature rustycog que Manifesto, mais le métier *est* l’auth. Les écarts P0 sont donc plus graves (middleware JWT, redirects OAuth) que des manques CRUD. Telegraph / Manifesto doivent *consommer* `/iam` + JWKS ; ils ne doivent pas réimplémenter l’IdP.
