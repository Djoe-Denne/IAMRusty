---
title: Component Catalog and Fail-Closed Adapter
category: concepts
tags: [components, integrations, projects, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/README.md
  - Manifesto/infra/src/adapters/component_service_client.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/configuration/src/lib.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/tests/component_service_client_tests.rs
summary: "Client catalogue HTTP actuel, fail-closed ; évolution future vers catalogue de releases immuables, avec résolution runtime séparée."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
created: 2026-04-14T20:25:00Z
updated: 2026-09-09T17:50:00Z
---

# Component Catalog and Fail-Closed Adapter

`[[projects/manifesto/manifesto]]` treats components as external capabilities rather than local feature modules. The runtime expresses that through `ComponentServiceClient`, which validates types through an HTTP component catalog and now fails closed when that dependency is unavailable or unhealthy.

## Key Ideas

- `ComponentUseCaseImpl::add_component()` refuses to attach a component until `validate_component_type()` succeeds and the project does not already contain the same component type.
- `ComponentServiceClient` calls `GET {base_url}/api/components` and parses `ComponentInfo` values when the external service responds successfully.
- `service.component_service.base_url`, `api_key`, and `timeout_seconds` are all consumed by the live runtime.
- When the HTTP call fails or returns a non-success status, the adapter returns `DomainError::ExternalServiceError`; it does not fall back to built-in mock component data.
- `tests/component_service_client_tests.rs` covers both the fail-closed error path and bearer API-key forwarding.
- `ComponentResponse.endpoint` and `access_token` still remain unset, which means type validation exists today but full provisioning handoff does not.

## Catalogue Apparatus futur

Le `ComponentInfo` actuel contient version et endpoint, mais ces valeurs ne sont pas figées dans `ProjectComponent`. Le client HTTP ne constitue donc pas le catalogue de releases admises ou le Registry OCI du futur dossier [[projects/manifesto/references/apparatus-factory-and-distribution]].

Proposition : le catalogue décrit ce qui est installable ; le Registry stocke les artifacts ; le resolver runtime choisit l’instance du binding. Conserver le comportement fail-closed, figer le digest à l’installation et laisser `endpoint`/`access_token` legacy non renseignés. L’invoke du gateway remplace la remise de credentials au client. ^[inferred]

## Open Questions

- Proposition désormais détaillée : séparer catalogue, resolver runtime et gateway ; choisir l’implémentation selon [[projects/manifesto/references/apparatus-implementation-plan]]. ^[inferred]

## Sources

- [[projects/manifesto/manifesto]] - Service overview for Manifesto's component orchestration role.
- [[projects/manifesto/concepts/component-instance-permissions]] - Permission and resource model that accompanies component attachment.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - API flows that expose add/list/remove component behavior.
- [[projects/manifesto/references/manifesto-event-model]] - Event behavior that accompanies component status changes.
