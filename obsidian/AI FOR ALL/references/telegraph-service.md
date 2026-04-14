---
title: Telegraph Service
category: references
tags: [reference, communication, architecture, visibility/internal]
sources:
  - README.md
  - Telegraph/setup/src/app.rs
  - Telegraph/infra/src/event/consumer.rs
  - Telegraph/http/src/lib.rs
  - Telegraph/configuration/src/lib.rs
summary: Code-backed overview of Telegraph's crate layout, parallel runtime, rustycog integrations, and notification-focused service boundaries.
provenance:
  extracted: 0.81
  inferred: 0.12
  ambiguous: 0.07
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-04-14T18:18:24.0602572Z
---

# Telegraph Service

These sources define the overall shape of `[[projects/telegraph/telegraph]]`: how the crates are split, how the runtime is assembled, and how the service exposes both async event processing and synchronous notification endpoints.

## Key Ideas

- Telegraph is split across domain, application, infrastructure, HTTP, configuration, setup, migration, and a root binary crate, matching the same broad service layout used elsewhere in the repo.
- `setup/src/app.rs` is the composition root: it creates the email adapter and service, template service, database pool, repositories, notification service, permission fetcher, communication factory, event processor, command registry, command service, and final event consumer.
- `TelegraphApp::run()` starts the event consumer and the HTTP server in parallel and waits on both with `tokio::select!`, so queue processing is not an auxiliary background thread but a first-class runtime path.
- The service leans heavily on shared `rustycog` crates for commands, config loading, DB access, HTTP route building, permission checks, queue consumers, and test fixtures.
- HTTP exposure is intentionally narrow: the live server only wires notification read-model routes, while richer communication DTOs remain present in code but not in the active route table. ^[ambiguous]
- The root repo README describes Telegraph as the platform communication service for emails, notifications, and SMS, but the service-level code reveals more implementation detail than the top-level overview does. ^[ambiguous]

## Open Questions

- Telegraph has no project-local README in this source tree, so the top-level repo README and the code are doing most of the documentation work today.
- The queue consumer is arguably the service's primary ingress path, but the current codebase does not declare one canonical “main interface” between HTTP and queue processing. ^[inferred]

## Sources

- [[projects/telegraph/telegraph]] - Main project overview.
- [[concepts/queue-driven-command-processing]] - How async events are pushed through the command runtime.
- [[references/telegraph-runtime-and-configuration]] - Runtime config, queue routing, templates, and local deployment details.
- [[references/telegraph-http-and-notification-api]] - Authenticated route surface and ownership model.
