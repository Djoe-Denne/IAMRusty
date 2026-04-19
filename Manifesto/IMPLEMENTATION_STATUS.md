# Manifesto Service - Implementation Status

**Last Updated:** April 19, 2026  
**Overall Status:** Production-ready baseline after remediation

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
- Public project/component reads can succeed anonymously.
- Non-public reads require real membership/permission checks.
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

- Manifesto publishes Manifesto domain events from project/component/member use cases on a best-effort basis.
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
- Manifesto domain-event publication is still best-effort rather than transaction-critical.
- The public HTTP surface does not yet expose a richer provisioning handoff for components.

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
- `tests/component_acl_consistency_tests.rs`
- `tests/component_service_client_tests.rs`
- `tests/event_runtime_tests.rs`
- `../rustycog/rustycog-http/tests/permission_middleware_tests.rs`

---

## Remaining Follow-Up Work

Useful future enhancements, but not blockers for the current runtime:

1. Expose component endpoint/access-token handoff through the API once provisioning design is finalized.
2. Add queue-backed end-to-end tests when dedicated broker fixtures are part of the default CI posture.
3. Decide whether any event-publication failures should become hard-fail instead of best-effort warnings.
