# Parcours organisation (Hive)

Hive gère les organisations, l’appartenance, les invitations et les jobs de sync vers un provider externe. Préfixe `/hive`. Compose : 8082.

Auth : Bearer IAM. AuthZ : type OpenFGA `organization`. Visibilité **par défaut Private** (GET org authentifié + `Read`).

## Créer et lire

- `POST /hive/api/organizations` — authentifié. Le créateur devient owner ; sentinel-sync pose `organization:{id}#owner@user:{id}` (et relations dérivées) à la réception de `organization_created`.
- `GET /hive/api/organizations` — liste des orgs du caller.
- `GET /hive/api/organizations/{id}` — `Read` sur l’org.
- `GET /hive/api/organizations/search` — public, orgs exposées à la recherche seulement.
- `PUT` / `DELETE` `.../{id}` — `Admin`.

## Membres

- `POST .../members` — `Write` (UUID org = dernier UUID).
- `GET .../members`, `GET .../members/{user_id}` — `Read` ; le GET by user utilise `with_permission_on_param(..., "organization_id")`.
- `PATCH` / `DELETE` `.../members/{user_id}` — `Write` + `*_param`.

Les rôles métier (`owner` / `admin` / `write` / `read`) sont traduits par sentinel-sync en relations FGA (`owner` / `admin` / `member` / `viewer`). Un rôle `Write` **ne pose plus** un tuple `admin` (contrat P0).

## Invitations

Pour un email **sans** compte encore :

1. Admin/membre `Write` : `POST .../invitations` (message, expiry, rôles).
2. Le jeton d’invitation n’est **pas** renvoyé ni loggé en clair dans les réponses de lecture (P1).
3. L’invité (compte créé ensuite) : `POST /hive/api/invitations/{token}/accept` — authentifié, **pas** org-scoped (le token *est* le secret).
4. `DELETE .../invitations/{invitation_id}` — `Write` + `*_param`.

## Liens externes et sync

- `POST .../external-links` — `Admin` : rattache un provider (IAM) à l’org.
- `POST .../sync-jobs` — `Write` : démarre un job de sync (état persisté, suivi asynchrone).

## Événements

`organization_created|updated|deleted`, `member_joined|removed`, `member_roles_updated` → file `sentinel-sync-events`. Sans worker, les Check HTTP restent deny même après un POST réussi.

## Références code

- Routes : [`Hive/http/src/lib.rs`](../../Hive/http/src/lib.rs)
- Wiki : `projects/hive/concepts/invitation-driven-membership`, `organization-resource-authorization`
