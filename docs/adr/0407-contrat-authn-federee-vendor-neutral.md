# ADR-0407 : IAM authentifie via un contrat fédéré vendor-neutral ; le domaine ne connaît pas GitHub ni GitLab

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Djoé Denne
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune — complète le trou « catalogue des providers » d’[0400](0400-iamrusty-identite-hexagonale.md) sans la réécrire
- SuperSédée par : —

`Accepted` ratifie une cible. `Réalité` : Implemented. Identité slug (newtype, pas enum `GitHub | GitLab`) : [0411](0411-idp-provider-slug-registry-fail-closed.md).

## Contexte

IAMRusty est l’IdP **plateforme** (JWT `iss=iamrusty`, comptes, linking) — [0400](0400-iamrusty-identite-hexagonale.md), [0302](0302-authn-jwt-authz-openfga.md). Les clients OAuth GitHub/GitLab sont des adapters **in-process** (`IAMRusty/infra/src/auth/github.rs`, `gitlab.rs`) derrière le port `ProviderOAuth2Client` (`IAMRusty/domain/src/port/service.rs`) et la façade application `OAuthService` (`IAMRusty/application/src/auth.rs`).

Le handbook `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` et le skill wiki `extending-iamrusty-with-oauth-providers` enseignent d’ajouter Google **dans IAM** (enum + client + factory + `ProviderPath`). 0400 a laissé ouvert le catalogue au-delà de GitHub/GitLab.

Tentation : étendre l’enum et la factory. Ça fige chaque vendor dans le binaire IdP.

## Décision

1. **IAM reste l’IdP plateforme.** GitHub Connect / GitLab Connect (et tout IdP futur) sont des **federated authenticators**, pas des émetteurs JWT plateforme.
2. **Contrat commun** = crate workspace `idp-connect-contract` :
   - DTOs : `ProviderTokens`, `ProviderUserProfile` (mêmes champs qu’aujourd’hui : `id`, `username`, `email`, `avatar_url`, `email_verified`) ;
   - `provider_id` : slug stable (`github`, `gitlab`, …), **pas** l’enum domaine ;
   - trait Rust `FederatedOAuthClient` (successeur de `ProviderOAuth2Client`) ;
   - helpers HMAC (spec [0409](0409-confiance-callback-oauth-idp-connect.md)) **sans** Axum dans le default ;
   - feature Cargo **optionnelle** `server` : middleware Axum HMAC + **3 handlers** wrapping `Arc<dyn FederatedOAuthClient>` — pour ne pas dupliquer le S2S HMAC/handlers entre GitHub Connect et GitLab Connect. Default (sans feature) = DTOs + trait + helpers HMAC, **pas** d’axum.
3. **Transport IAM ↔ connecteur** = **HTTP JSON synchrone** documenté par le crate (chemins `/v1/authorize`, `/v1/token`, `/v1/profile`). Pas gRPC. Pas d’events (0300 = faits métier, pas un aller-retour OAuth). Hexagone [0100](0100-services-metier-hexagonaux-rustycog.md) / [0103](0103-ports-adapters-command-factory.md) : le domaine IAM dépend du trait ; l’I/O est un adapter outbound.
4. **On n’élève pas** `OAuthService` (login vs relink, `redirect_uri` applicatif par requête). Il reste dans `IAMRusty/application`.
5. **On remplace** `ProviderOAuth2Client` comme port public long terme. Le trait actuel omet `state` et `redirect_uri` (baked dans le client `oauth2::BasicClient`). Le successeur les **exige** :

   | Méthode | Entrée | Sortie |
   |---|---|---|
   | `authorize` | `redirect_uri`, `state` | `authorization_url`, `scope` |
   | `exchange_code` | `code`, `redirect_uri` | `ProviderTokens` |
   | `user_profile` | access token | `ProviderUserProfile` |

   Le `redirect_uri` **par requête** est choisi parmi le tableau registry `redirect_uris[]` ([0408](0408-connecteurs-idp-services-http.md), allowlist [0409](0409-confiance-callback-oauth-idp-connect.md) §6).
6. **Subject mapping** : clé d’identité fédérée = `(provider_id, provider_user_id)` où `provider_user_id` = `ProviderUserProfile.id`. `ProviderLink` / `ProviderLinkService` restent IAM.
7. **Mapping claims** : le connecteur traduit le JSON vendor → `ProviderUserProfile`. IAM ne parse pas GitHub/GitLab.
8. **v1 IAM** peut garder l’enum `Provider` GitHub\|GitLab comme *adapter* de routes `/api/auth/{provider}` et du schéma SQL. Le contrat et le registry sont des slugs. L’enum fermé n’est plus la cible.

Hors décision : topologie processus ([0408](0408-connecteurs-idp-services-http.md)), confiance ([0409](0409-confiance-callback-oauth-idp-connect.md)), plan de migration ([0410](0410-migration-iam-connecteurs-idp.md)).

## Conséquences

- Un nouvel IdP = nouveau connecteur + entrée de registry, pas une variante d’enum dans le domaine IAM.
- IAM continue de **persister** `ProviderTokens` (store actuel). Les tokens vendor transitent sur le canal de confiance 0409.
- `PROVIDER_FACTORY_GUIDE` cesse d’être la cible (remplacement documentaire = 0408 / 0410).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Garder `ProviderOAuth2Client` dans `iam-domain` et l’implémenter in-process | Le domaine IAM resterait couplé aux vendors ; c’est le chemin actuel |
| gRPC | Stack nouvelle ; 0201/IT = HTTP WireMock |
| Events NATS/SQS pour authorize/token | 0300 n’est pas un RPC ; OAuth est synchrone |
| Élever `OAuthService` dans la crate contrat | Mélange façade login/relink IAM et authenticator fédéré |
| OIDC complet (Id Token signé, IAM ne voit jamais le code) | Cible plus propre, trop large pour le premier slice — non décidé ici |

## Non décidé ici

- Topologie crate vs microservice (0408).
- Callback navigateur, secrets, CSRF, HMAC (0409).
- Ordre d’extraction et tests (0410).
- Unification JWT HS256 consommateur vs RS256 IAM : tranchée par [0304](0304-jwt-acces-plateforme-rs256-jwks.md) (réalité runtime encore HS256).
- Contrat étendu org-sync / git API (Hive `ExternalProviderClient`).

## Références

- Wiki : `projects/iamrusty/concepts/oauth-provider-linking.md`, `projects/iamrusty/skills/extending-iamrusty-with-oauth-providers.md`
- Code : `IAMRusty/domain/src/port/service.rs`, `IAMRusty/domain/src/entity/provider.rs`, `IAMRusty/application/src/auth.rs`, `IAMRusty/domain/src/service/oauth_service.rs`
- ADR : 0100, 0103, 0302, 0400
- Preuve d’implémentation : `idp-connect-contract`, `IAMRusty/infra/src/auth/http_connector.rs`, `GitHubConnect/`, `GitLabConnect/`, IT `IAMRusty/tests/fixtures/idp_connect/`
