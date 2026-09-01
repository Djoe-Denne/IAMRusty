# Parcours projet (Manifesto)

Manifesto gère projets, composants attachés, et membership projet. Préfixe `/manifesto`. Compose : 8083.

## Lifecycles

- Projet : `draft → active → archived`
- Composant : `pending → configured → active → disabled`

Champs projet : ownership, visibilité, flags de collaboration, classification des données.

## Lecture (optionnellement authentifiée)

- `GET /manifesto/api/projects` — filtre selon caller / visibilité (anonyme = publics seulement).
- `GET .../projects/{id}` et `.../details` — `might_be_authenticated` + `Read` sur `project`. Un projet `public` a un tuple `viewer@user:*` (sentinel-sync). Sinon : 403 anonyme.
- Liste composants : `Read` projet. GET d’un composant par `{component_type}` : `with_permission_on_param(..., "project_id")`.

## Écriture

- `POST /api/projects` — authentifié ; bootstrappe l’accès owner tout de suite (ne pas attendre le worker pour le créateur — le use case doit rester utilisable ; FGA suit via `project_created`).
- `PUT` — `Write` ; `DELETE` — `Owner` ; `publish` / `archive` — `Admin`.
- Composants : add / patch status / remove — `Admin` projet. L’ACL instance composant doit rester synchrone : échec de sync ACL → échec de la requête.
- Membres et grants : `Admin` + `*_param` dès qu’un `{user_id}` ou `{resource}` suit `{project_id}`.

`organization_id` / `owner_id` en entrée : refusés sans membership prouvée (contrat P0 Manifesto).

## Catalogue composants

Appel HTTP sortant (`service.component_service`) : fail-closed, `api_key` + timeout honorés. En test : wiremock (taskboard + wiki par défaut).

`ComponentResponse.endpoint` et `access_token` restent `None` (provisioning non implémenté).

## Événements

Publiés vers `sentinel-sync-events`. Manifesto peut **consommer** `component_status_changed` si la queue est enabled. Voir [notifications.md](notifications.md) n’applique pas ici — c’est de l’AuthZ projet, pas du mail.

## Références

- [`Manifesto/README.md`](../../Manifesto/README.md), [`Manifesto/http/src/lib.rs`](../../Manifesto/http/src/lib.rs)
- Guides : [`Manifesto/docs/`](../../Manifesto/docs/)
