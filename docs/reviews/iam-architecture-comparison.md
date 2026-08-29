# Comparaison d’architecture — Manifesto / Telegraph / IAMRusty / Hive

| Champ | Valeur |
|---|---|
| Date | 2026-08-29 |
| Périmètre | Les 4 services RustyCog / IAM du monorepo : **Manifesto**, **Telegraph** (dit « Telegraf »), **IAMRusty**, **Hive** |
| Question | Utilisent-ils les mêmes stratégies, consignes et designs RustyCog / IAM — et où ça diverge ? |
| Méthode | Synthèse des 4 revues individuelles + skill `rustycog` + QMD wiki. Serena / fichiers seulement pour un écart wiki vs revue (JWT `default.toml`). Pas de revue from scratch. |
| Context Mode | Indisponible dans ce chat (`ctx_*` absent du catalogue MCP). Extraction des tableaux par script Node. |

**Sources**

- `docs/reviews/iam-manifesto-architecture.md`
- `docs/reviews/iam-telegraf-architecture.md`
- `docs/reviews/iam-rusty-architecture.md`
- `docs/reviews/iam-hive-architecture.md`
- `.cursor/skills/rustycog/SKILL.md` + `references/building-rustycog-services.md`
- QMD : `aiforall-wiki/skills/building-rustycog-services.md`, `aiforall-wiki/projects/iamrusty/references/iamrusty-runtime-and-security.md`

**Légende des verdicts** (identiques aux revues) : **conforme** / **partiel** / **divergent** / **N/A**.  
**Cohérent ?** = les 4 appliquent-ils *la même stratégie* (pas seulement le même score).

---

## Synthèse en une page

Les quatre services partagent **le même gabarit hexagonal RustyCog** : slice verticale, un composition root, `GenericCommandService`, `RouteBuilder` + `AppState`, `SERVICE_PREFIX` + `create_router` / `create_prefixed_router`, APIs monolithe (`router` + background tasks, pas de `run()` composé). Layout, tests et DI sont **cohérents**.

Ils **divergent structurellement** sur : logging (`setup_logging` vs subscriber maison Manifesto), contrat JWT (extracteur rustycog-http **HS256 only** vs garde RS256 IAM), boucle OpenFGA / `sentinel-sync`, atomicité outbox, surface d’erreurs (`ServiceError` vs mapping maison ; Telegraph flatten queue → infra), et fidélité OpenAPI ↔ routes live (Hive).

**Golden path scaffolding : Manifesto.**  
**Golden path IdP : IAMRusty.**  
**Exception logging : Manifesto est celui qui s’écarte** (wiki `^[ambiguous]`).

---

## 1. Layout / boundaries

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **conforme** | **conforme** | **conforme** | **conforme** |

**Cohérent ? oui**

Même slice `domain` / `application` / `infra` / `http` / `setup` / `configuration` / `migration`, ports vs adapters, prefix runtime :

| Service | `SERVICE_PREFIX` |
|---|---|
| Manifesto | `/manifesto` |
| Telegraph | `/telegraph` |
| IAMRusty | `/iam` |
| Hive | `/hive` |

Leftovers locaux (non structurels) : `domain/src/error.rs` mort (Manifesto, Hive), handlers `communication` morts (Telegraph), `_role_service` / `_user_service` construits puis ignorés (Hive, IAMRusty), domain qui réexporte `*-events` (Telegraph, Hive).

---

## 2. Config / secrets

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** | **partiel** |

**Cohérent ? partiel**

**Pareil :** config typée + préfixe service (`MANIFESTO` / `TELEGRAPH` / `IAM` / `HIVE`), sections rustycog `server` / `database` / `logging` / `queue`, secret JWT **et** mot de passe Postgres **en git** (`rustycog-dev-hs256-secret`, `postgres`/`postgres`). Tous les `[auth.jwt]` consommateurs pointent le même HS256.

**Pas pareil :**

| Point | Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|---|
| Section `[command]` | présente + retry | **absente** | présente | présente **mais non branchée** (`CommandRegistryBuilder::new()`) |
| Queue défaut | `disabled` (OK) | SQS enabled | SQS enabled | SQS enabled |
| Secrets distants | — | — | Vault / GCP **stubs** | — |
| Leftover | — | SMS doc ≠ struct ; descriptors hardcodés | `[kafka]` legacy + `APP_`/`IAM_` wiki | `iam_service` mort |

Vérification code (écart QMD vs revue IAMRusty) : `IAMRusty/config/default.toml` et `production.toml` sont **HS256 `type = "plain"`**, pas `pem_file` RS256. Le wiki `iamrusty-runtime-and-security` (l.40) est **stale**. La revue IAMRusty a raison.

---

## 3. Erreurs

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **divergent** | **partiel** | **partiel** |

**Cohérent ? non**

**Pareil (dette plateforme) :** `thiserror` local, mapping HTTP **maison**, `ServiceError` surtout côté events, `unwrap` prod, setup en `anyhow`. Contredit `using-rustycog-core` (`ServiceError::http_status_code` / `is_retryable`).

**Divergence Telegraph :** toute erreur commande queue → `ServiceError::infrastructure` (`TelegraphEventHandler::handle_event`). Retry / poison / ack-nack faux. Les mappers notification forcent aussi infra (500 au lieu de 403/404). C’est le seul verdict **divergent** de l’axe.

Hive / Manifesto : HTTP = match de strings (`"not found"`) ou `CommandError` aplati. IAMRusty : `ApiError::into_response` par variante.

---

## 4. Observabilité

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** | **partiel** |

**Cohérent ? non**

| | Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|---|
| Init logs | **subscriber maison** `tracing_subscriber` (`setup/src/config.rs`) | `rustycog::logger::setup_logging` | idem | idem |
| Metrics | wrapper permissions | checker OpenFGA | **aucune** métrique IAM | checker OpenFGA |
| `#[instrument]` | rare | absent | rare | absent |

Skill rustycog : *« `setup_logging` is a global singleton. Call it exactly once… never alongside hand-rolled `tracing_subscriber`. »*  
Wiki scaffolding : *« Manifesto still uses the latter. Conflict to resolve. `^[ambiguous]` »*

Les trois autres sont **plus alignés logger** que le gabarit officiel.

---

## 5. AuthN / AuthZ

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** (AuthZ OpenFGA **N/A**) | **partiel** |

**Cohérent ? non**

### AuthN — rupture plateforme

Tous câblent `UserIdExtractor::new(config.auth)` (rustycog-http). QMD : l’extracteur **n’accepte que HS256**. IAMRusty **émet** (garde compile-time) RS256 hors `test-relaxed-jwt`. Les 3 consommateurs + le middleware IAM cassent dès que l’émission passe RSA. Wiki : Phase B, `^[ambiguous]`.

Aujourd’hui les TOML (y compris `IAMRusty/config/production.toml` `[auth.jwt]` + `[jwt.secret] type = "plain"`) restent HS256 — d’où le *« prod RS256 non bootable sans PEM »* de la revue IAM.

IAMRusty en plus : redirects OAuth hardcodés `127.0.0.1:8081`, state CSRF sans expiry.

### AuthZ OpenFGA — 4 stratégies

| Service | Stratégie | Trou |
|---|---|---|
| Manifesto | `with_permission_on` large sur `"project"` + cache TTL 0 test + `MetricsPermissionChecker` | ACL SQL encore là ; lecture anonyme publique incomplète ; type FGA `component` non gardé HTTP |
| Telegraph | **une** route mark-read : `Write` / `"notification"` | **aucun** event `NotificationCreated` → tuples jamais posés → **403 fail-closed** |
| Hive | `with_permission_on(..., "organization")` sur writes | GET/search/list sans FGA ; delete org **sans** delete tuples ; SQL rôles encore là |
| IAMRusty | IdP : pas de `with_permission_on` ; `InMemoryPermissionChecker` vide | **N/A justifié** ; `sentinel-sync` IAM → `TupleDelta` vide |

---

## 6. Contrats API

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** | **divergent** |

**Cohérent ? non**

**Pareil :** HTTP only (pas de proto/gRPC), pas de `/v1`, OpenAPI présent, prefix runtime pas toujours dans les paths spec.

| | OpenAPI | Drift |
|---|---|---|
| Manifesto | 3.0.3 | `GET …/details` live hors spec |
| Telegraph | 3.1 | 3 routes live alignées ; DTOs send-message hors contrat ; handlers communication morts |
| IAMRusty | 3.1 | prefix `/iam` runtime vs spec ; drift historique `/start` vs `/login` |
| Hive | 3.0.3 | **beaucoup plus large** que `create_router` ; invitations / `update_member` non routés ; **`/roles` live, registry vide** → 500 |

Hive est le seul **divergent** : le contrat public ment.

---

## 7. Persistance

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **conforme** | **conforme** | **conforme** | **partiel** |

**Cohérent ? non**

**Pareil :** `DbConnectionPool` R/W, migrations, repos hexagonaux.

**Divergence Hive :** outbox = **seconde transaction** après le write métier (`OrganizationUseCaseImpl` puis `HiveOutboxUnitOfWorkImpl`). Org créée + API 500 + pas d’`OrganizationCreated` → pas de `#owner` → 403 sur Admin/Write.

| | Outbox / UoW |
|---|---|
| Manifesto | UoW atomique **création projet seulement** ; ailleurs publish direct |
| Telegraph | txn create+delivery notif ; **pas d’outbox** (pas de publisher) |
| IAMRusty | outbox + `SignupTransactionImpl` |
| Hive | dispatcher outbox **hors** txn métier |

---

## 8. Messaging / events

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** | **partiel** |

**Cohérent ? non**

Même *transport* rustycog (`QueueConfig`, multi-queue, factories **no-op silencieuses**, pas de health transport). Rôles **incompatibles** :

```
IAMRusty  --iam-events-->  Telegraph (consumer only, pas de publisher)
Manifesto --manifesto-events--> sentinel-sync  (publish swallow / warn)
Hive      --hive-events--> sentinel-sync      (publish OK, translator incomplet)
IAMRusty  --iam-events--> sentinel-sync       (translator no-op)
```

| | Publisher | Consumer | Contrat IAM | sentinel-sync |
|---|---|---|---|---|
| Manifesto | oui, **best-effort** (`warn!`) | Apparatus | `iam-events` **mort** | critique si P0 publish |
| Telegraph | **non** | SQS + `GenericCommandService` | **vivant** | bloqué (pas de `NotificationCreated`) |
| IAMRusty | oui + outbox | — | producteur | `TupleDelta::default()` sur tous les events |
| Hive | oui + outbox dispatcher | **non** | — | create/join/remove OK ; **delete/update/roles/invites = no-op** |

Preuve Hive (confirmée) : `sentinel-sync/src/translator/hive.rs` L115–123 (`OrganizationDeleted` → `TupleDelta::default()`).  
Preuve IAM : `sentinel-sync/src/translator/iam.rs` L32–36.

Consigne rustycog / wiki scaffolding : *« Emitting a domain event that has no matching translator arm — the OpenFGA store falls out of sync silently. »*

---

## 9. Tests

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **conforme** | **conforme** | **conforme** | **conforme** |

**Cohérent ? oui** (variantes d’harness, même contrat)

Tous : `ServiceTestDescriptor`, `setup_test_server`, base URL déjà préfixée, testcontainers.

| | OpenFGA | SQS | Wiremock | Particularité |
|---|---|---|---|---|
| Manifesto | mock | suite dédiée | catalog | ACL fail-closed |
| Telegraph | testcontainer | **`has_sqs()==true` toute la suite** | non | MailHog ; cache TTL 0 |
| IAMRusty | `has_openfga()=false` | suite dédiée ; Kafka souvent `#[ignore]` | GitHub/GitLab | `test-relaxed-jwt` |
| Hive | **testcontainer réel** | `has_sqs=false` + suite dédiée | oui | pas de tests HTTP roles/invitations |

---

## 10. DI / composition root

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **conforme** | **conforme** | **conforme** | **conforme** |

**Cohérent ? oui**

Un composition root, `AppState::new(command, extractor, checker)`, `router()` non préfixé pour le monolithe, `start|stop_background_tasks`, standalone via `serve_router` + prefix. Aucun service n’expose `run()` comme surface de composition.

Écarts locaux : `unwrap` boot (Telegraph `event_configs`, IAMRusty `RegistrationTokenServiceImpl`), checker in-memory vide côté IAM (voulu).

---

## 11. Health / shutdown / retries

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **partiel** | **partiel** | **partiel** | **partiel** |

**Cohérent ? partiel**

**Pareil :** `RouteBuilder.health_check()` = **liveness only**, pas de `/ready`, shutdown `ctrl_c` + stop background, cache OpenFGA TTL 0 honoré là où FGA existe.

**Pas pareil (retries) :**

| | `RegistryConfig` / `[command]` | `max_attempts` |
|---|---|---|
| Manifesto | branché | présent |
| Telegraph | **absent** | retries non configurables |
| IAMRusty | branché | `0` en `test.toml` **et** `development.toml` (piège rustycog : `0` = off) |
| Hive | TOML présent, **factory ignore** | inerte |

Health queue / consumer : mort ou absent partout (Telegraph `EventConsumer::health_check` zéro référence). Contredit *« Queue factories can degrade to no-op — add an explicit health check. »*

---

## 12. Alignement rustycog

| Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|
| **conforme** (dettes listées) | **partiel** | **partiel** | **partiel** |

**Cohérent ? partiel**

Manifesto est noté **conforme** parce que c’est **le gabarit de scaffolding** (skill + sources QMD = docs Manifesto), pas parce qu’il est sans dette. Les trois autres ont le même *shell* mais plus de dettes structurelles (erreurs, events, command, contrats).

Score trompeur : sur **logger**, Manifesto est le moins rustycog ; Telegraph / Hive / IAMRusty sont le golden path `setup_logging`.

---

## Socle commun

Ce que les 4 font **pareil** (stratégie, pas juste un air de famille) :

1. **Gabarit hexagonal** + crates de slice + ports/adapters.
2. **Un composition root** ; monolithe compose `router` + background, jamais `run()`.
3. **Une surface commande** `GenericCommandService` pour HTTP (et pour la queue Telegraph).
4. **Prefix contract** `/manifesto` `/telegraph` `/iam` `/hive` — standalone = monolithe.
5. **`RouteBuilder`** : tracing, panic, correlation, `/health`.
6. **`UserIdExtractor` + `AuthConfig` HS256** rustycog-http (même secret de dev commité).
7. **`DbConnectionPool` R/W** + migrations.
8. **Harness `rustycog-testing`** : descriptor, URL préfixée, testcontainers.
9. **Pas de `/ready`**, pas de proto, pas de version d’URL.
10. **HTTP sans `ServiceError` unifié** (dette partagée).

---

## Divergences structurelles

Pas des bugs locaux — des *choix de plateforme* différents.

| Thème | Manifesto | Telegraph | IAMRusty | Hive |
|---|---|---|---|---|
| Logging | maison | rustycog-logger | rustycog-logger | rustycog-logger |
| JWT | vérifie HS256 | vérifie HS256 | émet HS256 config / **garde RS256** code | vérifie HS256 |
| OpenFGA HTTP | large `"project"` | 1 route `"notification"` | N/A (IdP) | writes `"organization"` |
| Events sortants | best-effort + outbox partiel | **aucun** | outbox signup | outbox **non atomique** |
| Events entrants | Apparatus | IAM SQS | — | — |
| `iam-events` | mort | **vivant** | producteur | — |
| sentinel-sync | dépend du publish | bloqué | no-op identité | delete/roles no-op |
| Erreurs queue | — | **tout = infra** | mapper events | mapper events |
| `[command]` retry | oui | non | oui (`0` en dev) | TOML mort |
| OpenAPI | petit drift | aligné 3 routes | petit drift | **mensonger** |

---

## P0 transverses (cohérence plateforme)

Ces items cassent **plusieurs** services à la fois. Les P0 locaux (unwrap ACL Manifesto, `/roles` Hive, OAuth localhost IAM) restent dans les revues individuelles.

### P0-T1 — JWT HS256 middleware vs RS256 IAM

- **Qui :** rustycog-http `UserIdExtractor` (les 4) vs garde RS256 IAMRusty.
- **Effet :** dès que l’IdP émet RSA, Manifesto / Telegraph / Hive **et** les routes `.authenticated()` IAM refusent les tokens.
- **Preuve :** wiki `iamrusty-runtime-and-security` l.42 ; `UserIdExtractor::new(config.auth)` dans les 4 `setup/src/app.rs` ; `[auth.jwt] hs256_secret` dans les 4 `default.toml`.
- **Note wiki stale :** QMD dit `default.toml` IAM = `pem_file` RS256 — **faux**, c’est `plain` HS256 (revue + fichier).

### P0-T2 — Boucle OpenFGA / events non fermée

Quatre trous qui se **cumulent** :

1. IAM → sentinel-sync = no-op (pas de tuples identité).
2. Hive `OrganizationDeleted` (et update/roles/invites) = no-op → tuples orphelins.
3. Telegraph garde FGA **sans** publier `NotificationCreated`.
4. Manifesto **swallow** les erreurs de publish → sync FGA silencieuse.

Consigne rustycog : un event sans bras translator = store FGA qui dérive sans erreur API.

### P0-T3 — Secrets et algo JWT en git

Même secret HS256 + MDP Postgres (+ clés AWS test Telegraph/Hive/IAM) commités. Vault/GCP IAM = stubs. `production.toml` IAM contient encore HS256 `CHANGE-ME` **et** `[kafka]` legacy.

### P0-T4 — Queue no-op invisible

Factories rustycog peuvent « réussir » sans broker. Aucun des 4 n’expose un health transport. Hive/Telegraph/IAM bootent avec SQS enabled par défaut → outbox « OK » sans delivery. Manifesto est le seul à default `disabled` (meilleure hygiène).

### P0-T5 — Telegraph : erreurs queue aplaties

Seul service **divergent** sur l’axe erreurs. Casse le contrat `is_retryable` partagé HTTP + consumer — alors que Telegraph est *le* consommateur `iam-events`.

---

## Golden path — tranche

Les revues citent tantôt Manifesto, tantôt IAMRusty. Ce n’est pas contradictoire : **deux références pour deux rôles.**

| Rôle | Golden path | Qui s’en écarte | Preuve |
|---|---|---|---|
| Scaffolding HTTP / CRUD / monolithe | **Manifesto** | dettes locales seulement | Skill : *« look like the Manifesto reference service »* ; QMD `building-rustycog-services` sources = docs Manifesto |
| IdP / émission tokens / OAuth / JWKS | **IAMRusty** | consumers ne doivent **pas** réimplémenter l’IdP | Revue IAM : *« gabarit le plus spécialisé »* ; Telegraph consomme `iam-events`, Manifesto a `iam-events` mort |
| Logger rustycog | **Telegraph / Hive / IAMRusty** | **Manifesto** (subscriber maison) | Skill `setup_logging` ; wiki `^[ambiguous]` |
| Consommation events IAM | **Telegraph** | Manifesto (`iam-events` unused) | Revue Telegraph §8 |
| Producteur org → OpenFGA | **Hive (intention)** | Hive (translator + outbox) ; Manifesto (swallow) | `translator/hive.rs` vs UoW Manifesto création projet |
| AuthZ HTTP `with_permission_on` | **Manifesto** (largeur) | Telegraph (1 route sans tuples) ; Hive (sync incomplète) ; IAM N/A | `http/src/lib.rs` des 3 consumers |

**Règle d’alignement proposée :** cloner Manifesto pour un nouveau service CRUD ; cloner le trio logger de Telegraph/Hive/IAM ; **ne jamais** cloner le JWT HS256 comme contrat prod — le corriger dans rustycog-http + IAM d’abord.

---

## Recommandations (8 actions, pas une wishlist)

Ordre = impact plateforme, pas exhaustivité.

1. **Fermer le contrat JWT plateforme (P0-T1).** Faire vérifier à `UserIdExtractor` les tokens IAM (JWKS / RS256), pas seulement HS256. Une seule source de vérité algo (IAM) ; retirer le secret HS256 commité des 4 `default.toml` / `production.toml`. Tant que rustycog-http est HS256-only, **ne pas** activer la garde RS256 IAM en prod.

2. **Fermer la boucle OpenFGA (P0-T2).** Règle unique : tout event qui change une relation a un bras `sentinel-sync` **et** une publication qui ne swallow pas. Priorité : `OrganizationDeleted` (+ rôles) Hive ; `NotificationCreated` Telegraph **ou** retirer `with_permission_on` mark-read ; arrêter le `warn!` Manifesto sur publish. IAM no-op : décider explicitement (pas de tuples identité = OK documenté, ou `user` FGA).

3. **Outbox atomique partout où on publie.** Aligner Hive sur le UoW Manifesto (création projet / signup IAM) : **une** transaction métier + outbox. Telegraph n’a pas besoin d’outbox tant qu’il ne publie pas — mais dès `NotificationCreated`, oui.

4. **`ServiceError` + `is_retryable` sur HTTP *et* queue.** Commencer par Telegraph (flatten infra = P0 local qui pourrit les retries IAM→notif). Puis Manifesto / Hive / IAMRusty (mapping maison). Plus de match `"not found"`.

5. **Un seul logger : `rustycog::logger::setup_logging`.** Migrer Manifesto (résoudre le `^[ambiguous]` wiki). Interdire un second `tracing_subscriber` dans le process (monolithe).

6. **`/ready` + health transport queue** sur les 4. Boot « OK » ≠ SQS/Kafka live. Default queue `disabled` hors prod (modèle Manifesto) ; Hive/IAM/Telegraph ne doivent pas shipper SQS enabled + secrets AWS test.

7. **Hive : contrat live = registry.** Soit enregistrer les commandes `/roles` + `RoleUseCase`, soit démonter les routes. Recoller OpenAPI sur `create_router`. Sans ça le golden path « prefix + GenericCommandService » est une fiction sur Hive.

8. **Brancher `[command]` / `RegistryConfig`.** Telegraph (section absente) et Hive (TOML mort). Ne plus mettre `max_attempts = 0` en `development.toml` IAM (même sémantique que les tests).

Hors scope de ces 8 (P1 locaux à garder dans les revues) : OAuth CSRF/redirects IAM, unwrap ACL Manifesto, descriptors Telegraph hardcodés, leftovers crates.

---

## Matrice compacte des 12 axes

| # | Axe | Manifesto | Telegraph | IAMRusty | Hive | Cohérent ? |
|---|---|---|---|---|---|---|
| 1 | Layout / boundaries | conforme | conforme | conforme | conforme | **oui** |
| 2 | Config / secrets | partiel | partiel | partiel | partiel | **partiel** |
| 3 | Erreurs | partiel | **divergent** | partiel | partiel | **non** |
| 4 | Observabilité | partiel | partiel | partiel | partiel | **non** |
| 5 | AuthN / AuthZ | partiel | partiel | partiel (FGA N/A) | partiel | **non** |
| 6 | Contrats API | partiel | partiel | partiel | **divergent** | **non** |
| 7 | Persistance | conforme | conforme | conforme | partiel | **non** |
| 8 | Messaging / events | partiel | partiel | partiel | partiel | **non** |
| 9 | Tests | conforme | conforme | conforme | conforme | **oui** |
| 10 | DI / composition root | conforme | conforme | conforme | conforme | **oui** |
| 11 | Health / shutdown / retries | partiel | partiel | partiel | partiel | **partiel** |
| 12 | Alignement rustycog | conforme* | partiel | partiel | partiel | **partiel** |

\*conforme = *référence de scaffolding*, pas « zéro dette » (logger `^[ambiguous]`).

**3 axes vraiment alignés** (1, 9, 10). **5 axes structurellement divergents** (3, 4, 5, 6, 7, 8). Le reste est la même stratégie avec des trous inégaux.
