# ADR-0410 : Extraire GitHub/GitLab d’IAM par étapes ; les routes `/api/auth/{provider}` restent

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Djoé Denne
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Les IT IAM mockent **api.github.com / GitLab** via WireMock (`IAMRusty/tests/fixtures/github`, `gitlab`) — conforme [0201](0201-mocks-http-sortant-seulement.md) aujourd’hui parce que le collaborateur sortant **est** le vendor. Après extraction, le collaborateur sortant d’IAM est le **connecteur**. Mocker le use-case IAM reste interdit ([0200](0200-it-infra-reelle-rustycog-testing.md)).

Pas de big-bang : un bascule unique casserait login OAuth et les fixtures.

## Décision

1. **Pas de breaking UX.** `GET /api/auth/{provider}/login|callback|link|relink` restent sur IAM (préfixe `/iam`). 422 si slug absent du registry.
2. **Ordre** : contrat → adapter HTTP IAM derrière le port → services connecteurs (clients déplacés) → retrait factory / clients in-process / secrets IAM.
3. **IT** :
   - IAM : WireMock (ou testcontainer HTTP) **vers le connecteur** (HMAC 0409) ; **plus** vers `api.github.com`.
   - Connecteur : WireMock GitHub/GitLab (réutiliser les fixtures actuelles, les **déplacer**).
   - Interdit : fake in-process du use-case `OAuthLoginCommand` / `ProviderLinkService`.
4. **v1 enum** : `Provider` GitHub\|GitLab et `ProviderPath` peuvent rester le temps que le registry n’a que ces slugs. Pas de Google dans IAM.

## Conséquences

- Double volant temporaire (in-process **ou** HTTP) autorisé **un** slice, derrière un flag de config `idp.mode = in_process | http`, retiré avant clôture.
- `PROVIDER_FACTORY_GUIDE` et le skill wiki d’extension sont réécrits **après** le code, pour pointer 0407–0409.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Big-bang un PR « tout sort » | Trop de surfaces (setup, IT, compose) |
| Changer les URLs publiques vers le connecteur | 0409 ; casse les OAuth Apps |
| Garder WireMock GitHub dans les IT IAM | Testerait un collaborateur que IAM ne contacte plus (0201) |

## Non décidé ici

- Contrat / topologie / trust (0407–0409).
- Hive comme second client des connecteurs.

## Contrat d’implémentation

0407–0410 sont **Accepted** / **Implemented** (S1–S6). Le volant in-process IAM est retiré.

### Hors scope

- Apparatus P4 / ADR-0008, Lazaret, OpenFGA, `iam-events`, JWT HS256 vs RS256.
- Hive `ExternalProviderClient` / sync membres, webhooks, git API, listing repos.
- Nest monolith, Google/Microsoft, OIDC Id Token.
- Réécriture des ADR 0100–0502.

### Slices ordonnés

**S1 — Crate contrat**  
Ajouter `idp-connect-contract/` (member workspace) : `FederatedOAuthClient`, DTOs, erreurs, constantes de chemins `/v1/authorize|token|profile`, helpers HMAC (spec [0409](0409-confiance-callback-oauth-idp-connect.md) : chaîne `METHOD\nPATH\nTIMESTAMP\nBODY`, `PATH` **avec** préfixe service, fenêtre **30 s**, compare constant-time `hmac` + `subtle`). Feature Cargo **optionnelle** `server` : middleware Axum HMAC + 3 handlers wrapping `Arc<dyn FederatedOAuthClient>` (partagé GitHub/GitLab Connect). Default (sans feature) : DTOs + trait + helpers HMAC, **pas** d’axum. Pas de reqwest GitHub.

**S2 — Adapter IAM HTTP** (feature/config `idp.mode=http` possible en parallèle de l’in-process)  
- `IAMRusty/infra/src/auth/http_connector.rs` implémentant le port 0407.  
- Config `[[idp.connectors]]` (id, base_url, hmac_secret, **`redirect_uris`**).  
- Handler : `redirect_uris` **depuis** le registry (callback **et** relink-callback) ; supprimer les littéraux `127.0.0.1:8081`.  
- `OAuthService` / commandes inchangés.  
- IT IAM : nouvelle fixture WireMock **connecteur** ; les tests `auth_oauth_start` / `auth_oauth_callback` passent sans stub GitHub.

**S3 — GitHub Connect**  
Service `GitHubConnect/` (hexagone mince 0408). Déplacer `IAMRusty/infra/src/auth/github.rs` + config vendor. Compose `github-connect-service` :8085/:8446. HTTP S2S via `idp-connect-contract` feature `server` (HMAC PATH préfixé, 30 s). IT connecteur : fixtures GitHub déplacées depuis IAM. Pas de JWT user, pas de DB, pas d’OpenFGA.

**S4 — GitLab Connect**  
Idem `GitLabConnect/`, ports 8086/8447, fixtures GitLab. Même feature `server` (pas de duplication HMAC/handlers).

**S5 — Retrait in-process IAM**  
- Supprimer `OAuthProviderFactory<GH, GL>`, `infra/src/auth/github.rs` + `gitlab.rs`, `OAuthConfig.github/gitlab`.  
- `setup/src/app.rs` : uniquement adapters HTTP + `register_provider_client`.  
  Post-0411 : la map est injectée à `OAuthService::new` (plus de `register_provider_client`).  
- Retirer `idp.mode=in_process`.  
- Mettre à jour `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` (cible = nouveau connecteur, pas l’enum IAM).  
- Fiches `docs/services/github-connect.md`, `gitlab-connect.md`.

**S6 — Docs wiki skill**  
Réécrire `extending-iamrusty-with-oauth-providers` : « ajouter un IdP = service + ligne registry », plus cross-cutting IAM.

### Fichiers (indicatif)

| Ajouter | Modifier | Retirer (S5) |
|---|---|---|
| `idp-connect-contract/**` | `Cargo.toml` (members) | `IAMRusty/infra/src/auth/github.rs` |
| `GitHubConnect/**`, `GitLabConnect/**` | `IAMRusty/setup/src/app.rs`, `configuration`, `http/src/handlers/auth.rs` | `IAMRusty/infra/src/auth/gitlab.rs` |
| `IAMRusty/infra/src/auth/http_connector.rs` | `IAMRusty/application/.../oauth_provider.rs` → registry | factory générique GH/GL |
| `IAMRusty/tests/fixtures/idp_connect/` | `docker-compose.yml`, `docs/adr/README.md` déjà à jour | fixtures GitHub/GitLab **IAM** (après move) |
| `docs/services/github-connect.md`, `gitlab-connect.md` | `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` | secrets vendor dans TOML IAM |

`ProviderLinkService`, users, emails, JWT, refresh, registration, password, store `provider_tokens` : **ne pas toucher** sauf wiring du port.

### Critères d’acceptation

1. `cargo test -p iam-service --test auth_oauth_start --test auth_oauth_callback` verts avec WireMock **connecteur uniquement**.
2. IT GitHub Connect : token + profile contre WireMock `api.github.com` (0200 serveur réel connecteur + 0201 vendor mocké).
3. Idem GitLab Connect.
4. Login `GET /iam/api/auth/github/login` → 303, `state` décodable par `OAuthState::inspect`, `redirect_uri` login = entrée `.../callback` de `redirect_uris` (pas 8081 hardcodé).
5. Link authentifié inchangé fonctionnellement (0400 login ≠ link).
6. Aucun `client_secret` GitHub/GitLab dans les TOML IAM de prod/dev.
7. `rg GitHubOAuth2Client IAMRusty/` vide après S5.
8. Connecteur : requête sans HMAC → 401 ; bearer JWT IAM ignoré/rejeté.
9. `oodhive-monolith` **non** modifié v1.
10. Clippy/fmt du dépôt sur les crates touchées (`aiforall-sonar-policy`).

### Validation — pas « tout cargo »

- IAM : tests OAuth existants + fixture connecteur.  
- Chaque connecteur : sa crate `tests/`.  
- Un smoke compose optionnel (IAM + un connecteur + WireMock n’est pas requis si les IT couvrent S2–S4).  
- Ne pas allumer les files (0202) ni OpenFGA.

## Références

- ADR 0200, 0201, 0400, 0407, 0408, 0409
- Code : `IAMRusty/tests/auth_oauth_start.rs`, `IAMRusty/tests/auth_oauth_callback.rs`, `IAMRusty/tests/fixtures/github`, `gitlab`
- Skill : `.cursor/skills/creating-wiremock-fixtures/SKILL.md`, `docs/guides/nouveau-service.md`
- Preuve : `aucune`
