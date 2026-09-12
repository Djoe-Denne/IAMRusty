# ADR-0400 : IAMRusty est l’IdP hexagonal (OAuth, tokens) ; il n’est pas OpenFGA

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

La plateforme a besoin d’une autorité d’identité (comptes, OAuth, JWT) distincte du PDP d’autorisation. La tentation est de faire d’IAMRusty un writer OpenFGA (orgs, projets, notifications) ou de fusionner « se connecter avec un provider » et « lier un provider à un compte déjà authentifié ».

Le handbook (`docs/services/iamrusty.md`) et les routes `/iam` montrent déjà un IdP qui n’écrit pas les tuples org/projet.

## Décision

1. **IAMRusty** est l’**IdP** hexagonal (ports, adapters, `setup`) : signup/login mot de passe, JWT émetteur, refresh, reset MDP, JWKS.
2. **IAMRusty n’est pas OpenFGA.** Il n’écrit pas les tuples `organization` / `project` / `component` / `notification`. L’AuthZ instance vit chez Hive, Manifesto, Telegraph + `sentinel-sync` (0302, 0303).
3. **OAuth login et OAuth link sont des flux séparés** :
   - login (non authentifié) : `GET /api/auth/{provider_name}/login` → callback → session / tokens (`OAuthLoginCommand`) ;
   - link (authentifié) : `GET /api/auth/{provider_name}/link` (et relink) → attache un provider à un compte existant.
4. IAM **publie** `user_signed_up` / `user_email_verified` / `password_reset_requested` vers le contrat `iam-events` (consommé par Telegraph, 0403). Il n’est pas le service de notification.

Préfixe runtime : `/iam` (compose 8080). JWT émetteur : `[jwt]` + `[jwt.secret]` (HS256 aujourd’hui). JWT consommateur de ses propres routes (`/api/me`, …) : `[auth.jwt]`.

## Conséquences

- Un nouveau métier ne clone pas IAM pour du CRUD + OpenFGA (gabarit = Manifesto, 0100 / 0401).
- Relier un GitHub/GitLab à un compte déjà ouvert n’emprunte pas le start URL de login.
- Les Check OpenFGA des autres services restent fail-closed sans store ; IAM n’a pas à « réparer » l’AuthZ.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| IAM writer OpenFGA (tuples user/org) | Mélange IdP et PDP ; le dépôt sépare déjà émetteur JWT et `sentinel-sync` |
| Un seul flux OAuth « login ou link selon le cookie » | CSRF et comptes liés divergents ; le code a déjà deux commandes et deux routes |
| IAM = service de mail | Telegraph consomme `iam-events` (0403) |

## Non décidé ici

- Unification HS256 local vs RS256 / JWKS consommateur (0302).
- Catalogue des providers au-delà de GitHub/GitLab déjà câblés.
- Host Apparatus et bearer IAM interdit sur le plugin (0004).

## Références

- Wiki : `projects/iamrusty/concepts/hexagonal-architecture.md`, `projects/iamrusty/concepts/oauth-provider-linking.md`, `projects/iamrusty/references/iamrusty-api-and-auth-flows.md`
- Handbook : `docs/services/iamrusty.md`, `docs/platform/authn-jwt.md`, `docs/functional/identite.md`, `docs/reviews/iam-rusty-architecture.md`
- Code : `IAMRusty/http/src/lib.rs` (`oauth_login_start` vs `oauth_link_start`), `IAMRusty/application/src/command/oauth_login.rs`, `IAMRusty/application/src/usecase/oauth.rs`, `iam-events/`
- Preuve : routes login/link distinctes ; aucun client OpenFGA dans IAM ; events IAM → Telegraph
