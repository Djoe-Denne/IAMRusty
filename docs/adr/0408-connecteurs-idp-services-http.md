# ADR-0408 : GitHub Connect et GitLab Connect sont des services HTTP ; IAM ne lie plus les vendors in-process

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Djoé Denne
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune ADR — **remplace comme cible** `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` et le skill wiki d’extension in-process
- SuperSédée par : —

## Contexte

Composition actuelle : `OAuthProviderFactory<GH, GL>` (`IAMRusty/application/src/usecase/factory/oauth_provider.rs`), config typée `OAuthConfig { github, gitlab }` (`IAMRusty/configuration/src/lib.rs`), `setup/src/app.rs` instancie quatre clients (login+link × GH/GL) et `register_provider_client`. Dual runtime [0404](0404-runtime-microservices-et-monolithe.md) : standalones compose + nest `oodhive-monolith`.

Un nouveau service HTTP plateforme suit `docs/guides/nouveau-service.md` (events, OpenFGA, nest, `SERVICE_PREFIX`). Les connecteurs du premier slice sont des **authentifiers S2S**, pas des BC métier user-facing.

Hive a déjà `ExternalProviderClient` (`Hive/domain/src/port/service.rs`, `HttpExternalProviderClient`) pour **sync d’org** GitHub/GitLab/Confluence — autre contrat, hors slice.

## Décision

1. **v1 = crate contrat in-repo + services connecteurs HTTP indépendants** : `idp-connect-contract`, `GitHubConnect`, `GitLabConnect`. Pas seulement des crates liées dans le binaire IAM.
2. **Rejet explicite** de l’extension in-process (enum + `GitHubOAuth2Client` dans IAM + factory générique). Google (ou tout N+1) = **nouveau service** derrière 0407, pas une variante IAM.
3. **IAM consomme un registry** typé, pas un enum de composition :

   ```toml
   [[idp.connectors]]
   id = "github"
   base_url = "https://github-connect:8446/github-connect"
   hmac_secret = "…"          # 0409
   redirect_uris = [
     "https://<iam-public>/iam/api/auth/github/callback",
     "https://<iam-public>/iam/api/auth/github/relink-callback",
   ]
   ```

   Le champ registry est **`redirect_uris`** (tableau), **pas** un `redirect_uri` unique. Il **doit** contenir le callback **et** le relink-callback (les deux URIs encore littérales `127.0.0.1:8081` dans `IAMRusty/http/src/handlers/auth.rs` jusqu’au slice S2 — [0410](0410-migration-iam-connecteurs-idp.md), allowlist [0409](0409-confiance-callback-oauth-idp-connect.md) §6). Le `redirect_uri` envoyé à `authorize` / `exchange_code` (0407) est **choisi** dans ce tableau.

   v1 : la liste peut ne contenir que `github` et `gitlab` ; `ProviderPath` peut encore matcher ces slugs. Long terme : IdP supplémentaire = ligne de config + service, sans recompiler le domaine IAM.
4. **Dual runtime 0404** : v1 = **standalones compose seulement**. **Pas de nest** dans `oodhive-monolith`. Les connecteurs n’exposent pas de routes navigateur ; le monolithe n’a rien à préfixer pour l’UX. Une ADR future pourra nider si un runtime unique devient obligatoire.
5. **Forme crate** : hexagone **mince** (configuration, domain mapping vendor→profil, infra HTTP vendor, http S2S, setup, tests). **Exceptions** à la checklist `nouveau-service` v1 :
   - pas d’OpenFGA, pas de `UserIdExtractor` / bearer IAM utilisateur (0409) ;
   - pas de crate `*-events`, pas de translator `sentinel-sync` ;
   - pas de Postgres / `create-databases` (connecteur **stateless**) ;
   - pas de nest monolith.
   - **Oui** : member `Cargo.toml` racine, service `docker-compose.yml`, `SERVICE_PREFIX` + `create_router` / `create_prefixed_router`, HTTPS mesh T14b, fiche `docs/services/`.
   - HTTP S2S : les connecteurs activent la feature optionnelle **`server`** de `idp-connect-contract` (middleware Axum HMAC + 3 handlers wrapping `Arc<dyn FederatedOAuthClient>`, [0407](0407-contrat-authn-federee-vendor-neutral.md)). Default de la crate contrat = DTOs + trait + helpers HMAC, **sans** axum.
6. **Ports compose** (Lazaret occupe déjà 8084) : GitHub Connect **8085** HTTP / **8446** HTTPS ; GitLab Connect **8086** / **8447**. Préfixes `/github-connect`, `/gitlab-connect`.
7. **Hors premier slice** : org sync Hive, git API, webhooks, listing repos, « extras » IdP. Hive pourra plus tard appeler le **même** connecteur via un contrat **étendu** ; on ne le dessine pas ici.

## Conséquences

- `OAuthProviderFactory<GH, GL>` disparaît en fin de 0410. `setup` IAM enregistre des adapters HTTP 0407, un par entrée de registry.
- Secrets `client_id` / `client_secret` / URLs vendor **sortent** de `IAMRusty/configuration`.
- Un connecteur down ≠ IAM down pour mot de passe / JWT ; seul le login OAuth de ce slug échoue.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Crates GH/GL liées dans le binaire IAM | Les vendors restent dans le processus IdP via `setup` — le problème actuel |
| Un seul service « IdP Connect » multi-tenant | Couplage des secrets et des blasts ; N IdP = N déploiements est le but |
| Nest monolith v1 | Pas de surface user ; 0404 n’oblige le nest que si le service vit dans les deux modes |
| Checklist nouveau-service complète (FGA, events, DB) | Authn S2S stateless ; FGA/events violeraient 0400 (IAM n’est pas OpenFGA) et 0300 |

## Non décidé ici

- Contrat (0407) et frontières de confiance (0409).
- Migration (0410).
- Quand nider les connecteurs dans le monolithe.

## Références

- Code : `IAMRusty/application/src/usecase/factory/oauth_provider.rs`, `IAMRusty/setup/src/app.rs`, `IAMRusty/configuration/src/lib.rs`, `Hive/domain/src/port/service.rs` (`ExternalProviderClient`)
- Docs : `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`, `docs/guides/nouveau-service.md`, ADR 0400, 0404
- Wiki : `projects/iamrusty/skills/extending-iamrusty-with-oauth-providers.md`
- Preuve : `GitHubConnect/`, `GitLabConnect/`, `IAMRusty/infra/src/auth/http_connector.rs`, HMAC `idp-connect-contract`, IT `IAMRusty/tests/fixtures/idp_connect/`
