# Autorisation OpenFGA

L’identité dit *qui* (`sub` JWT). OpenFGA dit *s’il a le droit* sur *cet objet*. Un seul `Arc<dyn PermissionChecker>` par process, câblé dans `AppState` (souvent `OpenFgaPermissionChecker` + `CachedPermissionChecker` + `MetricsPermissionChecker`).

## Types (`openfga/model.fga`)

| Type | Relations utiles | Notes |
|---|---|---|
| `user` | — | Sujet |
| `organization` | `owner`, `admin`, `member`, `viewer` ; `read`/`write`/`administer`/`own` | Hive |
| `project` | idem + `organization` ; `viewer` accepte `user:*` | Manifesto, lecture publique |
| `component` | héritage depuis `project` | Checks HTTP Manifesto restent souvent **project-scoped** |
| `notification` | `recipient` | Telegraph ; tuple écrit par sentinel-sync |

Les noms passés à `.with_permission_on(Permission, "organization")` **doivent** exister dans le modèle. Typo → 403 fail-closed + log OpenFGA.

## RouteBuilder

Ordre obligatoire : **mode auth d’abord**, puis le garde.

```rust
.get("/api/organizations/{organization_id}", get_organization)
.authenticated()
.with_permission_on(Permission::Read, "organization")
```

- `.authenticated()` — Bearer obligatoire (`AuthUser`).
- `.might_be_authenticated()` — anonyme possible (`OptionalAuthUser`). Le middleware optional résout l’anonyme en `Subject::wildcard()` → `user:*` sur le fil (lecture publique projet).
- `.with_permission_on(Permission, object_type)` — objet = **UUID le plus profond** du path (segments non-UUID ignorés, ex. `{component_type}`).
- `.with_permission_on_param(Permission, object_type, "organization_id")` — **obligatoire** dès que le dernier UUID n’est pas l’objet FGA (membres, rôles, invitations, permissions imbriquées).

Sans `*_param`, un `PATCH .../members/{user_id}` checkerait `organization:{user_id}` (trou red-team P0, fermé le 2026-08-31).

## Cache

`OpenFgaClientConfig.cache_ttl_seconds = Some(0)` : le composition root **doit** sauter `CachedPermissionChecker`. Sinon grant→revoke en test sert un allow périmé.

## Tests

Hive / Telegraph / Manifesto : `TestOpenFga` (vrai conteneur). Défaut = deny. Chaque test appelle `openfga.allow(subject, action, resource)`. IAM : `has_openfga() == false`.

Voir [../guides/permissions.md](../guides/permissions.md) et [../guides/tests-integration.md](../guides/tests-integration.md).

## Worker

Les tuples ne sont **pas** écrits par les handlers HTTP. `sentinel-sync` traduit les événements domaine ([events-outbox.md](events-outbox.md)). Oublier un bras de translator = AuthZ silencieuse en retard.
