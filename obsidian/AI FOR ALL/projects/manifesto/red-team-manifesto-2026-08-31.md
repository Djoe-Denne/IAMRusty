---
title: Audit Red Team défensif Manifesto — 2026-08-31
category: project
tags: [projects, security, audit, manifesto, visibility/internal]
summary: >-
  Audit adversarial strictement défensif du service Manifesto : AuthZ liée
  au mauvais UUID, association org non vérifiée, JWT consommateur sans
  iss/aud, dérive OpenFGA, tests qui normalisent le défaut.
created: 2026-08-31
updated: 2026-08-31
---

# Audit Red Team défensif — Manifesto (2026-08-31)

Périmètre : crate `Manifesto/` (HTTP, application, domain, infra, setup, tests, config) plus les contrats partagés réellement consommés (`rustycog-http`, `rustycog-permission`, `rustycog-config`, `openfga/model.fga`, `sentinel-sync` translator Manifesto, claims IAM). Aucun exploit, payload, ni procédure d’attaque.

Méthode : skill rustycog, context-mode (analyse en code), lecture ciblée des sources. GrepAI embeddings indisponibles (Ollama refusé) ; Serena encore en chargement. Les constats ci-dessous viennent du code, pas d’une exécution offensive.

## Résumé exécutif + verdict

Le soupçon de gros problèmes de sécurité est **fondé**. Manifesto a une coquille HTTP RustyCog reconnaissable (RouteBuilder, HS256, OpenFGA fail-closed, catalogue composants fail-closed), mais l’autorisation est **mal branchée sur les routes à plusieurs UUID**, le domaine **fait confiance au middleware**, et le graphe OpenFGA (via sentinel-sync) **ne suit pas** les écritures métier critiques.

**Verdict : non prêt production / risque élevé.** Les priorités sont (1) lier les guards au `project_id` (pas au dernier UUID), (2) interdire la création de projet organisation sans appartenance Hive/OpenFGA, (3) durcir le consommateur JWT (`iss`/`aud`, secret hors dépôt), (4) resynchroniser sentinel-sync (delete, replace permissions, visibilité).

Hypothèse de gravité max : un appelant authentifié peut rattacher un projet à **n’importe quelle** organisation (`owner_id` client). Sentinel-sync écrit alors `project#organization@organization:{owner_id}` : les admins/membres de cette org héritent `administer` / `viewer` via `openfga/model.fga`. En parallèle, les tests d’API membres **enseignent** au framework de checker `project:{user_id}` au lieu de `project:{project_id}`.

---

## Table des findings

| Sévérité | `file:line` | Titre | Classe | Impact |
|---|---|---|---|---|
| Critique | `Manifesto/application/src/usecase/project.rs:285` + `sentinel-sync/src/translator/manifesto.rs:68` | Création org sans preuve d’appartenance | Confusion tenant / association non autorisée | Projet rattaché à une org tierce ; héritage OpenFGA admin/viewer |
| Critique | `rustycog/rustycog-http/src/middleware_permission.rs:33` + `Manifesto/http/src/lib.rs:83` | Guard OpenFGA sur le UUID le plus profond | AuthZ cassée / IDOR de ressource | Check sur `project:{user_id}` ou `project:{component_id}`, pas le projet |
| Haute | `Manifesto/tests/member_api_tests.rs:157` | Tests qui seedent le mauvais objet | Tests qui normalisent un défaut | CI verte alors que la prod (tuples sur `project_id`) 403 ou autorise le mauvais objet |
| Haute | `Manifesto/application/src/usecase/member.rs:409` | `remove_member` sans contrôle domaine | AuthZ service absente | Si le middleware est contourné ou mal lié, suppression sans rôle métier |
| Haute | `Manifesto/application/src/usecase/member.rs:545` | `revoke_permission` ignore le requester | AuthZ service absente | Révoque dès que le middleware passe |
| Haute | `Manifesto/application/src/usecase/project.rs:381` | `get_project` ignore `user_id` | Défense en profondeur absente | Fuite si le guard est bypass / mauvais objet |
| Haute | `rustycog/rustycog-http/src/jwt_handler.rs:71` + `rustycog-config/.../lib.rs:264` | JWT HS256 sans `iss`/`aud` | JWT mal vérifié (consommateur) | Tout token HS256 signé avec le secret partagé est accepté |
| Haute | `Manifesto/config/default.toml:9` | Secret HS256 commité | Secrets | Forge de Bearer si le défaut n’est pas overridé |
| Haute | `sentinel-sync/src/translator/manifesto.rs:170` | `MemberPermissionsUpdated` / `ProjectDeleted` / visibilité = no-op FGA | Dérive ACL / stale allow | Grants OpenFGA orphelins ; PUT permissions ne met pas à jour FGA |
| Haute | `sentinel-sync/src/translator/manifesto.rs:132` | Grant FGA toujours sur `project_id` | Divergence contrat ACL instance | Permission « composant » écrite sur l’id projet |
| Moyenne | `Manifesto/application/src/usecase/project.rs:469` | `FieldUpdate::Set(Option)` systématique | Optional-field-update / intégrité | Omettre `description` l’efface |
| Moyenne | `Manifesto/setup/src/app.rs:190` | Cache OpenFGA 15 s par défaut | Fenêtre revoke | Allow périmé après révocation |
| Moyenne | `Manifesto/infra/src/event/consumer.rs:158` | Worker queue sans authenticité d’événement | Spoofing / replay (transport) | Statut composant muté si la file est joignable |
| Moyenne | `Manifesto/http/src/lib.rs:38` | `Write` suffit à changer `visibility` | Élévation de publication | Membre write peut basculer public (liste SQL) |
| Moyenne | `rustycog/rustycog-http/src/middleware_auth.rs:163` | Token invalide → anonyme | Bypass sémantique auth optionnelle | Pas de 401 ; poursuite en `user:*` |
| Basse | `Manifesto/infra/src/repository/project_repository.rs:153` | `LIKE` avec jokers client | Énumération / injection de motif | Pas d’SQLi (SeaORM) mais `%`/`_` élargissent le filtre |
| Basse | `Manifesto/infra/src/adapters/component_service_client.rs:26` | reqwest : redirects par défaut | SSRF sortant (config) | URL catalogue uniquement ; redirect non restreint |
| Info | — | Upload / XXE / SSTI / cookies CSRF | Surface absente | Pas de multipart, templates, ni session cookie |

---

## Détail + correctifs (hardening uniquement)

### 1. Association organisation sans preuve d’appartenance — Critique

`create_project` force `owner_id = user_id` en personnel, mais en `organization` prend `request.owner_id` tel quel (`Manifesto/application/src/usecase/project.rs:285-289`). Aucun check Hive/OpenFGA « caller ∈ org ». La route n’a que `.authenticated()` (`Manifesto/http/src/lib.rs:36-37`).

Dès `ProjectCreated` avec `owner_type == "organization"`, sentinel-sync écrit le parent org (`sentinel-sync/src/translator/manifesto.rs:68-75`). Le modèle FGA donne alors `admin: ... or admin from organization` et `viewer: ... or viewer from organization` (`openfga/model.fga:21-28`).

**Fix :** avant persist, `Check` OpenFGA (ou API Hive) : le caller a au moins `member`/`admin` sur `organization:{owner_id}`. Rejeter sinon. Ne pas faire confiance au body pour le tenant. Idéalement `owner_id` n’est pas un champ client libre : il vient d’un contexte org déjà autorisé.

### 2. Middleware permission = dernier UUID du path — Critique

`extract_deepest_resource_id` prend le dernier segment UUID (`rustycog/rustycog-http/src/middleware_permission.rs:33-38`) et construit `ResourceRef::new(object_type, resource_id)` (`:68`). Manifesto déclare `with_permission_on(..., "project")` sur des routes dont le dernier UUID n’est **pas** le projet :

- `GET/PUT/DELETE .../members/{user_id}` — `Manifesto/http/src/lib.rs:83-100`
- `.../permissions/{resource}` — dernier UUID = `user_id` (`:102-113`)
- `.../permissions/{resource}/{resource_id}` — dernier UUID = `resource_id` (`:114-125`)

Le check devient `project:{user_id}#read/administer` ou `project:{component_id}#administer`.

Conséquences (classes, pas de recette) :

- **Prod** (tuples sentinel-sync sur `project:{project_id}`) : routes membres à UUID profond **échouent fermé** (403) pour les vrais admins — ou passent si un autre objet a le même UUID.
- **Confusion d’objet** : le guard autorise sur l’id de membre/composant ; le use-case mute le `project_id` du path. La cohérence AuthZ HTTP ≠ AuthZ métier.

Les commentaires dans `lib.rs:50-54` montrent que l’équipe connaît le piège pour `{component_type}` (non-UUID → projet). Elle ne l’a pas corrigé pour `{user_id}` / `{resource_id}`.

**Fix (rustycog + Manifesto) :**

- Permettre de nommer le paramètre de ressource (`with_permission_on(..., "project", path_param: "project_id")`) au lieu du « deepest UUID ».
- En attendant : ne pas mettre de UUID plus profond que le projet sur les routes project-scoped, **ou** extraire `project_id` explicitement.
- Recheck domaine : tout handler mutateur vérifie le rôle sur **ce** `project_id` (OpenFGA **et** membership DB), pas seulement le middleware.

### 3. Tests qui normalisent le défaut — Haute

`Manifesto/tests/member_api_tests.rs` documente et seed le mauvais objet :

- L.157-163 : `GET /members/{user_id}` → `ResourceRef::new("project", owner_id)`
- L.293-299 : `PUT` → `project:{regular_member_id}`
- L.357-363 : `DELETE` → `project:{member_to_remove_id}`
- L.549-555 / L.617-621 : grant spécifique → `project:{component.id()}`

Ce n’est pas un oubli : les commentaires disent « trailing UUID = user id / component id ». La CI **verrouille** le contrat dangereux.

**Fix :** arranger `ResourceRef::new("project", project.id())` uniquement. Les tests doivent **échouer** tant que le middleware checke le mauvais id. Ajouter un test de non-régression : admin du projet A ne passe pas un guard en mettant l’id d’un autre objet en dernier segment.

### 4. Couche métier trop mince — Haute

| Opération | Middleware | Use-case |
|---|---|---|
| `get_project` / `get_project_detail` | Read sur (dernier UUID) | `_user_id` ignoré (`project.rs:381`, `:390`) |
| `update` / `delete` / `publish` / `archive` | Write/Owner/Admin | Pas de `has_permission` domaine |
| `remove_member` | Admin (mauvais objet) | Aucun check requester (`member.rs:409-421`) |
| `revoke_permission` | Admin (mauvais objet) | `let _ = get_member(requester)` (`:545-548`) |
| `add` / `update` / `grant` membre | Admin | `has_permission` DB (meilleure défense) |

`can_manage` existe (`permission_level.rs:35-37`, `project_member.rs:117-125`) mais n’est pas branché sur remove/revoke.

**Fix :** une fonction unique `assert_can(requester, project_id, Permission)` appelée par **chaque** use-case. Les commandes ne doivent pas être exécutables (HTTP, file, Hive, tests) sans ça. Traiter `CommandError::forbidden`, pas seulement la validation.

### 5. JWT consommateur vs émetteur IAM — Haute

Consommateur Manifesto :

- `UserIdExtractor` HS256-only (`jwt_handler.rs:9-12`, `:71-76`)
- Claims requis : `exp` (Validation) + `sub`/`iat`/`jti` à la main (`:117-137`)
- `JwtAuthConfig` n’a que `hs256_secret` (`rustycog-config/.../lib.rs:264-268`) — **pas d’issuer, pas d’audience**
- `jti` exigé puis **jamais** confronté à une denylist / store de replay
- Setup : même secret que IAM, pas de JWKS (`Manifesto/setup/src/app.rs:176-179`)
- IAM `TokenClaims` : `sub`, `username`, `exp`, `iat`, `jti` — pas de `iss`/`aud` (`IAMRusty/domain/src/entity/token.rs:7-21`)
- JWKS IAM vide en HS256 (`jwt_encoder.rs:248-250`)

Ce qui est **sain** : `Validation::new(Algorithm::HS256)` refuse les autres `alg` ; `exp` est vérifié ; Manifesto n’active pas `default_user_id` (`UserIdExtractor::new` sans fallback).

**Fix :**

- Ajouter `iss` + `aud` côté IAM et `set_issuer` / `set_audience` côté rustycog-http.
- Secret uniquement via secret store / env ; retirer les valeurs commitées des TOML non-test.
- Plan JWKS/RS256 **aligné** émetteur + tous les consommateurs (aujourd’hui un RS256 IAM serait injouable).
- `username` et claims custom : ne jamais s’en servir pour l’AuthZ (déjà le cas — à garder).

### 6. Secret HS256 dans le dépôt — Haute

`Manifesto/config/default.toml:9` et `development.toml:13` : `hs256_secret = "rustycog-dev-hs256-secret"`. Le secret de test (`test.toml:9`) est le même que `rustycog-testing` (`TEST_HS256_SECRET`). `.env` est gitignoré ; les TOML **ne le sont pas**.

**Fix :** default sans secret (fail-boot). Dev : env uniquement. Rotation si ces valeurs ont jamais servi hors local.

### 7. Dérive OpenFGA / sentinel-sync — Haute

Translator (`sentinel-sync/src/translator/manifesto.rs:161-176`) :

- **Écrit** : ProjectCreated (owner + org parent), ComponentAdded/Removed, MemberAdded, PermissionGranted/Revoked
- **No-op** : `ProjectUpdated`, `ProjectDeleted`, `ProjectPublished`, `ProjectArchived`, `MemberPermissionsUpdated`, `ComponentStatusChanged`

Effets :

- PUT permissions membre : DB remplacée, FGA **inchangé** (stale allow).
- Delete projet : tuples owner/org **restent**.
- Visibilité `public` : pas de `viewer@user:*` (la wiki interne le dit déjà : Phase 2 non livrée). La liste SQL peut montrer un projet public (`project_repository.rs:126-129`) alors que GET anonyme 403 — split-brain, pas une fuite FGA.
- `permission_granted` utilise **toujours** `evt.project_id` comme object id (`:136-141`), même si `resource == "component"` → tuple `component:{project_id}` absurde, ou `project:{project_id}` pour une ressource UUID d’instance (`resource_to_object_type` fallback `"project"` `:41-45`).

Double ACL : tables Manifesto (`PermissionService`) **et** OpenFGA. Le HTTP ne consulte que FGA ; le métier membres consulte surtout la DB. Les deux peuvent diverger.

**Fix :** événements dédiés (visibilité, delete, replace ACL) + translator qui delete/write le **bon** object id (instance composant ≠ projet). Écrire FGA dans la même UoW que la DB, ou outbox déjà présente (`setup/src/app.rs:152-156`) avec contrat « pas de 200 métier si le delta FGA n’est pas enregistré ».

### 8. Optional-field-update mal branché — Moyenne

Le domaine a `FieldUpdate::Unchanged | Set` (`field_update.rs:3-7`) et `update_metadata` ne touche la description que sur `Set` (`project.rs:116-124`). Le use-case fait `FieldUpdate::Set(request.description.clone())` (`usecase/project.rs:469`) alors que le DTO est `Option<String>` (`dto/project.rs:33`). Absent et `null` sont indistinguables → clear involontaire.

Owner/role/tenant **ne sont pas** sur `UpdateProjectRequest` (pas de mass-assignment owner à l’update). Le risque ici est l’intégrité, pas l’élévation.

**Fix :** DTO `FieldUpdate` (ou `#[serde(default)]` + absence vs null) ; n’appeler `Set` que si le champ est présent.

### 9. Queue / worker apparatus — Moyenne

`ApparatusEventHandler` désérialise le JSON de n’importe quel `DomainEvent` supporté en `ApparatusDomainEvent` (`consumer.rs:158-168`) puis applique un changement de statut si `old_status` matche (`component_processor.rs:64-88`). Pas de signature, HMAC, ni identité publisher. Replay : skip si déjà sur la cible (`:52-61`) — raisonnable. Stale : ignore (`:66-76`).

La file est un **plan de confiance**. `development.toml` active SQS Localstack avec clés `test` (`:31-40`).

**Fix :** file privée, IAM broker, enveloppe signée ou source allowlist, idempotency key persistée, ne pas exposer le consumer sur un bus partagé non authentifié.

### 10. Auth optionnelle qui avale un JWT invalide — Moyenne

`optional_auth_middleware` : échec d’extraction → continue sans user (`middleware_auth.rs:155-168`). Les routes `might_be_authenticated` + permission optionnelle évaluent alors `Subject::wildcard()` (`middleware_permission.rs:134-141`). Un Bearer cassé n’est pas un 401.

**Fix :** header `Authorization` présent mais token invalide → 401. Anonyme seulement si header absent.

### 11. Visibility / Write — Moyenne

`PUT` projet : `Permission::Write` (`http/src/lib.rs:38-40`). Le body peut setter `visibility` (`dto/project.rs:35`). Un writer rend le projet `public` : la **liste** SQL l’expose (`project_repository.rs:128-129`) même sans tuple `user:*`.

**Fix :** changement de visibilité = `Admin` ou `Owner`. Émettre un événement FGA dédié quand Phase 2 public-read arrivera.

### 12. Cache OpenFGA 15 s — Moyenne

`unwrap_or(15)` (`setup/src/app.rs:190-197`). Fenêtre revoke→deny. Les tests mettent `cache_ttl_seconds = 0` (`config/test.toml:91`) — le code honore 0 (sain).

**Fix :** TTL court + invalidation sur events sentinel-sync ; ne pas cacher les deny si le produit exige un revoke immédiat.

### 13. Client catalogue / SSRF — Basse

`base_url` vient de la config, pas d’un champ utilisateur (`component_service_client.rs:21-42`). Pas de webhook user-controlled. reqwest suit les redirects par défaut. Les erreurs loggent le body amont (`:80-82`) — risque de secret catalogue dans les logs.

**Fix :** `redirect(Policy::none())`, allowlist host, ne pas logger le body.

### 14. Grant « owner » / ressources arbitraires — Basse à moyenne

`valid_permissions` inclut `"owner"` (`command/member.rs:93`). Le use-case exige que le requester **ait déjà** ce niveau (`member.rs:222-227`) — un admin ne peut pas s’auto-promouvoir owner (sain). En revanche `can_manage` (strictement supérieur) n’est pas utilisé : un admin peut cloner `admin`.

`grant_permission` prend `{resource}` path string (`handlers/members.rs:180`) et `get_or_create_role_permission` crée la ressource. Pas de allowlist stricte au-delà du catalogue composant.

**Fix :** allowlist `project|component|member` (+ UUID d’instance existante). Interdire `owner` sauf use-case de transfert d’ownership dédié.

### 15. CORS / CSRF / cookies

Aucun layer CORS dans `RouteBuilder` (`builder.rs:200-207`). Auth = Bearer, pas de cookie de session. CSRF navigateur classique : faible. Pas de `Access-Control-Allow-Origin` permissif dans Manifesto.

**Fix :** si un front cookie arrive un jour : SameSite, CSRF token, CORS allowlist. Aujourd’hui : garder Bearer-only.

### 16. SQL / path / templates / upload

Repositories SeaORM filtrés, pas de SQL interpolé. Le `LIKE` (`project_repository.rs:153`) est bindé — pas d’SQLi ; les jokers restent côté motif. Pas de `multipart`, XML, Tera/Askama. `access_token` / `endpoint` restent `None` (`usecase/component.rs:120-121`).

---

## Malimplémentations (hors « vuln » classique)

1. **Deux sources de vérité ACL** (tables Manifesto + OpenFGA) sans invariant unique.
2. **`FieldUpdate` domaine vs `Option` HTTP** — le pattern vault n’est pas appliqué à la frontière.
3. **Commentaire mensonger** `middleware_auth.rs:109` : « Extract user ID from token (no verification) » alors que `extract_user_id` vérifie la signature.
4. **Wiki API** (`manifesto-api-and-permission-flows.md`) dit « member routes project-scoped » sans dire que le middleware ne bind pas `project_id`.
5. **Hive** a son propre registry ; pas d’invocation Manifesto vue dans `Hive/`. Le risque « command bus nu » est surtout **intra-Manifesto** : les handlers passent `user_id` dans la commande, mais les handlers de commandes ne re-vérifient pas l’AuthZ. Un futur adaptateur file/Hive sur ces commandes hériterait du trou.
6. **`max_attempts = 0`** dans les TOML : retries coupés (piège rustycog, pas une faille d’auth).
7. **`Application::router()`** expose `create_router` sans préfixe (`setup/src/app.rs:338-339`) — correct pour monolithe si le nest est ailleurs ; à vérifier à la composition pour éviter un double mount / route nue. *Hypothèse* : le monolithe neste `/manifesto`.
8. **OpenFGA `store_id = ""`** dans default/dev (`default.toml:45`) : fail-closed ou client cassé au runtime selon l’impl checker — *hypothèse* jusqu’à lecture de `OpenFgaPermissionChecker`.
9. **Liste vs GET** : org inheritance FGA n’existe pas dans le SQL de liste (`internal` n’apparaît que si membre projet). Incohérence produit, pas une ouverture anonyme.

---

## Surfaces / hypothèses

| Surface | Statut |
|---|---|
| API HTTP `/manifesto` + `/health` | Cartographiée (`http/src/lib.rs`) |
| JWT consommateur HS256 | Cartographié ; pas de JWKS |
| OpenFGA + sentinel-sync | Cartographié ; sync incomplet |
| Queue apparatus | Consumer statut composant ; confiance transport |
| Appels sortants | Catalogue composants (config) |
| Upload / webhooks user URL | **Absents** |
| Multi-tenant org | Via `owner_type` + tuple org ; **sans check membership** |
| Cookies / CORS | Absents |
| Hive command parity | Hive n’enregistre pas les commandes Manifesto ; *hypothèse* : pas de bus croisé aujourd’hui |
| GrepAI / Serena | Embeddings down ; Serena loading — pas d’exploration symbolique LSP |

*Hypothèse* : si un déploiement oublie d’override `hs256_secret` et pointe OpenFGA + sentinel-sync réels, les findings Critique/Haute sont exploitables par tout porteur d’un compte IAM (pas besoin d’accès admin plateforme).

*Hypothèse* : collision UUID projet ↔ user est négligeable ; le scénario réaliste du deepest-UUID est surtout **403 légitimes** + **tests menteurs**, sauf si des tuples `project:{user_id}` existent en store (les tests les créent ; un mauvais sync pourrait les créer).

---

## Ce qui est sain

- Algorithme JWT **contraint HS256** ; `exp` / `sub` UUID / `iat` / `jti` non vides.
- Pas de `default_user_id` en composition Manifesto.
- Routes mutatrices projet/composant/liste membres : dernier UUID = `project_id` (guards corrects sur ces-là).
- `GET /api/projects` filtre visibilité SQL (public ou membership active), pas une liste globale.
- Création **personnelle** : `owner_id` forcé au caller.
- Update projet : pas de champs owner/tenant.
- Catalogue composants **fail-closed** (`component_service.rs:158-168`).
- ACL instance composant créée/supprimée avec le composant (`usecase/component.rs:168-170`, `:304+`) — intention de cohérence (côté DB Manifesto).
- `access_token` non émis.
- SeaORM, pas de SQL string-built pour l’authz.
- Cache FGA désactivable (`0`) et honoré au setup.
- Health sans auth (attendu).
- Bearer only → CSRF cookie faible.
- Prefix `SERVICE_PREFIX = "/manifesto"`.
- Quota projets/membres/composants.
- `ValidatedJson` + validator sur les bodies.
- Consumer apparatus : ignore stale / skip duplicate (anti-rewind).
- Tests d’intégration réels OpenFGA (pas un fake permissif global) — le problème est **ce qu’ils arrangent**, pas l’absence de checker.

---

## Ordre de remédiation suggéré

1. Binder les guards au `project_id` + casser/réécrire les tests membres.
2. Check org membership à la création + ne plus accepter un `owner_id` org arbitraire.
3. AuthZ domaine sur get/update/delete/remove/revoke (fail-closed hors HTTP).
4. JWT `iss`/`aud` + secret hors git.
5. Translator sentinel-sync : delete, replace permissions, object ids d’instance, (plus tard) public `user:*`.
6. Token optionnel invalide → 401 ; cache FGA ; file signée.

Pas de commit associé à cet audit.
