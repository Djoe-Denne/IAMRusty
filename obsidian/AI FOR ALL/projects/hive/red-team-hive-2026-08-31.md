---
title: Audit Red Team défensif Hive 2026-08-31
category: review
tags:
  - hive
  - security
  - authorization
  - jwt
  - openfga
  - visibility/internal
aliases:
  - red-team-hive
  - audit-hive-2026-08-31
sources:
  - Hive/http/src/lib.rs
  - Hive/application/src/command/factory.rs
  - Hive/setup/src/app.rs
  - Hive/tests/members_api_tests.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-http/src/jwt_handler.rs
  - sentinel-sync/src/translator/hive.rs
  - projects/hive/concepts/command-registry-route-parity.md
  - projects/aiforall/concepts/jwt-issuer-vs-consumer.md
summary: >-
  Audit défensif Hive (2026-08-31) : AuthZ UUID le plus profond, élévation
  Write→Admin via sentinel-sync, lecture anonyme des orgs, JWT consumer HS256
  sans aud/iss. Hive n'orchestre pas d'agents.
created: 2026-08-31T15:35:00Z
updated: 2026-08-31T15:35:00Z
---

# Audit Red Team défensif — Hive — 2026-08-31

> [!warning] Périmètre et méthode
> Analyse **strictement défensive** : classes de défaut, impacts, durcissement. Aucun exploit, PoC, payload ni procédure d'attaque. Hive dans ce repo est un service de **gestion d'organisations** (membres, invitations, liens externes, sync), pas un orchestrateur d'agents.

Liens : [[projects/hive/hive]] · [[projects/hive/concepts/command-registry-route-parity]] · [[projects/hive/concepts/organization-resource-authorization]] · [[projects/aiforall/concepts/jwt-issuer-vs-consumer]] · [[projects/aiforall/concepts/queue-readiness-signaling]]

## Résumé exécutif et verdict

**Verdict : NON CONFORME — risque élevé.** Les soupçons de « gros problèmes » sont fondés, mais **pas** sur l'axe injection shell / sandbox / IPC agents. La surface réelle est l'**autorisation** : le middleware rustycog lie le `Check` OpenFGA au **dernier UUID du chemin**, alors que les handlers mutent le **premier** (`organization_id`). Les tests d'intégration **figent ce contrat**. La couche métier a des `TODO: Add permission check` et ne rattrape rien.

En parallèle : toute organisation naît `Public` ; `GET /api/organizations/{id}` est anonyme et **ignore** l'appelant ; un principal avec `Write` peut poser des rôles `Admin` qui deviennent des tuples OpenFGA via [[projects/sentinel-sync/sentinel-sync]] ; le JWT consumer est HS256 sans `aud`/`iss`, incompatible avec l'issuer RS256 d'IAMRusty.

| Question | Réponse courte |
|---|---|
| Hive orchestre-t-il des agents / un shell ? | Non. « Command » = CQRS rustycog, pas `std::process`. |
| Injection de commande OS / sandbox / IPC | Surface absente dans Hive. |
| AuthZ tenant / IDOR | Défaut structurel sur les routes à deux UUID. |
| Parité registry ↔ routes live | Les routes **live** matchent `create_hive_registry`. Handlers et commandes **morts** restent. |
| JWT | Mêmes pièges plateforme : HS256 only, pas d'`aud`/`iss`, secret partagé. |

**Priorité de durcissement :** (1) lier le `Check` à `organization_id` et casser les tests qui accordent sur `user_id` ; (2) re-vérifier l'org en use case ; (3) séparer Write et grant Admin ; (4) visibilité + garde sur `GET` org ; (5) JWT `aud`/`iss` + plus de secret de démo.

## Table

| Sévérité | `file:line` | Titre | Classe | Impact |
|---|---|---|---|---|
| Critique | `rustycog/rustycog-http/src/middleware_permission.rs:33` | `Check` sur le dernier UUID, typé `organization` | AuthZ / confusion de ressource | Le garde ne porte pas sur l'org du chemin dès qu'un second UUID est présent. |
| Critique | `Hive/http/src/lib.rs:67` | Members / roles / invitations à deux UUID | IDOR inter-tenant | Handler utilise `organization_id` ; le `Check` utilise `user_id` / `role_id` / `invitation_id`. |
| Critique | `Hive/tests/members_api_tests.rs:277` | Tests qui accordent `organization:{user_id}` | Trou figé par test | Le contrat de sécu attendu est le défaut. Régression bloquée dans le mauvais sens. |
| Critique | `Hive/application/src/usecase/member.rs:323` | `TODO: Add permission check` | AuthZ absente au métier | HTTP est le seul garde ; file d'attente / appel interne = pas d'AuthZ. |
| Élevée | `Hive/http/src/lib.rs:64` | `Write` suffit pour add/update/remove member | Élévation de privilège | Un `member` OpenFGA peut poser un rôle `Admin` local. |
| Élevée | `sentinel-sync/src/translator/hive.rs:28` | `admin` → tuple OpenFGA `admin` | Confused deputy (events) | L'événement Hive devient `administer` effectif chez le voisin. |
| Élevée | `Hive/http/src/lib.rs:31` | `GET` org / search sans `with_permission_on` | Divulgation | Lecture anonyme de n'importe quelle org par id ; search des `Public`. |
| Élevée | `Hive/application/src/usecase/organization.rs:302` | `_user_id` ignoré | IDOR lecture | Aucun filtre visibilité / membership sur le get. |
| Élevée | `Hive/domain/src/entity/organization.rs:45` | Visibilité par défaut `Public` | Mauvaise config | Toute org créée est indexable par search anonyme. |
| Élevée | `rustycog/rustycog-http/src/jwt_handler.rs:71` | JWT : HS256, `exp` seul, pas `aud`/`iss`/`nbf` | JWT consumer | Jeton d'un autre service / usage accepté si le secret est partagé. |
| Élevée | `Hive/setup/src/app.rs:148` | Consumer HS256 vs issuer IAM RS256 | JWT issuer/consumer | Commentaire explicite : pas de JWKS. Secret plat partagé. |
| Élevée | `Hive/http/src/handlers/invitations.rs:161` | Jeton d'invitation logué | Secret dans les logs | Capacité d'adhésion dans stdout / agrégateurs. |
| Élevée | `Hive/application/src/dto/invitation.rs:35` | Jeton renvoyé dans l'API | Secret dans la réponse | `InvitationResponse.token` en clair ; listes futures = fuite de masse. |
| Moyenne | `Hive/http/src/lib.rs:101` | `accept` authentifié sans garde org | Confused deputy (jeton) | Tout compte authentifié + jeton = membre aux rôles de l'invitation. Pas de lien email ↔ `sub`. |
| Moyenne | `Hive/http/src/handlers/invitations.rs:89` | Handlers invitations orphelins | Parité registry/routes | `list` / `get` / `get_by_token` existent, pas dans `create_router`, commandes non enregistrées. |
| Moyenne | `Hive/application/src/command/invitation.rs:141` | Commandes stub non registered | Parité | `list_invitations` / `get_invitation_by_token` → erreur métier si on les câble sans registry. |
| Moyenne | `Hive/tests/organization_api_tests.rs:92` | Liste orgs sans permission, 200 figé | Trou figé par test | Le test documente l'absence de garde OpenFGA (le list métier filtre quand même). |
| Moyenne | `Hive/configuration/src/lib.rs:16` | `iam_service` mort ; clé provider unique | Confused deputy | IAM jamais appelé. Provider externe = une `api_key` service pour tous les tenants. |
| Moyenne | `Hive/infra/src/external_provider/external_provider_client.rs:144` | Body POST en `debug` | Secret dans les logs | `provider_config` (jetons) dans les traces. |
| Moyenne | `Hive/domain/src/entity/external_link.rs:152` | Config provider = objet JSON non borné | SSRF délégué | Hive transmet la config avec son Bearer au collaborateur. |
| Moyenne | `Hive/config/default.toml:12` | Secret HS256 et SQS de démo | Secret / config | `rustycog-dev-hs256-secret`, `secret_access_key = "test"` dans default/dev. |
| Moyenne | `Hive/domain/src/service/member_service.rs:32` | Bypass si `added_by` vide | Confusion de privilège | Chemin « système » documenté ; le HTTP passe un user, le sync invite au nom du owner. |
| Basse | `Hive/infra/src/repository/organization_repository.rs:168` | `LIKE %query%` | Injection de métacaractères | Pas de SQLi paramétré ; `%` / `_` élargissent le search. |
| Basse | `Hive/application/src/dto/organization.rs:29` | `avatar_url` = URL générique | URL non bornée | Schéma / hôte non restreints ; pas de fetch serveur constaté. |
| Basse | `Hive/application/src/command/invitation.rs:51` | `validate()` vide | Arguments non sanitisés | Rôles / email seulement au DTO HTTP ; commandes internes non bornées. |
| Info | — | Pas de spawn OS, pas d'IPC agent | Hors surface | Axe 1 « command injection shell » : non applicable à Hive tel qu'il est. |

## Détail et durcissement

### 1. Command injection / exécution non bornée

Hive n'appelle pas de processus. `command_service.execute` est le bus CQRS rustycog (`GenericCommandService`). Les handlers HTTP construisent des structs typées.

**Défauts restants (classe « arguments non sanitisés ») :**

- Beaucoup de `Command::validate` sont des `Ok(())` (`CreateInvitationCommand` `Hive/application/src/command/invitation.rs:51`, membres, orgs).
- `CreateInvitationRequest.roles` et `AddMemberRequest.roles` acceptent `Admin` sans plafond (`Hive/application/src/dto/invitation.rs:17`, `Hive/application/src/dto/member.rs:16`, `Hive/application/src/dto/role.rs:20`).
- `provider_config` : objet JSON non vide seulement (`Hive/domain/src/entity/external_link.rs:152`).
- `settings` d'org remplacé tel quel (`Hive/domain/src/entity/organization.rs:83`).
- Search : concaténation `%{name_pattern}%` puis `like` (`Hive/infra/src/repository/organization_repository.rs:168`).

**Durcissement :** allowlist des rôles assignables selon le garde OpenFGA de l'appelant (un `Write` ne peut pas accorder `Admin`/`Owner`) ; schéma JSON pour `provider_config` / `settings` ; échapper `%`/`_` du search ; `validate()` aligné sur les DTO HTTP.

### 2. Parité registry ↔ routes

Invariant wiki : [[projects/hive/concepts/command-registry-route-parity]].

**Sain sur le live :** `create_router` (`Hive/http/src/lib.rs:27`) et `create_hive_registry` (`Hive/application/src/command/factory.rs:41`) exposent le même ensemble : orgs (CRUD + list + search), members (add/remove/list/get/update), roles (list/get), invitations (create/cancel/accept), external-link create, sync-job start.

**Reste défectueux :**

- Handlers **non montés** : `list_invitations`, `get_invitation`, `get_invitation_by_token` (`Hive/http/src/handlers/invitations.rs:89`, `:123`, `:185`).
- `get_invitation` traite `{invitation_id}` comme un **jeton** (`:134`) — confusion d'identifiant si la route est ajoutée.
- Commandes définies, **non registered** : `ListInvitationsCommand`, `GetInvitationByTokenCommand`, `ResendInvitationCommand` ; handlers stub (`:141`, `:325`).
- `CreateRole` / `UpdateRole` / `DeleteRole` existent (`Hive/application/src/command/role.rs`) ; le use case refuse le live (`Hive/application/src/usecase/role.rs:71`). Pas de route. OK si ça reste mort.
- Helpers `create_builder_with_*` (`factory.rs:238`) : registries partiels pour tests — surface de dispatch plus large que HTTP si réutilisés hors test.

**Durcissement :** une assertion de test (ou codegen) « chaque `.get/.post/...` a un `register` et inversement pour le public » ; supprimer ou `#[cfg(test)]` les handlers orphelins ; ne jamais monter une route dont le `command_type` n'est pas dans le registry.

### 3. AuthZ — qui dispatch quoi ; IDOR

Middleware : `extract_deepest_resource_id` fait `.rev()` puis le premier UUID (`middleware_permission.rs:33`). Toutes les routes gardées déclarent `"organization"` (`Hive/http/src/lib.rs:40-109`).

| Forme de chemin | UUID soumis au `Check` | UUID métier |
|---|---|---|
| `/organizations/{org}` | org | org — cohérent |
| `/organizations/{org}/members` | org | org — cohérent |
| `/organizations/{org}/members/{user}` | **user** | org + user — **cassé** |
| `/organizations/{org}/roles/{role}` | **role** | org + role — **cassé** |
| `/organizations/{org}/invitations/{inv}` | **inv** | org + inv — **cassé** |
| `/organizations/{org}/sync-jobs` | org (`sync-jobs` n'est pas un UUID) | org — cohérent |

Le wiki [[projects/hive/concepts/organization-resource-authorization]] dit que les sous-ressources « collapse » en checks org **et** que le dernier UUID est l'instance OpenFGA. Les deux phrases ne peuvent pas être vraies ensemble sur les routes à deux UUID.

Sans tuple `organization:{user_id}`, ces routes **403 pour les usages normaux**. Avec un tuple `organization:{X}` (l'org de l'attaquant, ou un user traité comme org), le `Check` passe pour **n'importe quel** `{organization_id}` du chemin. La use case ne revérifie pas (`member.rs:323`, `:374` ; `update_member` non plus).

`list_members` / `get_member` extraient `OptionalAuthUser` alors que la route est `.authenticated()` (`Hive/http/src/handlers/members.rs:53`, `:82`) — incohérence, pas un bypass si le middleware auth est bien posé.

**Durcissement :** extraire l'UUID **d'un nom de paramètre** (`organization_id`), pas « le plus profond » ; ou n'utiliser `with_permission_on` que sur des chemins à un seul UUID et vérifier l'org en use case (membership / `Check` explicite sur `organization_id`) ; interdire de typer `user` comme `organization` dans OpenFGA.

### 4. JWT consumer

`UserIdExtractor` (`jwt_handler.rs:9`, `:71`) :

- algorithmes : **HS256 uniquement** (`Validation::new(Algorithm::HS256)`).
- claims exigés à la main : `sub` (UUID), `exp`, `iat` (lu, **non comparé**), `jti` (non vide, **pas d'anti-rejeu**).
- `validate_nbf = false`.
- pas de `aud`, pas d'`iss`. `JwtAuthConfig` n'a que `hs256_secret` (`rustycog/rustycog-config/src/lib.rs:264`).

Hive câble ce consumer (`Hive/setup/src/app.rs:148`) et le commente : rustycog-http ne vérifie pas le JWKS/RS256 d'IAM. Aligné avec [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].

`default_user_id` sur jeton vide (`jwt_handler.rs:96`) : Hive n'appelle pas `with_default_user_id`. Commentaire trompeur dans `middleware_auth.rs:109` (« no verification ») alors que la signature HS256 **est** vérifiée.

`create_jwt_token` de rustycog-testing (utilisé partout dans `Hive/tests`) émet le même profil HS256 sans audience — les tests ne couvrent pas un refus `aud`.

**Durcissement :** `aud` = `hive` (ou audience plateforme), `iss` = IAM ; vérifier `nbf`/`iat` ; JWKS RS256 partagé ; secret HS256 hors git, distinct par env ; ne pas exiger `jti` sans store de révocation (ou l'utiliser vraiment).

### 5. Confused deputy

- **`iam_service`** (`Hive/configuration/src/lib.rs:16`, configs `base_url` / `api_key`) : **aucun client** dans Hive. Pas d'appel IAM/Manifesto. Slot de crédential mort (clé vide en toml).
- **External Provider** : un `HttpExternalProviderClient` avec `base_url` + `api_key` **service** (`Hive/setup/src/app.rs:504`). Chaque sync pousse le `provider_config` **tenant** dans le body (`external_provider_client.rs:224`) sous Bearer Hive. Le collaborateur exécute avec l'identité Hive.
- **sentinel-sync** : `MemberJoined` / `MemberRolesUpdated` écrivent les relations issues du payload (`sentinel-sync/src/translator/hive.rs:104`, `:119`, mapping `:28`). Hive est le député qui fait écrire OpenFGA.
- **Sync members** : invitations créées avec `organization.owner_user_id` comme invitant (`Hive/domain/src/service/sync_service.rs:197`) — actions au nom du owner.

**Durcissement :** ne pas faire confiance au body d'événement pour `owner`/`admin` sans re-`Check` de l'acteur ; scoped token par tenant vers le provider ; retirer `iam_service` ou s'en servir avec un client à moindre privilège ; le sync ne doit pas impersonner le owner.

### 6. Queue / replay / readiness

Hive est **publisher** : `create_signaled_multi_queue_event_publisher` + `ReadinessProbe::with_publisher` (`Hive/setup/src/app.rs:108`, `:183`) + `attach_ready` (`Hive/http/src/lib.rs:115`). Conforme à [[projects/aiforall/concepts/queue-readiness-signaling]]. Pas de consumer de commandes : **pas** de spoofing inbound « dispatch via queue » dans Hive.

Risques restants : outbox → bus (si le transport est faible, les events Hive sont injectables **vers** Telegraph / sentinel-sync) ; `secret_access_key = "test"` dans `Hive/config/default.toml` et `development.toml`. Pas d'anti-rejeu applicatif sur les events (idempotence côté consommateur, hors Hive).

**Durcissement :** credentials queue hors fichiers default ; `/ready` doit rester **degraded** si le publisher est no-op (déjà signalé — à vérifier en ops) ; consumers aval : idempotence + authn du producteur.

### 7. SSRF / webhooks

Pas d'endpoint webhook inbound. Sorties HTTP :

- client provider vers `base_url` **de config** (reqwest + timeout, redirects par défaut) ;
- `provider_config` / `avatar_url` contrôlés par l'API, non fetchés directement par Hive (avatar stocké ; config **relayée**).

Classe : **SSRF délégué** + URL stockée non allowlistée (`Hive/application/src/dto/organization.rs:29`).

**Durcissement :** allowlist de schémas `https` et d'hôtes pour avatar ; schéma provider (pas d'URL interne) ; `redirect(Policy::none())` sur le client Hive ; le service provider doit borner les fetches.

### 8. Secrets dans commandes / logs / réponses

- Jeton d'invitation : `tracing::info!(..., token)` (`invitations.rs:161`, `:190`) ; champ API `token` (`invitation.rs:35`).
- `debug!("... body: {}", body)` (`external_provider_client.rs:144`).
- `IamServiceConfig` / `AppConfig` : `Debug` dérivé — une clé IAM finirait dans les traces si elle était renseignée.
- HS256 et SQS en clair dans les toml default/dev (`Hive/config/default.toml:12`, `:54`).

Le generateur de jeton (`organization_invitation.rs:150`) est deux UUIDv4 (entropie correcte malgré le `TODO` « pas CSPRNG »). Le problème est la **fuite**, pas l'entropie.

**Durcissement :** ne jamais logger ni renvoyer le jeton (une fois à la création, canal hors bande) ; `Debug` redact sur les configs ; secrets uniquement via env / vault.

### 9. Path traversal / écriture fichier

Aucune commande Hive n'ouvre de chemin utilisateur (`fs::`, `OpenOptions` absents du métier). Axe **non applicable**. `avatar_url` / `settings` sont des données, pas des paths runtime.

### 10. Gardes rustycog mal appliqués

| Attendu rustycog | Hive |
|---|---|
| Un `GenericCommandService` | Oui (`app.rs:146`) |
| `with_permission_on` après le mode auth | Oui sur les routes gardées |
| UUID le plus profond = ressource | **Mal calé** sur members/roles/invitations |
| `object_type` dans `model.fga` | `"organization"` existe (`openfga/model.fga:6`) |
| `Permission::Admin` → relation `administer` | Mapping rustycog OK (`rustycog-permission/src/lib.rs:62`) |
| Skip cache si `cache_ttl_seconds == 0` | Oui (`app.rs:168`) |
| `might_be_authenticated` + permission ⇒ `user:*` | `organization.viewer` **n'a pas** `user:*` (seul `project` l'a). Hive a contourné en **ôtant** la permission sur GET org. |
| Routes sans garde | search, get org, create org, list orgs, accept invitation |

**Durcissement :** GET org = `might_be_authenticated` + `Read` **et** tuples `viewer@user:*` seulement si `visibility=Public` (job sentinel-sync, comme les projects) ; create org : quota / permission plateforme ; list : déjà filtré membership — documenter pourquoi pas de `Check`.

### 11. Confusion de privilège interne

Trois plans qui divergent :

1. **OpenFGA** : `owner` / `admin` / `member` / `viewer` → `own` / `administer` / `write` / `read` (`openfga/model.fga:8-14`).
2. **RBAC Hive SQL** : tables `role_permissions` / `organization_member_role_permissions` encore écrites ; **plus lues** pour décider le HTTP (wiki « What went away »).
3. **HTTP** : `Permission::Write` sur add/update/remove member **et** create invitation (`lib.rs:64-94`). `Admin` seulement sur update/delete org et external-link.

Conséquences : un `Write` invite ou ajoute un `Admin` → event → tuple `admin` (`role_to_org_relation`). Le RBAC SQL peut montrer « read » pendant qu'OpenFGA dit `write`. `MemberRolePermission::Delete` est un variant API refusé à la conversion (`role.rs:76`) — bruit de contrat.

`accept_invitation` : authentifié, **pas** de `Check` org (`lib.rs:101`). Le jeton est la capability ; le `user_id` JWT devient membre **sans** correspondance avec l'email (`aggregate_id`).

**Durcissement :** `Admin` (ou `administer`) pour tout grant `admin`/`owner` ; lier acceptation à l'identité (email/`sub` IAM) ; une source de vérité (OpenFGA) et tests qui échouent si SQL et tuples divergent.

### 12. Tests qui figent un trou

`Hive/tests/members_api_tests.rs:237-283` et `:322-377` : commentaires + `ResourceRef::new("organization", owner_id)` / `read_user_id` — **pas** `org.id`. Le happy path get/delete member n'est vert que si OpenFGA autorise l'**utilisateur** comme organisation.

`Hive/tests/organization_api_tests.rs:92-98` : fige le 200 sans `with_permission_on` sur `GET /api/organizations` (le filtre membership sauve l'impact).

`Hive/tests/organization_api_tests.rs:359` : `GET` org inexistant **sans** Bearer attend 404 — officialise la lecture anonyme.

`Hive/tests/common.rs:22` : pas de `mock_check_any(true)` par défaut — bien. Les allows ciblés sont trop souvent sur le mauvais id.

**Durcissement :** inverser les tests : allow sur `organization:{org.id}` ; les routes à deux UUID doivent 403 si seul `user_id` a un tuple ; ajouter un test de **refus croisé** (allow sur org A, requête org B) **sans** décrire de procédure d'attaque — juste l'assertion 403.

## Malimplémentations (hors CVE-like)

- Hive n'est pas un orchestrateur d'agents : le modèle mental « sandbox / IPC / registry de commandes shell » ne correspond pas au code.
- Double pile d'autorisation (SQL + OpenFGA) après suppression des `.conf` Casbin : le SQL est un vestige qui donne une fausse impression de garde.
- `OrganizationInvitation::new` : « Reserved for future validation ; currently always returns `Ok` » (`organization_invitation.rs:42`).
- `update_description` ignore silencieusement un texte trop long (`organization.rs:69`).
- Pagination `list_members` : `total_count` = taille de la **page** (`member.rs:336`) — pas sécu, contrat API faux.
- Commentaire auth rustycog « no verification » vs vérif HS256 réelle.
- OpenAPI / handlers / registry : le live s'est resserré ; les stubs invitations et rôles restent une dette de câblage (500 si on « complète » la spec sans registry).
- `GetInvitationByTokenCommand` utilisé par deux handlers, **non** dans `register_invitation_commands`.

## Surfaces et hypothèses

**Inclus :** `Hive/**`, composition rustycog HTTP/JWT/permission, `openfga/model.fga`, traducteur `sentinel-sync/src/translator/hive.rs`, configs `Hive/config/*.toml`, tests `Hive/tests/**`.

**Hors Hive (hypothèse notée, pas audit complet) :** fetch SSRF **dans** le service External Provider ; idempotence des consumers Telegraph ; émission IAM RS256 en prod.

**Outils :** skill rustycog + concepts vault ; context-mode (`ctx_index` / `ctx_execute`) ; Grep exact (GrepAI en échec : Ollama embeddings injoignable) ; Serena resté `loading` — pas utilisé, pas de repair dans ce chat.

**Hypothèses d'impact :**

- sentinel-sync consomme vraiment les events Hive en env cible (le code de traduction le prévoit).
- Les UUID user et org partagent le même espace : un id d'org est un second segment de chemin **valide**.
- `default.toml` peut être chargé hors laptop (secret HS256 connu).

## Ce qui est sain

- Un seul bus de commandes HTTP (`GenericCommandService`) ; pas de second dispatcher queue dans Hive.
- Routes live ↔ registry : plus de `/roles` orphelin à 500 (l'invariant de parité est tenu **dans ce sens**).
- `create_role` / `update_role` / `delete_role` refusés au use case s'ils étaient invoqués.
- OpenFGA fail-closed ; tests sans allow-all global (`Hive/tests/common.rs:22`).
- Skip `CachedPermissionChecker` si TTL 0 (`app.rs:169`) — pas de stale allow en test.
- Publisher signalé + `/ready` (`attach_ready`, `ReadinessProbe`).
- `list_organizations` filtre par membership (`organization.rs:411`) malgré l'absence de `Check`.
- Search : `visibility=Public` **ou** membership (`organization_repository.rs:171`) — le trou est le GET par id, pas le search.
- JWT : HS256 pincé (pas `none`) ; `exp` vérifié ; `sub` UUID.
- Invitation : expiration métier (`organization_invitation.rs:75`) ; acceptation passe par un user authentifié (pas d'anonyme).
- Prefix `/hive`, `create_router` / `create_prefixed_router` monolithe-compatibles.
- Mapping `Permission::Admin` → relation `administer` cohérent avec `model.fga`.

---

Audit statique défensif. Pas d'exécution dynamique ni de tentative sur un runtime. Rejouer les tests d'AuthZ **après** correction du binding `organization_id` avant de rouvrir un merge.
