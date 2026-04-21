# OpenFGA Authorization

This folder holds the centralized Zanzibar/OpenFGA authorization model that
replaces the per-service Casbin `.conf` files.

## Layout

- [`model.fga`](model.fga) — the unified authorization model in OpenFGA DSL.
- [`docker-compose.openfga.yml`](docker-compose.openfga.yml) — pointer file
  only. OpenFGA now runs as part of the root [`docker-compose.yml`](../docker-compose.yml)
  so it shares the same `postgres` instance and `aiforall-network` as every
  other service.

## Running locally

Bring up the whole stack — OpenFGA included:

```bash
docker compose up -d
```

Or just the OpenFGA dependencies (Postgres + database creation + OpenFGA
migrate + OpenFGA itself):

```bash
docker compose up -d postgres create-databases openfga-migrate openfga
```

Endpoints (mapped on the host):

- HTTP API: `http://localhost:8090`
- gRPC API: `localhost:8091`
- Playground: `http://localhost:3000`

Inside the `aiforall-network` (other services in the compose file) use the
service name and the container port: `http://openfga:8080`.

## Loading the authorization model

Once OpenFGA is up, create a store and upload the model via the Playground at
`http://localhost:3000` or the `fga` CLI:

```bash
fga store create --name aiforall --api-url http://localhost:8090
fga model write --store-id <STORE_ID> --file openfga/model.fga --api-url http://localhost:8090
```

Then expose the resulting `store_id` and `authorization_model_id` to each
service through `OPENFGA__STORE_ID` and `OPENFGA__AUTHORIZATION_MODEL_ID` env
vars (read by `OpenFgaPermissionChecker` in `rustycog-permission`). For
in-network service-to-service calls set `OPENFGA__API_URL=http://openfga:8080`;
for tests running on the host use `http://localhost:8090`.

## Mapping from the deleted Casbin `.conf` files

The model collapses every legacy `.conf` into a single relation graph:

| Former file                                         | Former shape                              | OpenFGA type / relation                                      |
|-----------------------------------------------------|-------------------------------------------|--------------------------------------------------------------|
| `Hive/resources/permissions/organization.conf`      | `(sub, obj, act)` matcher on org id       | `organization` with `owner`, `admin`, `member`, `viewer`     |
| `Hive/resources/permissions/member.conf`            | `(sub, org, member, act)` flat matcher    | `organization#member` tuple (no separate member type)        |
| `Hive/resources/permissions/external_link.conf`     | `(sub, obj, act)` matcher                 | `organization#admin` check (links are org-scoped)            |
| `Manifesto/resources/permissions/project.conf`      | `(sub, project, act)` matcher             | `project` with org-inherited `admin`/`viewer`                |
| `Manifesto/resources/permissions/member.conf`       | `(sub, project, member, act)` matcher     | `project#member` tuple                                       |
| `Manifesto/resources/permissions/component.conf`    | Two-level `(sub, project, component, act)` | `component#project@project:{id}` hierarchy                   |
| `Telegraph/resources/permissions/notification.conf` | `(sub, obj, act)` matcher                 | `notification#recipient` tuple                               |
| `IAMRusty/resources/permissions/provider.conf`      | Unused flat matcher                       | Handled in IAM directly; no OpenFGA relation                 |
| `IAMRusty/resources/permissions/user.conf`          | Unused flat matcher                       | Handled in IAM directly; no OpenFGA relation                 |

## Verb mapping

The legacy `Permission` enum (`Read`, `Write`, `Admin`, `Owner`) is preserved
on the Rust side. The `OpenFgaPermissionChecker` translates each variant to a
relation name at call time:

| `Permission` | Relation invoked                                     |
|--------------|------------------------------------------------------|
| `Read`       | `read`                                               |
| `Write`      | `write`                                              |
| `Admin`      | `administer`                                         |
| `Owner`      | `own`                                                |

Every object type in [`model.fga`](model.fga) exposes all four verbs as derived
relations so checks can be uniform across services.
