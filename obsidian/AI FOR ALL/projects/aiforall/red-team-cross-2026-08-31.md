# Synthèse Red Team — IAM / Manifesto / Hive — 2026-08-31

Audits défensifs (classes d’attaque + hardening, aucun exploit). Sources :

- [[projects/iamrusty/red-team-iam-2026-08-31]]
- [[projects/manifesto/red-team-manifesto-2026-08-31]]
- [[projects/hive/red-team-hive-2026-08-31]]

## Verdict

**Les trois services sont non conformes.** Tes soupçons sont fondés. Ce n’est pas « crypto cassée » : Argon2, `alg` JWT figé, OpenFGA fail-closed, SeaORM tiennent. Le modèle **identité + AuthZ + contrat rustycog** est trop permissif, et **les tests figent les trous**.

| Service | Verdict | Critique | Haute | Cœur du risque |
|---|---|---|---|---|
| IAMRusty | Identité trop permissive | 3 | 8 | Account takeover (signup / OAuth state / fusion email) |
| Manifesto | Non prêt prod | 2 | 8 | Guard FGA sur le mauvais objet + rattachement org |
| Hive | Non conforme, risque élevé | 4 | 9 | Même guard FGA + Write→Admin + lecture anonyme |

## Motifs transverses (une cause, plusieurs services)

1. **Dernier UUID du path** — `rustycog-http/src/middleware_permission.rs` : le `Check` OpenFGA porte sur le UUID le plus profond, typé comme la ressource métier. Hive mute `organization_id` mais checke `user_id`/`role_id`. Manifesto mute le projet mais checke `user_id`/`component_id`. **Un correctif rustycog, deux services.**
2. **Tests qui sanctifient le défaut** — `Hive/tests/members_api_tests.rs`, `Manifesto/tests/member_api_tests.rs`, tests OAuth IAM. La CI valide le contrat dangereux.
3. **JWT sans `iss`/`aud`** — issuer IAM + consumers Hive/Manifesto (HS256, `exp` seul). Commentaire Hive : pas de JWKS, secret plat ; IAM parle RS256 côté issuer vs HS256 côté rustycog. Replay / confusion inter-services si le secret est partagé.
4. **Secrets dans les TOML trackés** — HMAC, OAuth, `postgres:postgres`, secret démo rustycog. Rotation + sortie de Git.
5. **sentinel-sync = confused deputy** — Hive : rôle `Admin` local → tuple FGA `admin`. Manifesto : `owner_id` org cru → héritage admin/viewer ; delete/replace permissions / visibilité = no-op FGA.
6. **AuthZ HTTP-only** — use cases Hive `TODO: Add permission check` ; Manifesto `get`/`remove_member`/`revoke` ignorent le requester. File / appel interne = pas de garde.

## Ordre de correction (pas d’exploit)

**P0 — même semaine**

1. rustycog : résoudre l’objet FGA depuis le **paramètre de ressource** (pas le dernier UUID). Casser puis réécrire les tests qui seedent `organization:{user_id}` / `project:{user_id}`.
2. IAM : signup ne doit jamais attacher un mot de passe à un compte existant ; `state` OAuth signé + TTL + bind `user_id` ; fusion email seulement si l’IdP affirme un email vérifié.
3. Hive : `Write` ne pose plus un rôle `Admin` ; sentinel-sync ne doit pas promouvoir `administer` depuis un événement non authentifié comme « admin ».
4. Manifesto : refuser `organization_id` / `owner_id` sans membership prouvée.

**P1**

- JWT : `iss` + `aud` obligatoires, secret par service ou JWKS RS256 aligné IAM.
- IAM : hasher les refresh, révoquer les sessions au reset MDP, rate limit auth, retirer la route qui exfiltre les tokens IdP.
- Hive : `GET` org authentifié + visibilité par défaut non `Public` ; ne plus logger/renvoyer le jeton d’invitation.
- Manifesto : AuthZ métier sur get/member/revoke ; translator FGA pour delete / replace / visibilité.

**P2** — énumération, politique MDP, cache FGA 15 s, optional-field-update, handlers invitations orphelins.

## Ce qui n’est pas le problème

Pas d’injection OS / IPC / sandbox côté Hive (pas de consumer de commandes live). Manifesto : pas d’upload, SSTI, cookies CSRF. IAM : Argon2, reset hashé, `alg` figé. La parité registry ↔ routes Hive **live** tient.
