---
name: aiforall-new-service
description: >-
  Platform checklist for adding a new AIForAll RustyCog service (not just the
  Manifesto crate slice): events crate, OpenFGA model, sentinel-sync translator,
  docker-compose, oodhive-monolith nest, SERVICE_PREFIX, [auth.jwt]. Use when
  scaffolding a new bounded context, registering a service in the monolith or
  compose file, adding hive-events-style contracts, or wiring a new OpenFGA type.
---

# New AIForAll service (platform)

Crate shape lives in `.agents/skills/rustycog/references/building-rustycog-services.md`.
This skill is the **workspace wiring**. Human how-to: `docs/guides/nouveau-service.md`.

## When to use

- User asks to add a service beside Hive / Manifesto / Telegraph / IAM.
- Touching `monolith/src/routes.rs`, root `docker-compose.yml`, `openfga/model.fga`, or `sentinel-sync/src/translator/`.
- Adding a `*-events` crate or a new `SERVICE_PREFIX`.

## Checklist (in order)

1. Vertical slice (domain → http → setup) like Manifesto. `SERVICE_PREFIX`, `create_router`, `create_prefixed_router`, `start_background_tasks` / `stop_background_tasks`.
2. `[auth.jwt]` with `hs256_secret`, `issuer = "iamrusty"`, `audience = "aiforall"`. `UserIdExtractor::new`. See `docs/guides/jwt-consommateur.md`.
3. New AuthZ type? Edit `openfga/model.fga` + regenerate `openfga/model.json`. Routes: `.authenticated()` then `with_permission_on` / `with_permission_on_param`.
4. `foo-events` workspace member. If the event changes FGA: translator module + register in `sentinel-sync/src/main.rs`. Unknown events are silent no-ops.
5. `[queue.queues]` → physical queues (`sentinel-sync-events` or `telegraph-events`). Default `enabled = false`.
6. `MonolithRouters` + `.nest(foo_http::SERVICE_PREFIX, …)` in `monolith/src/routes.rs`. Do not call service `run()`.
7. Root `Cargo.toml` members, `docker-compose.yml` service + `create-databases` (`foo_dev`), host port ≥ 8084.
8. `tests/common.rs` returns a **prefixed** base URL. `create_jwt_token` + `TestOpenFga::allow` as needed.
9. Service README + `docs/services/<name>.md`.

## Forbidden

- Unprefixed standalone routes that diverge from monolith nests.
- RS256 issuer / per-service JWT secret while extractors are HS256 + shared HMAC.
- OpenFGA `object_type` not in `model.fga`.
- Domain event that should move AuthZ with no translator arm.
- Re-declaring `#[path = "fixtures/mod.rs"]` in every `*_test.rs`.

## Related

- `docs/guides/nouveau-service.md`
- `docs/platform/overview.md`
- `.agents/skills/rustycog/SKILL.md`
