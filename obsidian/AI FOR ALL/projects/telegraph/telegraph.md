---
title: Telegraph
category: project
tags: [communication, events, notifications, visibility/internal]
sources:
  - README.md
  - Telegraph/openspecs.yaml
  - Telegraph/setup/src/app.rs
  - Telegraph/infra/src/event/consumer.rs
  - Telegraph/http/src/lib.rs
summary: Telegraph is a Rust communication service that consumes IAM events from queues and exposes a JWT-protected notification API using rustycog primitives.
provenance:
  extracted: 0.73
  inferred: 0.17
  ambiguous: 0.10
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-04-14T18:18:24.0602572Z
---

# Telegraph

`Telegraph` is the communication service in the AIForAll workspace. It combines a queue-driven event consumer with a JWT-protected notification API, using shared `rustycog` components for configuration, commands, HTTP routing, permissions, database access, and queue transport.

## Key Ideas

- The service is split across domain, application, infrastructure, HTTP, configuration, setup, and migration crates, with `setup/src/app.rs` acting as the composition root for email, template, repository, command, and queue-consumer wiring.
- Telegraph runs two entry paths in parallel: an SQS-backed consumer that reacts to IAM events and an authenticated HTTP API that serves stored notification state.
- Event payloads are transformed into user-facing output through `[[concepts/descriptor-driven-communications]]`, where per-event TOML descriptors and Tera templates determine whether an event yields email, in-app notification content, or both.
- Queue events flow through `[[concepts/queue-driven-command-processing]]`, so async consumers and HTTP handlers both delegate into the same `rustycog` command runtime instead of bypassing it.
- Runtime behavior depends on `[[concepts/structured-service-configuration]]`, especially the split between transport-level `queue` settings and Telegraph-specific `queues.*` event-routing and `communication.*` delivery settings.
- The root repo overview and some Telegraph config/model surfaces describe SMS alongside email and notifications, but the currently wired processor composite only registers email and notification handlers. Conflict to resolve. ^[ambiguous]

## Related

- [[references/telegraph-service]] - Code-backed overview of Telegraph's crate layout, shared dependencies, and parallel runtime shape.
- [[references/telegraph-runtime-and-configuration]] - `TELEGRAPH_*` config loading, queue routing, template paths, and local runtime drift.
- [[references/telegraph-http-and-notification-api]] - The live notification route table, ownership checks, and OpenAPI drift.
- [[references/telegraph-event-processing]] - SQS consumption, command dispatch, descriptor loading, and delivery-mode routing.
- [[references/telegraph-testing-and-smtp-fixtures]] - Real SQS, SMTP, DB, and JWT-backed integration tests.
- [[skills/building-event-driven-notification-services]] - Reusable workflow for building Telegraph-style communication services.

## Open Questions

- The root `README.md` says Telegraph runs on port `8081` in the shared stack, while `Telegraph/docker-compose.yml` exposes `8080:8080`. Conflict to resolve. ^[ambiguous]
- The repo overview says IAMRusty publishes to `user-events`, while Telegraph's own queue-routing examples are keyed under `test-user-events`; the naming split needs a single operator-facing story. Conflict to resolve. ^[ambiguous]
- `http/src/handlers/communication.rs` defines richer send-message DTOs, but the live route table only exposes notification read-model endpoints. ^[ambiguous]

## Sources

- [[references/telegraph-service]]
- [[references/telegraph-runtime-and-configuration]]
- [[references/telegraph-http-and-notification-api]]
- [[references/telegraph-event-processing]]
- [[references/telegraph-testing-and-smtp-fixtures]]
