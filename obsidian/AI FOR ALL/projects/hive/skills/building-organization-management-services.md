---
title: Building Organization Management Services
category: skills
tags: [organizations, permissions, services, visibility/internal]
sources:
  - Hive/setup/src/app.rs
  - Hive/application/src/command/factory.rs
  - Hive/domain/src/service/permission_service.rs
  - Hive/application/src/usecase/organization.rs
  - Hive/application/src/usecase/invitation.rs
  - Hive/application/src/usecase/external_link.rs
  - Hive/tests/common.rs
summary: Build a Hive-style service by combining typed org models, resource-backed permissions, event-publishing use cases, and real API fixtures.
provenance:
  extracted: 0.76
  inferred: 0.18
  ambiguous: 0.06
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-19T11:13:11Z
---

# Building Organization Management Services

Use this page when building a service that manages organizations, members, invitations, external integrations, and permission-scoped HTTP resources in the style of `[[projects/hive/hive]]`.

## Workflow

- Start with the domain model: define organizations, members, roles, resources, permissions, invitations, and external links before wiring HTTP.
- Keep route authorization resource-scoped by pairing `RouteBuilder` resources with repository-backed `PermissionsFetcher` implementations instead of hardcoding role checks in handlers.
- Register commands for the full product workflow, then decide deliberately which ones should be exposed over HTTP and which should stay internal or queue-triggered.
- Reuse `[[skills/building-rustycog-services]]` when you need the shared RustyCog composition order (config -> logging -> DB -> registry -> routes) behind this Hive-specific domain workflow.
- Publish domain events from use cases after successful state changes so downstream systems can react without coupling themselves to Hive's HTTP API.
- Treat external-provider integration as a first-class port with explicit config, validation, connection tests, and fixtures rather than as a loose helper client.
- Test the live API with real DB state, JWTs, and mock external-provider services before adding queue-backed verification.

## Open Questions

- Keep the API contract, handlers, and route registration aligned as the service evolves; Hive shows how easy it is for those three layers to drift apart. ^[ambiguous]

## Sources

- [[projects/hive/hive]] - Service where this pattern is implemented concretely.
- [[projects/hive/concepts/organization-resource-authorization]] - Authorization model to preserve when building the API.
- [[projects/hive/concepts/invitation-driven-membership]] - Invitation-first onboarding pattern.
- [[projects/hive/concepts/external-provider-sync-jobs]] - External integration and sync-job pattern.
- [[skills/building-rustycog-services]] - Shared service-construction playbook for the RustyCog runtime layer.