---
title: Audit Red Team défensif IAMRusty — 2026-08-31
tags: [audit, security, iam, red-team, defensive]
date: 2026-08-31
project: iamrusty
---

# Audit Red Team défensif — IAMRusty (IAM)

**Périmètre :** `IAMRusty/` (issuer JWT, auth email/mot de passe, OAuth GitHub/GitLab, refresh, reset, registration) + contrat consommateur `rustycog/rustycog-http` (extracteur / middleware).  
**Méthode :** skills rustycog ; context-mode (`ctx_execute` / `ctx_execute_file`) ; lecture ciblée des surfaces auth. GrepAI embeddings indisponibles (Ollama down), graphe RPG vide, Serena encore en chargement — non utilisés pour le fond.  
**Contrainte :** classes de défauts et hardening uniquement. Aucune procédure d’exploitation.

## Résumé exécutif

Le soupçon de « gros problèmes » est **fondé**. IAMRusty n’est pas un IAM « cassé partout » (Argon2, reset hashé, rotation de refresh, `alg` JWT figé, `sub` registration distinct) mais le **modèle d’identité est trop permissif** : le signup peut **attacher un mot de passe et émettre une session** sur un compte OAuth déjà complet ; le `state` OAuth est du JSON base64 **non signé, sans TTL, nonce jamais persisté** ; le login OAuth **fusionne par email** sans exiger un email fournisseur vérifié. Un attaquant qui vise la prise de compte n’a pas besoin d’un bug crypto : il s’appuie sur ces règles métier.

Autour : JWT **sans `iss`/`aud`**, secret HMAC **partagé et commité**, refresh et tokens fournisseur **en clair** en base, **aucun rate limit**, changement de mot de passe **sans révocation de sessions**, callback OAuth avec **redirect URI hardcodé** (`127.0.0.1:8081`, préfixe `/iam` absent). Verdict : **non déployable en l’état comme source de confiance plateforme**. Priorité : figer le linking (signup + OAuth + state signé), puis JWT audience + secrets hors git, puis sessions (refresh hashés, révocation, rate limit).

## Table des findings

| ID | Sévérité | Localisation | Titre | Classe d’attaque | Impact |
|----|----------|--------------|-------|------------------|--------|
| F01 | Critical | `IAMRusty/domain/src/service/auth_service.rs:339-390` | Signup attache un mot de passe aux comptes existants | Account takeover / confusion d’identité | Session attaquant sur compte victime (surtout OAuth complet sans `password_hash`) |
| F02 | Critical | `IAMRusty/http/src/oauth_state.rs:64-84` ; `handlers/auth.rs:468-493` | State OAuth non authentifié | CSRF OAuth / IDOR linking | Liaison d’un IdP attaquant à n’importe quel `user_id` |
| F03 | Critical | `IAMRusty/domain/src/service/oauth_service.rs:180-198` ; `infra/src/auth/github.rs:236-249` | Fusion de comptes par email OAuth | Account takeover par collision d’email | Login OAuth = session du compte IAM qui a le même email |
| F04 | High | `IAMRusty/infra/src/token/jwt_encoder.rs:192-216` ; `rustycog-http/src/jwt_handler.rs:71-75` | JWT sans `iss`/`aud` | Token confusion / replay inter-services | Tout bearer HS256 plateforme authentifie partout |
| F05 | High | `IAMRusty/infra/src/repository/entity/refresh_tokens.rs:12` ; `domain/src/service/refresh_token_service.rs:97-145` | Refresh en clair, pas de kill-family | Session hijack / reuse | Fuite SQL = sessions longues ; reuse d’un token tourné non détecté |
| F06 | High | `IAMRusty/application/src/usecase/password_reset.rs:362-447` | Reset/changement MDP sans révocation refresh | Persistence of access | Compromission MDP n’invalide pas les sessions déjà émises |
| F07 | High | `IAMRusty/http/src/lib.rs:40-55` (absence) | Pas de rate limit auth | Brute force / stuffing | Login, signup, reset, verify, username sans frein |
| F08 | High | `IAMRusty/http/src/lib.rs:68` ; `handlers/auth.rs:828-867` ; `provider_tokens.rs:12` | Tokens IdP en clair + route « internal » | Credential theft | Tout JWT utilisateur exfiltre le token GitHub/GitLab |
| F09 | High | `IAMRusty/config/production.toml:12-25,42-44` ; `development.toml:24-25` | Secrets et creds commités | Secret exposure | HMAC, OAuth GitHub, `postgres:postgres` dans le dépôt |
| F10 | High | `IAMRusty/http/src/handlers/auth.rs:475-479,1123-1127` | Redirect URI OAuth hardcodé | OAuth mix-up / binding cassé | Échange de code hors contrat config / préfixe `/iam` |
| F11 | High | `IAMRusty/domain/src/service/registration_service.rs:216-327` | Registration complète → session sans email vérifié | AuthZ prématuré | Compte non vérifié obtient access+refresh |
| F12 | Medium | `IAMRusty/http/src/lib.rs:56-59` ; `handlers/auth.rs:1187-1190` | `relink-start` public | CSRF de démarrage OAuth | URL d’autorisation sans identité |
| F13 | Medium | `password_reset.rs:135` ; `refresh_token_service.rs:99` | Jetons dans les logs | Secret leakage | Reset/refresh en clair si niveau debug |
| F14 | Medium | `http/src/error.rs:813-857` ; `auth_service.rs:627-638` | Énumération de comptes | Account enumeration | 409 signup, email non vérifié, verify EmailNotFound |
| F15 | Medium | `auth_service.rs:468-498` | Pas de hash factice au login | Timing oracle | Distinguer email inconnu vs mauvais mot de passe |
| F16 | Medium | `http/src/validation.rs:207-248` | Politique MDP faible | Weak credentials | 8 car. + lettre + chiffre, liste courte |
| F17 | Medium | `auth_service.rs:277-287,572-575` | Tokens de vérif. UUID en clair | Token theft (DB) | Lecture table = prise d’email |
| F18 | Medium | `jwt_encoder.rs:271` ; `jwt_handler.rs:129-137` | `jti` jamais révoqué | Stolen token window | Access 15 min non invalidable |
| F19 | Medium | `application/src/command/signup.rs:17-27` ; `http/src/error.rs:821-826` | Messages d’erreur trop riches | Error leakage | Détails infra/validation renvoyés au client |
| F20 | Medium | `IAMRusty/setup/src/app.rs:235-238` | Checker OpenFGA no-op | Missing AuthZ | Routes « internal » = n’importe quel user JWT |
| F21 | Medium | `infra/src/auth/github.rs:191,217` | URLs IdP issues de la config | SSRF (si config compromise) | `user_url`/`auth_url` non allowlistés |
| F22 | Medium | `tests/utils/oauth.rs:22-31` ; `tests/auth_oauth_callback.rs:81-121` | Tests qui sanctifient le state unsigned | Dangerous-as-intended | Régression de sécu « voulue » |
| F23 | Medium | `configuration/src/lib.rs:319-326` ; configs `type = "plain"` | HMAC par défaut / fichiers | Weak/hardcoded secrets | Secret placeholder + fichiers trackés |
| F24 | Medium | `rustycog-http/src/builder.rs:72-76,263-265` | `pending_auth.take()` | Guard mal branchée | Route oubliée = publique (cas F12) |
| F25 | Low | `registration_token.rs:42` ; `registration_token_service.rs:164-218` | `jti` registration non persisté | Replay jusqu’à complete | Bearer 24 h jusqu’à username choisi |
| F26 | Low | `rustycog-http/src/middleware_auth.rs:109` | Commentaire « no verification » | Doc mensongère | Risque de « correctif » qui retire la vérif |
| F27 | Low | `auth_service.rs:281-284` ; `IAMRusty/Cargo.toml:9-12` | `test-mode` / token statique | Test backdoor (hypothèse) | Feature service non câblée au crate domain |
| F28 | Info | `rustycog-http/src/jwt_handler.rs:16-17,94-97` | `default_user_id` | Auth bypass de labo | IAM n’appelle pas `with_default_user_id` (OK aujourd’hui) |
| F29 | Info | Absence CORS | Pas de `CorsLayer` | — | Fail-closed navigateur ; pas une faille ouverte |
| F30 | Info | Pas de tenant | User plat | — | Pas d’isolation multi-tenant à casser |

## Détail par finding

### F01 — Critical — Signup = prise de compte OAuth / incomplet

**Quoi.** `signup_existing_email` ne refuse que si **à la fois** `password_hash` et `username` sont présents (`auth_service.rs:351-352`). Sinon il **écrit le mot de passe fourni** (`:355-365`) et, si un username existe déjà, **émet access + refresh** (`:368-390`) avec le message « Password authentication added to existing account ».

**Pourquoi c’est dangereux.** Un compte créé via OAuth a typiquement un username et **pas** de `password_hash` (`user.rs:33-39`). La classe d’attaque est : « je connais l’email, je m’inscris, je deviens le second facteur mot de passe et je reçois une session ». Si le compte n’a que l’email (registration incomplète), l’attaquant **écrase** le hash et reçoit un `registration_token`.

**Condition.** Email déjà en base ; compte sans couple (hash + username). Pas besoin d’accès IdP.

**Correction.** Ne jamais attacher un mot de passe à un compte existant sans preuve (session JWT, lien email à usage unique, ou flux « set password » authentifié). Réponse identique au signup « nouvel email » (anti-énumération). Interdire l’émission de tokens sur ce chemin.

### F02 — Critical — State OAuth forgeable (link IDOR)

**Quoi.** `OAuthState` est sérialisé en JSON puis base64 URL (`oauth_state.rs:69-83`). Champs : `operation` (`login` ou `link` + `user_id`) et `nonce`. Pas de MAC, pas d’expiry, nonce **jamais** stocké ni comparé. Le callback public (`lib.rs:54`) décode le state et, pour `link`, appelle `handle_link_callback` avec le `user_id` du state (`auth.rs:486-493`).

**Pourquoi.** La classe est CSRF / IDOR de linking : le callback croit le client sur l’identité cible. Un `nonce` cosmétique ne fait pas un anti-CSRF.

**Condition.** Callback joignable + code OAuth de l’attaquant (son propre GitHub). Le `user_id` cible doit être connu ou devinable (UUID fuite via signup F01 / `/api/me` / events).

**Correction.** State opaque côté serveur (store + TTL + one-time) **ou** JWT/HMAC signé (secret dédié) avec `exp`, `nonce` one-shot, binding `user_id` uniquement si session déjà authentifiée. Le callback **link** doit exiger le même sujet que le state signé, pas un UUID fourni par le client.

### F03 — Critical — Login OAuth fusionne par email

**Quoi.** `find_or_create_user` ignore le provider (`_provider`) et résout l’utilisateur **uniquement** par email (`oauth_service.rs:180-198`). GitHub lit l’email primaire **sans tester `verified`** alors que le DTO a le champ (`github.rs:65-71,236-249`). Le fallback `github_user.email` (email public) n’est pas plus garanti. `registration_service.rs:283` affirme que l’email OAuth est « already verified by the provider » — **faux vis-à-vis du code**.

**Pourquoi.** Collision d’email (email GitHub non vérifié, ou même adresse sur un compte password) = **session du compte existant**. C’est une prise de compte, pas un SSO maîtrisé.

**Condition.** L’IdP renvoie un email qui existe déjà dans IAM, sans preuve IAM que le sujet contrôle cette boîte.

**Correction.** Lier d’abord par `(provider, provider_user_id)`. Ne fusionner par email que si l’email IdP est **vérifié** **et** l’email IAM est vérifié, sinon créer un incomplet + confirmation. Ne jamais logger l’utilisateur complet sur une fusion non prouvée.

### F04 — High — JWT issuer/consumer sans audience ni issuer

**Quoi.** Claims d’accès : `sub`, `username` (souvent `""`), `exp`, `iat`, `jti` — pas de `iss`/`aud`/`typ` (`domain/.../token.rs:7-21`, `jwt_encoder.rs:266-272`). `decode` exige `sub,exp,iat,jti` seulement (`jwt_encoder.rs:197-198`). Côté rustycog, `Validation::new(HS256)` + `required_spec_claims = {exp}` puis lecture manuelle de `sub`/`iat`/`jti` (`jwt_handler.rs:71-150`). Pas de `set_issuer` / `set_audience`.

**Pourquoi.** Tout service qui partage `hs256_secret` accepte n’importe quel token IAM (et inversement si un autre issuer utilise le même secret). Pas de cloisonnement Manifesto / Hive / IAM.

**Condition.** Secret HMAC commun (contrat actuel `http_verifier_auth`, `setup/src/app.rs:230-232`).

**Correction.** `iss` fixe, `aud` par service (ou `typ=access`), validation stricte des deux côtés. À terme RS256 + JWKS côté consommateurs (aujourd’hui `UserIdExtractor` **refuse** RS256, `configuration/src/lib.rs:294-312`).

### F05 — High — Refresh : clair, rotation partielle

**Quoi.** Colonne `token` texte (`refresh_tokens.rs:12`). Rotation atomique existe (`refresh_token_service.rs:138-140`). Si `!is_valid`, simple `InvalidToken` (`:97-100`) — **pas** d’invalidation de toute la famille.

**Pourquoi.** Dump DB = 30 jours de session (`refresh_token_expiration` 2_592_000, `jwt_encoder.rs:59`). Réutilisation d’un refresh déjà tourné ne déclenche pas de lockout.

**Correction.** Stocker un hash (SHA-256 + pepper). Détecter le reuse → `revoke_all_tokens`. Cookie `HttpOnly` + rotation déjà en place.

### F06 — High — Changement de mot de passe sans tuer les sessions

**Quoi.** Reset authentifié et non authentifié mettent à jour le hash et effacent les **reset tokens** (`password_reset.rs:362-368,425-435`). Aucun `revoke_all_tokens` refresh.

**Pourquoi.** Un attaquant qui a déjà un refresh garde l’accès après reset.

**Correction.** Révoquer tous les refresh (et denylist `jti` si vous en ajoutez une) sur tout changement de mot de passe / linking forcé.

### F07 — High — Absence de rate limiting

**Quoi.** Aucun `governor` / quota sur `lib.rs` routes publiques (signup, login, reset, verify, username, OAuth, refresh). Un test note explicitement l’absence (`tests/auth_resend_verification.rs:443`).

**Pourquoi.** Stuffing, énumération accélérée (F14), spam de reset / verify.

**Correction.** Limiter par IP + identifiant (email hashé). Lockout progressif. Captcha ou délai sur reset.

### F08 — High — Tokens fournisseur en clair et exposés

**Quoi.** `provider_tokens.access_token` / `refresh_token` en clair (`provider_tokens.rs:12-13`). `POST /internal/{provider}/token` est seulement `.authenticated()` (`lib.rs:68-69`), handler `AuthUser` → renvoie le token IdP (`auth.rs:828-867`). Pas de permission service, pas de mTLS.

**Pourquoi.** XSS, malware, ou simple client qui appelle l’API = **token GitHub/GitLab** (scopes `user`/`email`). Le nom `internal` n’est pas une frontière.

**Correction.** Chiffrer au repos. Endpoint réservé à un identity de service (token distinct, réseau privé). Ne jamais renvoyer le token IdP à un user token.

### F09 — High — Secrets dans Git

**Quoi.** Fichiers **trackés** :

- `production.toml:12,22,25,43-44` — `hs256_secret` (36 car.), `password` 8 car., replicas `postgres://postgres:postgres@...`
- `development.toml:24-25` — `client_id` format GitHub `Iv23…` (20 car.) + `client_secret` 24 car. (**hypothèse : app OAuth réelle**)
- `default.toml` / `test.toml` — HMAC `type = "plain"`

`.env` et PEM locaux **non trackés** (positif).

**Pourquoi.** Le dépôt devient source de secrets. Un HMAC commité = forge de JWT plateforme (F04).

**Correction.** Placeholders uniquement ; secrets via env / secret manager. Rotater tout secret déjà poussé. Purger l’historique si les valeurs étaient réelles.

### F10 — High — Redirect URI hardcodé (et hors préfixe)

**Quoi.** Commentaire « hardcoded for now » (`auth.rs:475`). GitHub/GitLab → `http://127.0.0.1:8081/api/auth/.../callback` (`:477-478`). Relink : même host/port (`:1123-1127`). Le service réel est préfixé `/iam` (`lib.rs:33,83-84`). La config (`default.toml`) pointe plutôt `localhost:5173/oauth/...`.

**Pourquoi.** Binding redirect cassé : échec en prod (fail-closed, **hypothèse**) **ou** app IdP enregistrée sur une URI de test. Classe OAuth mix-up / confusion d’environnement, pas un open redirect classique (la redirection initiale va vers l’URL d’authorize du provider, `auth.rs:334`).

**Correction.** Une seule source : config `redirect_uri` + `SERVICE_PREFIX`. Interdire le hardcode.

### F11 — High — Session avant vérification email

**Quoi.** `complete_registration` valide le JWT registration, pose le username, **émet access+refresh** (`registration_service.rs:269-327`) sans exiger `user_email.is_verified`. Le login password, lui, refuse les comptes complets non vérifiés (`auth_service.rs:516-518`) — **incohérence**.

**Pourquoi.** Signup → complete → session active avec email non prouvé (phishing d’email, occupation d’identifiant).

**Correction.** Ne délivrer de session qu’après verify, **ou** session restreinte (`email_verified=false` + claims) jusqu’à verify.

### F12 — Medium — `relink-start` sans garde

**Quoi.** Enregistré **avant** `.authenticated()` (`lib.rs:56-61`). `push_current` consomme `pending_auth` (`builder.rs:76`) : cette route part **publique**. Le handler n’a pas d’`AuthUser` (`auth.rs:1187-1190`).

**Pourquoi.** N’importe qui obtient une URL d’autorisation relink. Le callback relink exige un JWT (`:1109`) — l’impact est surtout CSRF de démarrage / abus d’app IdP.

**Correction.** `.authenticated()` sur la route + `AuthUser`.

### F13 — Medium — Jetons loggés

**Quoi.** `debug!("Validating reset token: {}", request.token)` (`password_reset.rs:135`) ; `debug!("Invalid refresh token: {}", refresh_token)` (`refresh_token_service.rs:99`). `development.toml` / `test.toml` en `logging.level = debug`. Signup/login loggent l’email (`auth.rs:634,706`).

**Pourquoi.** Agrégateurs de logs = store de secrets.

**Correction.** Ne jamais logger secrets ; hasher / masquer. Interdire debug en prod.

### F14 — Medium — Énumération

**Quoi.** Signup email complet → 409 « User with this email already exists » (`error.rs:818`). Login compte complet non vérifié → message dédié (`:856`). Verify : `EmailNotFound` vs `InvalidVerificationToken` (`auth_service.rs:627-638`). Username check public (`lib.rs:46`).

**Pourquoi.** Cartographie d’emails / usernames (amplifiée par F07).

**Correction.** Réponses uniformes. Username check derrière un token de registration ou quota strict.

### F15 — Medium — Timing login

**Quoi.** Email inconnu → `InvalidCredentials` **avant** Argon2 (`auth_service.rs:475`). Email connu sans hash → même erreur **sans** verify (`:489`).

**Pourquoi.** Oracle temporel (classe enumeration), plus faible que F14 mais réel.

**Correction.** Hash factice à coût constant si l’utilisateur n’existe pas.

### F16 — Medium — Politique mot de passe

**Quoi.** 8–128, une lettre, un chiffre, petite liste `COMMON_WEAK_PASSWORDS` (`validation.rs:207-248`). Pas de complexité / breach list.

**Correction.** Allonger, interdire les fuites connues (k-anonymity), encourager une passphrase.

### F17 — Medium — Vérification email

**Quoi.** Token = UUID v4 (`auth_service.rs:280-287`), lookup `find_by_email_and_token` (`:574`) — **hypothèse : stockage en clair**.

**Correction.** Hash à la conservation, TTL court, single-use (déjà un cleanup `:608`).

### F18 — Medium — Access token non révocable

**Quoi.** `jti` généré (`jwt_encoder.rs:271`) et exigé à la vérif rustycog, jamais consulté en denylist.

**Correction.** TTL court (déjà 900 s) + denylist/version de credentials sur logout / reset (F06).

### F19 — Medium — Fuite d’erreurs

**Quoi.** Fallback signup mappe `error.to_string()` (`signup.rs:18-27`). Plusieurs `AuthError` recopient `CommandError::Validation { message }` (`error.rs:821-826`). `DomainError::AuthorizationError` peut contenir des détails de clé (`jwt_encoder.rs:119`).

**Correction.** Codes stables côté client ; détails uniquement logs serveur.

### F20 — Medium — AuthZ rustycog no-op

**Quoi.** Commentaire explicite : IAM n’utilise pas `with_permission_on` ; `InMemoryPermissionChecker` (`setup/src/app.rs:235-238`). Les routes internal ne sont pas des permissions objet.

**Pourquoi.** Confusion de rôle « user vs service ». Pas d’élévation RBAC interne (pas de rôles sur `User`, `user.rs:10-27`) mais **pas de barrière service**.

**Correction.** Identité machine pour `/internal/*`. Ne pas exposer ces routes sur l’ingress public.

### F21 — Medium — SSRF / open-redirect (hypothèse config)

**Quoi.** GitHub : redirects HTTP **désactivés** (bon) (`github.rs:18-21`). `user_url` / `auth_url` viennent de la config (`:124-131,191`). `Redirect::to` vers l’URL d’authorize générée (`auth.rs:334`).

**Condition.** Config ou env `IAM_OAUTH_*` contrôlés par un attaquant (compromission ops), pas un user HTTP anonyme.

**Correction.** Allowlist des hosts IdP. Ne pas rendre `auth_url`/`user_url` overridables en prod sans revue.

### F22 — Medium — Tests « state unsigned = valide »

**Quoi.** `create_link_state` fabrique le même JSON+base64 que la prod (`tests/utils/oauth.rs:22-31`). `test_oauth_callback_links_external_account_with_valid_link_state` (`auth_oauth_callback.rs:81-121`) valide le linking via ce state. Le nonce est commenté « for security » (`oauth.rs:72`) alors qu’il n’est pas vérifié.

**Pourquoi.** Toute signature/TTL cassera ces tests — risque de les « réparer » en gardant le trou.

**Correction.** Tests d’échec : state altéré, expiré, rejoué. Helpers qui passent par l’API start, pas un encode local.

### F23 — Medium — HMAC `plain` par défaut

**Quoi.** `JwtConfig::default` secret placeholder (`configuration/src/lib.rs:319-326`). Tous les TOMLs trackés : `type = "plain"`. Vault/GCP **non implémentés** (`:148-156`).

**Correction.** Interdire `plain` hors test (`http_verifier` + assert). Prod = fichier/env/secret manager.

### F24 — Medium — Malimplémentation RouteBuilder

**Quoi.** `.authenticated()` pose `pending_auth` sur la route **courante** ; le prochain `.get/.post` fait `take()` (`builder.rs:72-76,263-265`). Oublier l’appel = route publique. Le skill rustycog dit « mode auth d’abord » — le code IAM fait l’inverse (route puis flag), ce qui marche **seulement** si on n’oublie jamais le flag.

**Correction.** API fail-safe : opt-in public explicite (`.public()`), auth par défaut.

### F25 — Low — Registration JWT 24 h

**Quoi.** `sub = "registration"` (`registration_token.rs:77`) — **pas** utilisable comme access (rustycog parse `sub` en UUID, `jwt_handler.rs:149`). Audience `registration` à la validation (`registration_token_service.rs:185`). `jti` non stocké ; réutilisable jusqu’à `complete`.

**Correction.** One-time store du `jti`. OK de garder `sub` non-UUID (empêche F-confusion, voir « Ce qui est OK »).

### F26 — Low — Commentaire middleware rustycog

**Quoi.** `middleware_auth.rs:109` : « Extract user ID from token (no verification) » alors que `extract_user_id` vérifie HMAC+claims.

**Correction.** Corriger le commentaire pour éviter un « nettoyage » dangereux.

### F27 — Low — Token de vérif. statique (hypothèse)

**Quoi.** `#[cfg(any(test, feature = "test-mode"))]` → `"VALIDATION_TOKEN"` (`auth_service.rs:281-284`). `iam-service` définit `test-mode` (`IAMRusty/Cargo.toml:11`) mais **iam-domain n’a pas** cette feature. En binaire normal, cfg inactif. **Hypothèse :** risque seulement si quelqu’un câble la feature plus tard sans retirer le token fixe.

**Correction.** Ne jamais compiler de secret statique hors `cfg(test)`.

### F28 — Info — `default_user_id`

Présent dans rustycog (`jwt_handler.rs:94-97`). IAM utilise `UserIdExtractor::new` sans défaut (`setup/src/app.rs:232`). Ne pas l’activer hors tests.

### F29–F30 — Info

Pas de CORS (navigateur bloqué, fail-closed). Pas de multi-tenant : un seul espace utilisateurs, pas d’IDOR org.

---

## Malimplémentations (bugs de sécu vs features mal conçues)

| Type | Exemples |
|------|----------|
| **Bug de sécu / trou** | F02 state unsigned ; F10 URI hardcodée ; F12 route publique ; F06 pas de révocation ; F13 logs |
| **Feature mal conçue (intentionnelle et dangereuse)** | F01 « ajouter un password au compte existant » ; F03 SSO-par-email ; F08 endpoint internal user-facing ; F11 session avant verify |
| **Contrat rustycog fragile** | F04 HS256-only sans aud ; F20 checker no-op ; F24 auth opt-in ; F26 commentaire faux |
| **Drift spec / code** | Wiki `iamrusty-runtime-and-security` : state horodaté vs JSON nu ; `[jwt.secret_storage]` vs `[jwt.secret]` ; RS256 docs vs HS256 obligatoire ; routes `/start` vs `/login` (`iamrusty-api-and-auth-flows`) |
| **Tests qui figent le danger** | F22 ; 409 signup documenté comme succès (`tests/auth_email_password.rs:73-77`) ; « no rate limiting in this test » |

`User.update_profile` (`user.rs:92`) pourrait mass-assign username/avatar **s’il était exposé** : **aucune route PATCH user** aujourd’hui — pas de finding mass-assignment HTTP.

Injections SQL/commande : SeaORM, pas de `query(&format!(...))` vu. Path traversal : non vu. Header injection : non vu au-delà des logs.

---

## Surfaces d’attaque / hypothèses non vérifiées

- **Entropie réelle** des HMAC commités (longueurs 25/26/36 seulement).
- **Secret GitHub** de `development.toml` : format `Iv23` = **hypothèse d’app réelle** — à révoquer si confirmé.
- Isolation réseau de `/iam/internal/*` (ingress, mesh). Non lue ici.
- Emails GitHub **toujours** vérifiés côté GitHub pour cette app (le code ne le vérifie pas).
- Consommateurs (Manifesto, Hive, Telegraph) : même secret, même absence d’`aud` (**hypothèse** alignée sur le contrat `http_verifier_auth`).
- Chiffrement disque / backups Postgres (refresh et tokens IdP en clair).
- Filet email (MailHog / Telegraph) : fuite des liens reset / verify.
- Comportement exact de `request_password_reset` si `create` token échoue (anti-enum vs 5xx).
- OpenFGA d’autres services : hors périmètre IAM issuer.

---

## Ce qui est OK (ne pas crier au loup)

- **Argon2id** par défaut, sel `OsRng` (`infra/src/auth/password.rs:1-63`).
- **Reset tokens** hashés SHA-256, 32 car. alphanum, TTL 24 h, `used_at`, lookup par hash (`password_reset_token.rs:13-45`, usecase `:286-292`).
- **Rotation** de refresh (même imparfaite, F05).
- **jsonwebtoken** `Validation::new(HS256|RS256)` : pas d’alg `none`, pas de confusion HS/RS sur un même decode.
- **JWKS vide en HS256** (`jwt_encoder.rs:248-250`) — ne publie pas le secret.
- **Registration `sub` = `"registration"`** : un registration JWT ne passe pas `Uuid::parse_str(sub)` côté rustycog.
- **Resend verification** : message générique côté handler (`auth.rs:795-808`) et service (`auth_service.rs:676-704`).
- **Reset-request** : message générique si le use case réussit ; mapper `password_reset_request_failed` en 200 (`error.rs:1080-1082`).
- **`catch_panic`** : message générique (`rustycog-http/src/lib.rs:36-52`).
- **`/api/me`** est bien derrière `.authenticated()` + `AuthUser`.
- Linking **provider déjà lié à un autre user** → 409 (`auth_oauth_callback.rs:305+`) : une partie du linking est défendue.
- PEM / `.env` locaux non trackés.
- Pas de `default_user_id` en composition IAM.

---

## Priorités de hardening (ordre défensif)

1. **F01 + F03 + F02** — modèle d’identité (signup, email OAuth, state).
2. **F09 + F04** — secrets hors git, `iss`/`aud`, rotation HMAC.
3. **F05 + F06 + F08** — hash refresh, revoke-all, tokens IdP, `/internal`.
4. **F07 + F11 + F10 + F14** — rate limit, verify-before-session, URI OAuth, réponses uniformes.
5. **F24 + F22** — RouteBuilder fail-closed, tests qui interdisent le state forgeable.

---

## Méthode et limites

Analyse statique du monorepo au 2026-08-31. Pas d’exécution dynamique, pas de scan réseau. GrepAI (embeddings) et Serena indisponibles. rustycog lu via le submodule `rustycog/rustycog-http`.
