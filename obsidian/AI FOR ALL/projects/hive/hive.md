---
title: Hive
category: project
tags: [organizations, permissions, integrations, visibility/internal]
sources:
  - Hive/Cargo.toml
  - Hive/openspecs.yaml
  - Hive/setup/src/app.rs
  - Hive/http/src/lib.rs
  - Hive/application/src/command/factory.rs
summary: Hive is a Rust organization-management service for organizations, members, invitations, external links, and sync jobs built on rustycog and hive-events.
provenance:
  extracted: 0.74
  inferred: 0.16
  ambiguous: 0.10
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-14T18:56:22.3888182Z
---

# Hive

`Hive` is the organization-management service in the AIForAll workspace. It owns organizations, members, invitations, role-based permissions, external provider links, and sync jobs, and it publishes `[[projects/hive-events/hive-events]]` events for downstream consumers such as `[[projects/telegraph/telegraph]]`.

## Key Ideas

- The service is split across domain, application, infrastructure, HTTP, configuration, setup, and migration crates, with `setup/src/app.rs` acting as the composition root for DB access, permission fetchers, event publishing, use cases, and the command registry.
- Hive's live HTTP surface is narrower than its declared OpenAPI contract: the route builder exposes core organization, member, invitation creation, external-link creation, and sync-job start flows, while the spec and handlers describe a wider API. Conflict to resolve. ^[ambiguous]
- Command orchestration is centralized through a RustyCog registry, but Hive's registry breadth is larger than its live route table, so some operations exist as commands and handlers without being wired into the server.
- Runtime behavior depends on `[[concepts/structured-service-configuration]]`, especially the `HIVE` env prefix, command retry settings, queue transport, and the outbound `iam_service` and `external_provider_service` sections.
- Hive publishes `[[projects/hive-events/hive-events]]` domain events for organization, member, invitation, external-link, and sync-job changes rather than treating HTTP as the only integration surface.
- Hive uses the shared `rustycog` stack, but it diverges from IAMRusty and Telegraph in its custom HTTP error model and in how much of its command or spec surface is actually exposed over HTTP. Conflict to resolve. ^[ambiguous]

## Related

- [[references/hive-service]] - Code-backed overview of Hive's crate layout, runtime wiring, and shared dependencies.
- [[references/hive-runtime-and-configuration]] - `HIVE_*` config loading, queue publishing, retry settings, and service-to-service config.
- [[references/hive-http-api-and-openapi-drift]] - The live route table, custom error surface, and the gaps between shipped HTTP and `openspecs.yaml`.
- [[references/hive-command-execution]] - Registry coverage, command names, and event-publishing use cases.
- [[references/hive-data-model-and-schema]] - Organizations, members, invitations, external links, sync jobs, and permission tables.
- [[references/hive-testing-and-api-fixtures]] - Real DB, JWT, and external-provider fixture patterns in the Hive tests.
- [[skills/building-organization-management-services]] - Reusable workflow for building Hive-style org-management services.

## Open Questions

- The service has no Hive-local README in this tree, so Cargo metadata, OpenAPI, config, and code are the main documentation sources.
- The OpenAPI contract, registered routes, and command registry do not currently describe the same breadth of operations. Conflict to resolve. ^[ambiguous]

## Sources

- [[references/hive-service]]
- [[references/hive-runtime-and-configuration]]
- [[references/hive-http-api-and-openapi-drift]]
- [[references/hive-command-execution]]
- [[references/hive-data-model-and-schema]]
- [[references/hive-testing-and-api-fixtures]]