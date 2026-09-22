# ADR-0409 : IAM possède callback navigateur, CSRF et linking ; le connecteur possède secrets vendor et l’appel OAuth

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Djoé Denne
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Flux actuel : le navigateur frappe IAM `/api/auth/{provider}/login|callback|link` (`IAMRusty/http/src/handlers/auth.rs`). `OAuthState` (`IAMRusty/http/src/oauth_state.rs`) porte opération + nonce + `exp` (TTL 600 s), HMAC, anti-rejeu — **le wiki CSRF est périmé** (il dit « pas de timestamp »). Le callback **hardcode** encore `http://127.0.0.1:8081/api/auth/{github|gitlab}/callback` (IAM compose = 8080). `client_secret` vit dans la config IAM.

T14b : HTTPS compose, CA mesh, **client cert optionnel** (`IAMRusty/tests/https_mesh_optional_mtls.rs`). 0004 interdit le bearer IAM sur un plugin Apparatus — autre frontière ; ici le risque est d’envoyer un JWT **utilisateur** au connecteur ou de laisser le connecteur mentir l’identité.

## Décision

### 1. Callback navigateur = IAM

L’OAuth App vendor enregistre le `redirect_uri` **public IAM** : `/iam/api/auth/{provider}/callback` (préfixe 0404). GitHub/GitLab redirigent le **navigateur** vers IAM. Le connecteur **n’est pas** user-facing. Routes 0400 inchangées.

IAM envoie ce `redirect_uri` (valeur **unique par requête**, choisie dans `redirect_uris[]` — [0408](0408-connecteurs-idp-services-http.md)) au connecteur à `authorize` et `exchange_code`. Le connecteur **allowliste** la valeur (égalité exacte avec **une** entrée de `redirect_uris[]`) — pas d’open-redirect.

### 2. Secrets vendor = connecteur seulement

`client_id`, `client_secret`, auth/token/user URLs, scopes : config du connecteur. IAM ne les charge plus. IAM voit encore : le `code` (un hop, secret, jamais logué) et les `ProviderTokens` renvoyés (store IAM inchangé).

### 3. CSRF / `state` = IAM

IAM mint/valide `OAuthState` (opération login|link, nonce, `exp`, HMAC, one-time nonce). Le connecteur **propage** `state` tel quel vers le vendor ; il n’invente pas de CSRF utilisateur. Invariant : `decode` refuse unsigned / `exp==0` / expiré / replay — **déjà dans le code**, à conserver.

### 4. Account linking = IAM

`ProviderLinkService` (conflits, emails secondaires, relink) reste IAM. Le connecteur ne connaît pas `user_id` plateforme.

### 5. Confiance IAM ↔ connecteur (v1)

| Contrôle | Règle v1 |
|---|---|
| Transport | HTTPS mesh T14b en compose/prod (comme `iam-service`) |
| mTLS client | **Optionnel** (T14b) — **insuffisant seul** |
| AuthZ applicative | **HMAC-SHA256 partagé par connecteur**. Headers : `X-IdP-Connect-Timestamp` + `X-IdP-Connect-Signature`. Chaîne canonique **exacte** : `METHOD\nPATH\nTIMESTAMP\nBODY` (quatre parties séparées par des newlines **littéraux**, sans espaces autour des `\n`). `PATH` = chemin **reçu avec** préfixe service (ex. `/github-connect/v1/token`), **pas** le chemin non préfixé `/v1/token`. Fenêtre d’horloge **30 s**. Comparaison **constant-time** (crates `hmac` + `subtle`). Secret dans le registry IAM **et** la config connecteur |
| Identité utilisateur | **Interdit** : bearer JWT IAM / `UserIdExtractor` sur les routes connecteur. Le connecteur n’est pas un consommateur 0302 |
| Assertion | Le body JSON 0407 **est** l’assertion, authentifiée par HMAC. Pas de JWT utilisateur émis par le connecteur |
| Code OAuth | Traité comme secret sur IAM et connecteur ; pas de log |

Prod : HMAC obligatoire, HTTPS obligatoire. IT : HMAC + HTTP loopback acceptable derrière WireMock.

### 6. Redirect hardcodé 127.0.0.1:8081

Aujourd’hui `IAMRusty/http/src/handlers/auth.rs` contient encore des littéraux `127.0.0.1:8081` pour `/callback` **et** `/relink-callback`. Cible : champ registry **`redirect_uris`** (tableau, [0408](0408-connecteurs-idp-services-http.md)), **pas** un `redirect_uri` unique. Le tableau **doit** inclure le callback **et** le relink-callback. Ces littéraux disparaissent au slice **S2** ([0410](0410-migration-iam-connecteurs-idp.md)). Pas un breaking d’URL publique si l’OAuth App reste pointée sur les callbacks IAM.

## Conséquences

- Compromission d’IAM n’exfiltre plus `client_secret` GitHub (elle peut encore rejouer un `code` vivant et lire des tokens stockés).
- Compromission du connecteur = secrets vendor + capacité à forger un profil **si** l’HMAC IAM est volé. Rotation = secrets HMAC + OAuth App.
- Le wiki `oauth-state-and-csrf-protection` doit cesser de nier `exp` / HMAC.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Callback navigateur sur le connecteur | Change l’UX ; second hop ; surface publique × N |
| mTLS required like-Lazaret comme seul trust | T14b client cert **optionnel** aujourd’hui ; on ne durcit pas tout le mesh dans ce chantier |
| JWT d’assertion connecteur (mini-OIDC) | Plus propre, pas nécessaire v1 si HMAC+HTTPS |
| Bearer IAM utilisateur vers le connecteur | Le connecteur n’a aucune AuthZ user ; confusion IdP/authenticator |
| Laisser les `client_secret` dans IAM « pour simplifier l’échange » | Annule 0408 |

## Non décidé ici

- OIDC Id Token pour que IAM ne voie plus code/tokens vendor (évolution 0407).
- mTLS **required** mesh-wide (plateforme, pas IAM-IdP).
- Stockage at-rest des `provider_tokens` (reste IAM ; hors chiffrement field-level).

## Références

- Code : `IAMRusty/http/src/oauth_state.rs`, `IAMRusty/http/src/handlers/auth.rs` (callback 127.0.0.1:8081), `IAMRusty/tests/https_mesh_optional_mtls.rs`
- Wiki : `projects/iamrusty/concepts/oauth-state-and-csrf-protection.md` (écart : struct a `exp` + HMAC + replay), `projects/aiforall/concepts/https-platform-mesh.md`
- ADR : 0400 (login ≠ link), 0302, 0004 (bearer plugin — analogie, pas copie)
- Preuve : HMAC S2S `idp-connect-contract` + `IAMRusty/infra/src/auth/http_connector.rs` ; CSRF `IAMRusty/http/src/oauth_state.rs` ; IT `IAMRusty/tests/fixtures/idp_connect/` ; `GitHubConnect/` + `GitLabConnect/`
