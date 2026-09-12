# ADR-0302 : AuthN JWT rustycog-http HS256 ; AuthZ OpenFGA via `with_permission_on`

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : le volet impersonation de `docs/project/Archi.md` (cible historique Project Service)
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Tous les HTTP métier doivent identifier un sujet, puis autoriser une action sur un objet. La tentation est de recoller AuthN et AuthZ dans IAM, ou de relire `docs/project/Archi.md` (impersonation JWT, registry) comme architecture vivante.

Le flux actuel : IAM émet le JWT ; Hive / Manifesto / Telegraph (et l’extractor IAM) vérifient le Bearer ; OpenFGA Check porte l’AuthZ métier.

## Décision

1. **AuthN** = JWT plateforme vérifié par **rustycog-http** (`UserIdExtractor`) en **HS256** (`Authorization: Bearer`). IAMRusty est l’émetteur (`iss=iamrusty`, `aud=aiforall`, même HMAC que les consommateurs).
2. **AuthZ** = **OpenFGA** réelle. Hive, Manifesto et Telegraph câblent les routes avec **`with_permission_on`** / **`with_permission_on_param`** (types du modèle `openfga/model.fga`).
3. **IAM est l’IdP, pas un PDP FGA** : composition root en **`InMemoryPermissionChecker`** (`has_openfga() == false`).
4. L’**impersonation** décrite dans `docs/project/Archi.md` est **caduque** : pas de service d’impersonation, pas de JWT `iss=impersonation-service` dans le runtime livré.

Les IT Hive / Manifesto / Telegraph utilisent `TestOpenFga` (conteneur). Défaut = deny.

## Conséquences

- Un secret HMAC partagé entre IAM `[jwt.secret]` et chaque `[auth.jwt].hs256_secret` tant que l’extractor est HS256-only.
- Typo de type FGA → 403 fail-closed.
- IAM ne Check pas OpenFGA ; ne pas y ajouter `with_permission_on` « pour faire comme Manifesto ».
- Ne plus implémenter l’Impersonation Service d’Archi.md comme cible courante.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| IAM comme PDP (tuples + Check dans l’IdP) | IAMRusty n’a pas OpenFGA ; checker mémoire seulement |
| AuthZ maison dans chaque handler | `RouteBuilder` + OpenFGA déjà le chemin Hive/Manifesto/Telegraph |
| Impersonation JWT / cascading Archi.md | Document historique ; le MVP Manifesto n’expose pas ce runtime |

## Non décidé ici

- Unification émetteur RS256 IAM vs consommateur HS256-only (le composition root refuse déjà RS256 côté HTTP).
- Unification des quatre stratégies de câblage OpenFGA.
- Modèle FGA détaillé (relations, wildcards) au-delà de `with_permission_on` — voir 0303 pour l’écriture des tuples.

## Références

- Handbook : `docs/platform/authn-jwt.md`, `docs/platform/authz-openfga.md`, `docs/platform/overview.md`
- Historique caduc : `docs/project/Archi.md` (section Impersonation Service)
- AuthN : `rustycog/rustycog-http/src/jwt_handler.rs` (`UserIdExtractor`, HS256)
- AuthZ : `Hive/http/src/lib.rs`, `Manifesto/http/src/lib.rs`, `Telegraph/http/src/lib.rs` (`with_permission_on`)
- IAM sans FGA : `IAMRusty/setup/src/app.rs` (`InMemoryPermissionChecker`)
- Preuve : IT `TestOpenFga` Hive/Manifesto/Telegraph ; IAM `has_openfga() == false`
