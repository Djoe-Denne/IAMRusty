# Autorisation (langage produit) et contrats de sécurité

Modèle technique : [../platform/authz-openfga.md](../platform/authz-openfga.md). Audits défensifs du 2026-08-31 : `obsidian/AI FOR ALL/projects/*/red-team-*-2026-08-31.md`. Les P0/P1 listés ici sont les **contrats à ne pas réouvrir** (commit `5707f9e` et suivants).

## Qui peut quoi

### Organisation (Hive)

| Action | Qui (relation FGA) |
|---|---|
| Chercher des orgs publiques | Anonyme |
| Lire une org | `viewer`+ (membre ou plus) |
| Modifier membres / invitations / sync | `member`+ (`write`) |
| Admin org, liens externes, delete | `admin` / `owner` |

Défaut de visibilité : **Private**. Un `Write` métier ne crée pas un admin FGA.

### Projet (Manifesto)

| Action | Qui |
|---|---|
| Lister / lire un projet public | Anonyme via `user:*` |
| Lire un projet non public | `viewer`+ |
| Éditer le projet | `member`+ (`write`) |
| Publier / archiver / composants / membres | `admin`+ |
| Supprimer le projet | `owner` |

Composants : les routes HTTP checkent le **projet** (param `project_id`), pas `project:{component_id}`.

### Notification (Telegraph)

Seul le `recipient` a `read`/`write`/`administer`/`own`.

### Identité (IAM)

Pas d’OpenFGA. Les routes sensibles sont `.authenticated()` + rate limit. IAM n’écrit pas de tuples org/projet.

## Contrats red-team (fermés)

**P0**

1. Objet FGA = paramètre de ressource (`with_permission_on_param`), pas « dernier UUID du path » dès qu’il y a un UUID imbriqué.
2. Signup IAM : pas d’attachement mot de passe sur un compte existant ; `state` OAuth signé + TTL + bind user ; fusion email si email IdP vérifié.
3. Hive : `Write` ≠ promotion `admin` FGA.
4. Manifesto : pas d’`organization_id` / `owner_id` sans membership.

**P1**

- JWT : `iss` + `aud` requis côté extracteur (`iamrusty` / `aiforall`).
- Refresh hashés ; révocation session au reset MDP ; rate limit auth.
- GET org Hive authentifié ; jeton d’invitation non loggé / non renvoyé.
- AuthZ métier Manifesto sur get/member/revoke ; translator visibilité / delete.

**Toujours vrai (ne pas « corriger »)**

- Argon2, `alg` JWT figé HS256 côté extracteur, OpenFGA fail-closed, SeaORM.
- Tests qui *seedent* `organization:{user_id}` sont des régressions, pas une baseline.

## Confused deputy

`sentinel-sync` fait confiance au **payload d’événement**. Un event `admin` non authentifié ne doit plus promouvoir `administer`. Tout nouveau translator doit mapper des permissions explicites, jamais « le champ owner_id du JSON client ».
