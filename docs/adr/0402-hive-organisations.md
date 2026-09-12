# ADR-0402 : Hive possède organisations, invitations, membres et liens externes

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

La plateforme sépare l’identité (IAM, 0400) du **contexte organisation**. La tentation est d’y mettre les projets (Manifesto) ou de traiter Hive comme un annuaire IAM.

Hive expose déjà un hexagone `/hive` : organisations, rôles, membres, invitations, liens externes, sync jobs.

## Décision

1. **Hive** est le bounded context **organisations**. Préfixe `/hive` (compose 8082). JWT consommateur `[auth.jwt]` (`iss=iamrusty`, `aud=aiforall`).
2. Le périmètre HTTP livré est :
   - CRUD / search / list d’organisations ;
   - **membres** (`/api/organizations/{id}/members`) ;
   - **invitations** (création / annulation org-scopées ; `POST /api/invitations/{token}/accept` authentifié, pas org-scopé) ;
   - **liens externes** (`POST …/external-links`, admin org) et **sync jobs**.
3. **OpenFGA type `organization`**. Les routes imbriquées lient `{organization_id}` via `with_permission_on_param` pour ne pas prendre le dernier UUID (`user_id`, `invitation_id`) comme objet.
4. Events de cycle de vie org/membres → `hive-events` / `sentinel-sync-events`. Hive n’est pas l’IdP et n’est pas le registre des projets.

## Conséquences

- Un projet Manifesto n’appartient pas à Hive ; le lien org↔projet n’est pas inventé ici.
- Accepter une invitation ne Check pas l’org via le token URL ; le caller est un user JWT.
- Un lien externe (provider) est une ressource Hive, pas un compte OAuth IAM (0400).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Organisations dans IAM | IAM est l’IdP ; pas de tuples org (0400) |
| Projets dans Hive | Manifesto possède projets / composants / membership projet (0401) |
| Invitations anonymes sans JWT | `accept` est `.authenticated()` dans le routeur actuel |

## Non décidé ici

- Transfert de projet / changement d’organisation (`APP-07`, 0001).
- Fidélité OpenAPI Hive vs implémentation (écart déjà noté en 0100).
- Catalogue des providers externes et politique de sync.

## Références

- Wiki : `projects/hive/hive`
- Handbook : `docs/services/hive.md`, `docs/functional/organisation.md`, `docs/guides/permissions.md`
- Code : `Hive/http/src/lib.rs`, `Hive/domain/src/service/external_provider_service.rs`, `Hive/infra/src/repository/organization_invitation_repository.rs`
- Preuve : routeur org/membres/invitations/external-links + Check `organization` ; events → sentinel-sync
