# Gardes de permission HTTP

Complète [../platform/authz-openfga.md](../platform/authz-openfga.md). Skill : [using-rustycog-permission.md](../../.agents/skills/rustycog/references/using-rustycog-permission.md).

## Règle d’extraction d’objet

Par défaut, le middleware prend le **dernier segment UUID** du path et le type passé à `with_permission_on`.

| Path | UUID retenu | Type à passer | Appel |
|---|---|---|---|
| `/api/organizations/{organization_id}` | org | `organization` | `with_permission_on(Read, "organization")` |
| `/api/organizations/{organization_id}/members/{user_id}` | **user_id** | trop profond | `with_permission_on_param(Write, "organization", "organization_id")` |
| `/api/projects/{project_id}/components/{component_type}` | project (type non-UUID sauté) | `project` | `with_permission_on` suffit souvent |
| `/api/projects/{project_id}/members/{user_id}` | **user_id** | | `with_permission_on_param(..., "project_id")` |
| `/api/notifications/{id}/read` | notification | `notification` | `with_permission_on(Write, "notification")` |

Hive et Manifesto ont déjà les `*_param` sur membres / rôles / invitations / grants. Un nouveau nested UUID = re-lire cette table.

## Ordre builder

```rust
.delete("/api/organizations/{id}/members/{user_id}", remove_member)
.authenticated()
.with_permission_on_param(Permission::Write, "organization", "organization_id")
```

Inverser auth et permission, ou poser `with_permission_on` trop tôt, casse le mode anonyme vs obligatoire.

## Permissions rustycog

`Permission::Read | Write | Admin | Owner` se mappent sur les relations `read` / `write` / `administer` / `own` du modèle. Le modèle dérive ensuite `viewer`/`member`/`admin`/`owner`.

## Anonyme

`.might_be_authenticated()` + lecture : le wildcard `user:*` ne passe que si sentinel-sync a écrit un tuple public (`project:{id}#viewer@user:*` pour `visibility = public`). Hive : GET org est **authentifié** + `Read` ; la recherche reste publique (orgs visibles selon le use case).

## Tests deny

Ne rien `allow` → 403. Pour un 404 handler, **accorder** d’abord le Check (sinon le garde coupe avant le use case). `cache_ttl_seconds = 0` + pas de wrapper cache.
