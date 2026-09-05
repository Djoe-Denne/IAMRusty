# Parcours projet (Manifesto)

Manifesto gère projets, composants attachés, et membership projet. Préfixe `/manifesto`. Compose : 8083.

## Lifecycles

- Projet : `draft → active → archived`
- Composant : `pending → configured → active → disabled`

Champs projet : ownership, visibilité, flags de collaboration, classification des données.

`visibility` : `private` (défaut) / `internal` / `public`. **`public` = lecture monde (`viewer@user:*`).** `internal` org-owned = lecture des membres org (`viewer@organization:{id}#member`). Privé org-owned = `project_member` ou admin org. Un user IAM **n’est pas obligé** d’être dans une org. `POST /api/projects/{id}/join` permet l’auto-adhésion sur un public live (`write` / `project`). L-PARTNERSHIP reste ouvert. `external_collaboration_enabled` est persisté et **n’autorise rien**.

## Lecture (optionnellement authentifiée)

- `GET /manifesto/api/projects` — filtre **SQL** : public live, ou `project_member`, ou org Internal/public pour un viewer org, ou tout projet org-owned non filtré autrement pour un admin org. Un `public`+`archived` n’apparaît pas en liste anonyme.
- `GET .../projects/{id}` — `might_be_authenticated` + `Read` OpenFGA, puis garde use case : lecture monde si public live ; sinon membre projet, créateur, viewer org sur Internal, admin org sur privé. Plus de `viewer from organization` : un membre org ne lit plus le privé par UUID.
- Tuple `viewer@user:*` : écrit par **sentinel-sync** à la création si `visibility=public`, et sur un flip réel via `ProjectVisibilityChanged`. Manifesto n’écrit pas FGA. `publish` (lifecycle `active`) ne touche plus le wildcard. `ProjectArchived` le retire.

## Écriture

- `POST /api/projects` — authentifié ; bootstrappe **seulement le créateur** en owner (`MemberSource::Direct`).
- `POST /api/projects/{id}/join` — JWT, public live, `MemberSource::Invitation`, 409 si déjà membre. `POST /members` reste Admin-only.
- Org-owned : `owner_type=organization` + `owner_id`. Contrôle P0 = OpenFGA `Write` sur `organization:{id}`, pas un appel Hive.
- `PUT` — `Write` pour les métadonnées ; un flip **impliquant `public`** exige aussi `Admin` (use case). `private↔internal` reste `Write`. Un flip réel émet `ProjectVisibilityChanged`. `DELETE` — `Owner` ; `publish` / `archive` — `Admin`.
- Composants : add / patch status / remove — `Admin` projet. L’ACL instance composant doit rester synchrone : échec de sync ACL → échec de la requête.
- Membres et grants : `Admin` + `*_param`. Self-join public via `POST .../join`. L-PARTNERSHIP ouvert.

## Limites à travailler plus tard

IDs stables — ne pas les redébattre ici, les implémenter plus tard :

| ID | Limitation |
|---|---|
| L-USER-NO-ORG | **Fait** : join public sans org. |
| L-PUBLIC-PARTICIPATE | **Fait** : join = membership `write`. |
| L-PARTNERSHIP | Pas de graphe org↔org ; un feeder parmi d’autres, pas le seul. |
| L-JOIN-PRIMITIVE | **Fait** : `POST .../join`. |
| L-LIST-GET / L-INTERNAL-FLAG | **Fait** : liste org alignée ; Internal userset live. |
| L-PUBLISH-NE-PUBLIC / L-VISIBILITY-FGA | Corrigés : publish n’écrit plus `user:*` ; PUT visibilité sync FGA via `ProjectVisibilityChanged`. |

Détail : wiki `projects/manifesto/concepts/org-owned-visibility-and-participation-limits` (section Later work).

## Catalogue composants

Appel HTTP sortant (`service.component_service`) : fail-closed, `api_key` + timeout honorés. En test : wiremock (taskboard + wiki par défaut).

`ComponentResponse.endpoint` et `access_token` restent `None` (provisioning non implémenté).

## Événements

Publiés vers `sentinel-sync-events`. Manifesto peut **consommer** `component_status_changed` si la queue est enabled. Voir [notifications.md](notifications.md) n’applique pas ici — c’est de l’AuthZ projet, pas du mail.

## Références

- [`Manifesto/README.md`](../../Manifesto/README.md), [`Manifesto/IMPLEMENTATION_STATUS.md`](../../Manifesto/IMPLEMENTATION_STATUS.md), [`Manifesto/http/src/lib.rs`](../../Manifesto/http/src/lib.rs)
- Wiki : `obsidian/AI FOR ALL/projects/manifesto/concepts/org-owned-visibility-and-participation-limits.md`
- Guides : [`Manifesto/docs/`](../../Manifesto/docs/)
