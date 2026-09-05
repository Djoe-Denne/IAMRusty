# Manifesto Service - Implementation Status

**Last Updated:** September 5, 2026
**Overall Status:** Production-ready baseline; L-PARTNERSHIP remains open.

This file is the current source of truth for Manifesto's live runtime behavior.

---

## Executive Summary

Manifesto now matches the security, permission, runtime, and event behavior that the surrounding docs describe.

Most important outcomes from the remediation pass:

- verified HS256 bearer-token handling
- correct optional-auth behavior for anonymous public reads
- strict ACLs for non-public project/component reads
- component lifecycle and component-instance ACL sync now fail together
- real config wiring for logging, retry, quotas, pagination, and component-service timeout/api key
- fail-closed component catalog integration
- structured HTTP error mapping
- apparatus consumer and processor wired into startup when queues are enabled
- focused tests covering signed auth rejection, public-read permission logic, component/ACL consistency, fail-closed component client behavior, and apparatus runtime semantics

---

## What Is Implemented

### Auth and Permissions

- Bearer auth uses shared `AuthConfig` with `auth.jwt.hs256_secret`.
- `rustycog-http` verifies JWT signatures instead of trusting payload-only parsing, and the shared verifier path is HS256-only today.
- Optional-auth project/component resource routes evaluate anonymous callers through the shared permission path.
- `GET /api/projects` uses optional auth plus service-layer visibility filtering rather than UUID-scoped permission middleware.
- Public project GET/details and component list/get succeed anonymously when `viewer@user:*` exists **and** the row is still world-readable (`visibility=public`, status `draft|active`). A leftover wildcard after private/internal/archive is ignored on those surfaces.
- Anonymous `GET /api/projects` lists only public **live** rows (`draft|active`). Authenticated list also includes `project_members`, Internal/public org-owned rows for org viewers, and org-owned rows for org admins. Non-public GET-by-id succeeds for owners, project members, Internal org viewers, and org admins on private.
- `POST /api/projects/{id}/join` is JWT-only self-join on public live projects (`write` / `project`, `MemberSource::Invitation`, 409 if already a member).
- A visibility flip that involves `public` requires project `Admin`. `private↔internal` stays `Write`.
- Specific component-instance grants preserve resource type semantics.

### Project, Component, and Member Flows

- Project CRUD, publish, archive, list, and detail flows are implemented.
- Component add/get/list/update/remove flows are implemented.
- Component add/remove aborts if the matching component-instance ACL resource cannot be synchronized, with compensation to avoid silent drift.
- Member add/get/list/update/remove plus permission grant/revoke flows are implemented.
- Project creation bootstraps owner membership and owner permissions immediately.
- Member removal uses the configured grace period instead of hard delete.

### Runtime and Configuration

- `main.rs` uses setup/config wiring rather than bypassing it.
- `logging.level` is consumed through setup logging.
- `[command.retry]` is threaded into the command registry factory.
- `service.component_service.base_url`, `api_key`, and `timeout_seconds` are used by `ComponentServiceClient`.
- `service.business.*` limits are enforced in live application flows.
- Checked-in `default`, `development`, and `test` configs explicitly set:

```toml
[queue]
type = "disabled"
```

This keeps local/test boots stable unless queue behavior is explicitly enabled.

### Events

- Create, visibility change, archive, and member add/join record AuthZ events in the same DB transaction as the aggregate (outbox). `ProjectUpdated` / `ProjectPublished` still publish after persist (no-op FGA).
- `ApparatusEventConsumer` is created in `setup/src/app.rs` and started alongside the HTTP server when queue config resolves to a real consumer.
- `ComponentStatusProcessor` now performs real component-state reconciliation:
  - duplicate target-state events are a no-op
  - stale events are ignored instead of rewinding state
  - applied timestamps use the incoming event's `changed_at`
- The old unused outbound apparatus adapter is gone; current outbound runtime vocabulary is Manifesto domain events.

### Testing

Focused coverage now exists for:

- signed-token acceptance plus tampered-token rejection
- anonymous public-read versus denied private-read permission behavior
- visibility flip emits `ProjectVisibilityChanged`; `publish` does not write `user:*`
- HTTP + real OpenFGA: create/publish/PUT never write `viewer@user:*` in-process; leftover wildcard after private/archive does not keep GET/details/components open; Write-only cannot flip to public; list SQL can show a live public row before GET is allowed, but hides `public`+`archived`
- SQS routing: `project_visibility_changed` / `project_published` / `project_archived` go to `test-sentinel-sync-events`; `project_updated` stays on the default queue. No in-process sentinel-sync consumer in Manifesto tests (repo pattern: translator unit tests + HTTP `allow_wildcard`).
- forwarding of project-list visibility/search filters through service wiring
- fail-hard component-instance ACL synchronization on add/remove flows
- fail-closed component-service HTTP behavior
- apparatus consumer bootstrap in disabled mode plus safe no-op fallback for enabled queue config without a broker
- apparatus component-status processor duplicate-delivery no-op behavior, stale-event handling, and state updates

---

## Known Limits

These are current product/runtime limits, not hidden implementation gaps:

- `ComponentResponse.endpoint` and `ComponentResponse.access_token` are still `None`.
- Queue-backed end-to-end behavior is not enabled by default in checked-in local/test configs.
- No Hive org-to-org partnership (L-PARTNERSHIP). Partnership is one future feeder into `POST .../join`, not the only public-participation story.
- `external_collaboration_enabled` is stored and never read.
- Anonymous list can show a public live SQL row before `user:*` exists; GET `{id}` still needs the wildcard.
- Historical leftover `viewer@user:*` on non-public rows: use the ops sweep below (no Manifesto cron).

---

## Key Files

Runtime composition:

- `setup/src/app.rs`
- `setup/src/config.rs`
- `src/main.rs`

Security and permission flow:

- `http/src/lib.rs`
- `http/src/handlers/projects.rs`
- `http/src/error.rs`
- `domain/src/service/permission_fetcher_service.rs`
- `application/src/error.rs`
- `../rustycog/rustycog-http/src/jwt_handler.rs`

Runtime integrations:

- `infra/src/adapters/component_service_client.rs`
- `infra/src/event/consumer.rs`
- `infra/src/event/processors/component_processor.rs`

Configuration:

- `configuration/src/lib.rs`
- `config/default.toml`
- `config/development.toml`
- `config/test.toml`

Focused tests:

- `tests/public_acl_api_tests.rs`
- `tests/project_visibility_event_tests.rs`
- `tests/project_visibility_acl_api_tests.rs`
- `tests/project_join_api_tests.rs`
- `tests/wildcard_reconcile_tests.rs`
- `tests/sqs_event_routing_tests.rs`
- `tests/transaction_readiness_tests.rs`
- `tests/component_acl_consistency_tests.rs`
- `tests/component_service_client_tests.rs`
- `tests/event_runtime_tests.rs`
- `../rustycog/rustycog-http/tests/permission_middleware_tests.rs`

---

## Remaining Follow-Up Work

Useful future enhancements, but not blockers for the current runtime:

1. Expose component endpoint/access-token handoff through the API once provisioning design is finalized.
2. Add queue-backed end-to-end tests when dedicated broker fixtures are part of the default CI posture.
3. L-PARTNERSHIP: org↔org feeder into `POST .../join` if/when Hive has a partnership graph.
4. Run the wildcard sweep after the AuthZ outbox is in production (see below). Do not add a Manifesto cron reconciler in this pass.

## Wildcard sweep runbook

Manifesto never writes OpenFGA. After the AuthZ outbox is live in production:

1. SQL on Manifesto: select `id` where `visibility != 'public'` OR `status IN ('archived', 'suspended')`.
2. For each id, call sentinel-sync `OpenFgaWriteClient::reconcile_wildcards` with `(id, false)` (idempotent delete of `project:{id}#viewer@user:*`).
3. Optionally pass `(id, true)` for current public live rows if you need to backfill missing wildcards.
4. Do not schedule a periodic Manifesto↔FGA reconciler until this one-shot sweep has run.
