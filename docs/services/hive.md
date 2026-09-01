# Hive

Organisations, membres, invitations, liens externes, sync jobs.

- Préfixe : `/hive` — compose : **8082**
- JWT : `[auth.jwt]` consommateur (HS256, `iss=iamrusty`, `aud=aiforall`)
- OpenFGA : type `organization` ; nested routes → `with_permission_on_param`
- Events : cycle de vie org/membres → `sentinel-sync-events`

## Docs

- [../functional/organisation.md](../functional/organisation.md)
- [../guides/permissions.md](../guides/permissions.md)
- Wiki : `projects/hive/hive`
