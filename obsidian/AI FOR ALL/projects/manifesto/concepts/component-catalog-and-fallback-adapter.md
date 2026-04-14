---
title: Component Catalog and Fallback Adapter
category: concepts
tags: [components, integrations, projects, visibility/internal]
sources:
  - Manifesto/README.md
  - Manifesto/infra/src/adapters/component_service_client.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/configuration/src/lib.rs
  - Manifesto/setup/src/app.rs
summary: Manifesto validates component types through an external component service, but its current adapter falls back to a mock catalog when the service is unavailable.
provenance:
  extracted: 0.76
  inferred: 0.11
  ambiguous: 0.13
created: 2026-04-14T20:25:00Z
updated: 2026-04-14T20:25:00Z
---

# Component Catalog and Fallback Adapter

`[[projects/manifesto/manifesto]]` treats components as external capabilities rather than local feature modules. The runtime expresses that through `ComponentServiceClient`, which validates types through an HTTP component catalog but still keeps an MVP fallback path when the external service is missing.

## Key Ideas

- `ComponentUseCaseImpl::add_component()` refuses to attach a component until `validate_component_type()` succeeds and the project does not already contain the same component type.
- `ComponentServiceClient` calls `GET {base_url}/api/components` and returns parsed `ComponentInfo` values when the external service responds successfully.
- When the HTTP call fails or returns a non-success status, the adapter falls back to a built-in mock catalog containing component types such as `taskboard`, `custom_forms`, and `analytics`.
- The current adapter lets Manifesto keep working as a project-service MVP even before the broader component-service ecosystem is fully deployed.
- `ManifestoConfig` defines `service.component_service.timeout_seconds`, but setup currently constructs the client with a hardcoded `30` second timeout rather than reading that field. Conflict to resolve. ^[ambiguous]
- `ComponentResponse` still leaves `endpoint` and `access_token` unresolved, which shows the catalog and provisioning edge is only partly surfaced through the current API. ^[ambiguous]

## Open Questions

- When the real component registry is live, should fallback mock components remain available in development only? ^[inferred]
- Should endpoint discovery and component-scoped token issuance come from the same adapter or from separate provisioning flows? ^[ambiguous]

## Sources

- [[projects/manifesto/manifesto]] - Service overview for Manifesto's component orchestration role.
- [[projects/manifesto/concepts/component-instance-permissions]] - Permission and resource model that accompanies component attachment.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - API flows that expose add/list/remove component behavior.
- [[projects/manifesto/references/manifesto-event-model]] - Event emission that accompanies component additions, removals, and status changes.
