# ADR-0407 : IAM authentifie via un contrat fédéré vendor-neutral ; le domaine ne connaît pas GitHub ni GitLab

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Djoé Denne
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune — complète le trou « catalogue des providers » d’[0400](0400-iamrusty-identite-hexagonale.md) sans la réécrire
- SuperSédée par : [0413](0413-oauthconnect-multi-instances-profils.md), **partiellement** pour l'interface d'authentification v2, depuis le 2026-11-06

`Accepted` ratifie une cible. `Réalité` : Implemented. Identité slug (newtype, pas enum `GitHub | GitLab`) : [0411](0411-idp-provider-slug-registry-fail-closed.md).

## Actualisation du 2026-11-06 — interface partiellement supersédée par 0413

L'utilisateur a explicitement accepté [0413](0413-oauthconnect-multi-instances-profils.md) et son [contrat actif](../contracts/oauthconnect-multi-instances.md). Pour les nouveaux consommateurs v2, les portions interface des Décisions 2–3 et 5 (trois handlers/chemins v1 et séquence token puis profil) sont remplacées par `authorize` + `complete` : nonce indépendant durable dans la transaction IAM, puis tokens actuels + profil minimal validé par Connect. La topologie N+1 mentionnée dans les conséquences suit désormais 0413/0408 actualisée. Les descriptions v1 ci-dessous restent historiques, pas une livraison de v2.

Le domaine vendor-neutral, les DTO d'identité actuels, la clé provider+ID externe, le default sans Axum, le transport HTTP et les responsabilités IAM sont conservés. Les claims riches et actions restent différés. Statut **Accepted** et Réalité **Implemented** de cette ADR inchangés pour son périmètre historique ; 0413 est désormais **Accepted / Partial** pour sa première tranche de contrat v2 vérifiée, sans service ni raccord IAM livrés. Les unités/IT autorisées peuvent commencer après intégration documentaire, sans lever les gates E2E/cutover.

## Contexte

IAMRusty est l’IdP **plateforme** (JWT `iss=iamrusty`, comptes, linking) — [0400](0400-iamrusty-identite-hexagonale.md), [0302](0302-authn-jwt-authz-openfga.md). Les clients OAuth GitHub/GitLab sont des adapters **in-process** (`IAMRusty/infra/src/auth/github.rs`, `gitlab.rs`) derrière le port `ProviderOAuth2Client` (`services/IAMRusty/domain/src/port/service.rs`) et la façade application `OAuthService` (`services/IAMRusty/application/src/auth.rs`).

Le handbook `services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md` et le skill wiki `extending-iamrusty-with-oauth-providers` enseignent d’ajouter Google **dans IAM** (enum + client + factory + `ProviderPath`). 0400 a laissé ouvert le catalogue au-delà de GitHub/GitLab.

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
4. **On n’élève pas** `OAuthService` (login vs relink, `redirect_uri` applicatif par requête). Il reste dans `services/IAMRusty/application`.
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

### Limitation connue — claims additionnels et usages délégués (2026-11-06)

Le besoin actuel utilise les fournisseurs externes comme **autorités d’identité**. Le contrat restitue déjà une projection normalisée des informations d’identité (`ProviderUserProfile` : `id`, `username`, `email`, `avatar_url`, `email_verified`), mais **pas les claims additionnels du fournisseur**. `ProviderTokens` ne transporte pas non plus l’ID token OIDC. Sources : `crates/idp-connect-contract/src/{dto,client}.rs`.

- **Évolution identifiée, explicitement différée par l’utilisateur** : permettre de demander les informations/claims supplémentaires nécessaires, puis de les restituer au consommateur du contrat. Leur exposition ne doit pas être implémentée dans la refonte courante des connecteurs à deux profils OIDC/OAuth. Cette note ne fixe ni enveloppe de claims, ni nouvelle méthode, ni regroupement des opérations existantes.
- Pour OIDC, les scopes et, lorsqu’il est supporté, le paramètre `claims` permettent de demander des claims ; leur présence reste soumise aux capacités et politiques du fournisseur. Pour GitHub OAuth, les scopes autorisent l’accès aux informations via ses API : il ne s’agit pas d’un paramètre de claims OIDC standard.
- Les futures actions **au nom de l’utilisateur** sur GitHub/GitLab, déjà hors premier slice en [0408 § Décision 7](0408-connecteurs-idp-services-http.md#décision), restent un besoin distinct : elles dépendent des access tokens et des scopes/permissions accordés. Retourner davantage de claims ne confère pas ces permissions et ne suffit pas à fournir une capacité d’action déléguée.
- Le report concerne l’**exposition des claims additionnels**, pas la validation nécessaire à l’authentification OIDC ni les informations minimales requises par IAM. Les règles de compte/linking, les transactions navigateur et les frontières de confiance [0409](0409-confiance-callback-oauth-idp-connect.md) / [0412](0412-oauth-transaction-persistante-liee-navigateur.md) restent applicables.

Cette limitation est une photographie du contrat et un besoin futur conservé, **pas une extension implémentée ni une modification des décisions ou statuts de cette ADR par cette note**. L’évolution OIDC du contrat, dont le traitement de l’ID token, restait à cadrer lors de sa rédaction ; elle est désormais bornée, pour l'identification minimale seulement, par [0413](0413-oauthconnect-multi-instances-profils.md) **Accepted / Partial**. La première tranche de frontière v2 ne livre pas encore les profils ni leur validation OIDC ; cette ratification séparée ne lève pas le report des claims additionnels/actions.

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
- Code : `services/IAMRusty/domain/src/port/service.rs`, `services/IAMRusty/domain/src/entity/provider.rs`, `services/IAMRusty/application/src/auth.rs`, `services/IAMRusty/domain/src/service/oauth_service.rs`
- ADR : 0100, 0103, 0302, 0400
- Preuve d’implémentation : `idp-connect-contract`, `services/IAMRusty/infra/src/auth/http_connector.rs`, `services/GitHubConnect/`, `services/GitLabConnect/`, IT `services/IAMRusty/tests/fixtures/idp_connect/`
