# Autorisation OpenFGA

L'identité vérifiée dit *qui* par le couple exact **(iss,sub)** ; sub seul n'est pas une autorité. OpenFGA dit *s’il a le droit* sur *cet objet*. Un seul `Arc<dyn PermissionChecker>` par process, câblé dans `AppState` (souvent `OpenFgaPermissionChecker` + `CachedPermissionChecker` + `MetricsPermissionChecker`).

Le type FGA reste `user`. L'autorité plateforme configurée conserve `user:UUID`
pour les droits dont la provenance plateforme est établie ; une autre autorité
utilise `user:p1.BASE64URL_UTF8_ISSUER.UUID` canonique selon0314. Sujet et clé de
cache conservent l'issuer exact : pas fallback UUID ni alias dérivé de Host/kid/org
ou d'un suffixe. Compatibilité `iamrusty` seulement explicitement attestée/configurée,
pas équivalence générale. Les identités organisation restent autorisables dans
Hive avec leurs propres grants ; les frontières HumanAccount/UUID-only refusent
les principals incompatibles avant effets, sans restriction globale du vérificateur.

## Types (`ops/openfga/model.fga`)

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

- `.authenticated()` — Bearer et principal vérifié obligatoires ; `AuthPrincipal` pour une interface principal-aware, `AuthUser` uniquement pour une vue compte plateforme validée. Aucune conversion silencieuse d'un principal org en UUID plateforme.
- `.might_be_authenticated()` — anonyme possible (`OptionalAuthUser`). Le middleware optional résout l’anonyme en `Subject::wildcard()` → `user:*` sur le fil (lecture publique projet). Absence réelle de principal seulement : un principal authentifié incompatible/refusé ne devient pas wildcard.
- `.with_permission_on(Permission, object_type)` — objet = **UUID le plus profond** du path (segments non-UUID ignorés, ex. `{component_type}`).
- `.with_permission_on_param(Permission, object_type, "organization_id")` — **obligatoire** dès que le dernier UUID n’est pas l’objet FGA (membres, rôles, invitations, permissions imbriquées).

Sans `*_param`, un `PATCH .../members/{user_id}` checkerait `organization:{user_id}` (trou red-team P0, fermé le 2026-08-31).

## Cache

`OpenFgaClientConfig.cache_ttl_seconds = Some(0)` : le composition root **doit** sauter `CachedPermissionChecker`. Sinon grant→revoke en test sert un allow périmé.

La clé distingue autorité exacte et genre de sujet, pas sub seul ; TTL0 conserve
le bypass existant.

## Tests

Hive / Telegraph / Manifesto : `TestOpenFga` (vrai conteneur). Défaut = deny. Chaque test appelle `openfga.allow(subject, action, resource)`. IAM : `has_openfga() == false`.

Voir [../guides/permissions.md](../guides/permissions.md) et [../guides/tests-integration.md](../guides/tests-integration.md).

## Worker

Les tuples ne sont **pas** écrits par les handlers HTTP. `sentinel-sync` traduit les événements domaine ([events-outbox.md](events-outbox.md)). Oublier un bras de translator = AuthZ silencieuse en retard.

Les projections Hive conservent le principal propriétaire/membre persisté visé,
distinct de l'acteur ; source ambiguë : refus/quarantaine, jamais grant UUID deviné.
La [campagne2026-10-29](../contracts/20261029-auth-security-qualified-state.md)
qualifie les frontières ciblées et le navigateur, pas toutes les projections
runtime ou une réconciliation live complète. Modèles/relations inchangés ici.
