---
title: >-
  Hive
category: project
tags: [organizations, permissions, integrations, visibility/internal]
sources:
  - Hive/Cargo.toml
  - Hive/openspecs.yaml
  - hive-events/README.md
  - Hive/setup/src/app.rs
  - Hive/http/src/lib.rs
  - Hive/application/src/command/factory.rs
summary: >-
  Hive : organisations et permissions. T14b : HTTPS compose port 8443,
  CA mesh optionnelle (pas rustls required).
provenance:
  extracted: 0.74
  inferred: 0.16
  ambiguous: 0.10
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-09-20T10:35:00Z
---

# Hive

## Indexes

- [[projects/hive/concepts/index]] — concepts
- [[projects/hive/skills/index]] — skills
- [[projects/hive/references/index]] — references

`Hive` is the organization-management service in the AIForAll workspace. It owns organizations, members, invitations, role-based permissions, external provider links, and sync jobs, and it publishes `[[projects/hive-events/hive-events]]` events for downstream consumers such as `<!-- [[projects/telegraph/telegraph]] -->`.

## Key Ideas

- The service is split across domain, application, infrastructure, HTTP, configuration, setup, and migration crates, with `setup/src/app.rs` acting as the composition root for DB access, permission fetchers, event publishing, use cases, and the command registry.
- Hive's live HTTP surface is still narrower than its declared OpenAPI contract (invitations / `update_member` leftovers). That contract drift remains. ^[ambiguous]
- Operational invariant since 2026-08-29: live routes must match `create_hive_registry`. A route without a command (the `/roles` 500) is a defect — [[projects/hive/concepts/command-registry-route-parity]].
- Runtime behavior depends on `[[concepts/structured-service-configuration]]`, especially the `HIVE` env prefix, command retry settings, queue transport, and the outbound `iam_service` and `external_provider_service` sections.
- Hive publishes `[[projects/hive-events/hive-events]]` domain events for organization, member, invitation, external-link, and sync-job changes rather than treating HTTP as the only integration surface.
- Hive uses the shared `[[projects/rustycog/rustycog]]` stack, but it diverges from IAMRusty and Telegraph in its custom HTTP error model and in how much of its command or spec surface is actually exposed over HTTP. Conflict to resolve. ^[ambiguous]
- Hive treats `hive-events` as event-contract vocabulary and relies on `[[projects/rustycog/references/rustycog-events]]` for queue transport and publisher runtime behavior.
- T14b (HEAD `a27ea5b`) : HTTPS compose via dual-bind rustycog, hôte **8443**, CA mesh `./certs/platform-mesh` **distincte** de la CA Lazaret, authentification client **optionnelle** — pas un rustls required. Preuve `Hive/tests/https_mesh_optional_mtls.rs`. Concept : [[projects/aiforall/concepts/https-platform-mesh]].

## Related

- [[projects/hive/references/hive-service]] - Code-backed overview of Hive's crate layout, runtime wiring, and shared dependencies.
- [[projects/hive/references/hive-entity-model]] - Organization, membership, RBAC, and integration entities owned by Hive.
- [[projects/hive/references/hive-runtime-and-configuration]] - `HIVE_*` config loading, queue publishing, retry settings, and service-to-service config.
- [[projects/hive/references/hive-http-api-and-openapi-drift]] - The live route table, custom error surface, and the gaps between shipped HTTP and `openspecs.yaml`.
- [[projects/hive/references/hive-command-execution]] - Registry coverage, command names, and event-publishing use cases.
- [[projects/hive/references/hive-data-model-and-schema]] - Organizations, members, invitations, external links, sync jobs, and permission tables.
- [[projects/hive/references/hive-testing-and-api-fixtures]] - Real DB, JWT, and external-provider fixture patterns in the Hive tests.
- [[projects/hive/skills/building-organization-management-services]] - Reusable workflow for building Hive-style org-management services.
- [[projects/aiforall/concepts/https-platform-mesh]] — T14b HTTPS + CA client optionnelle.

## Open Questions

- The service has no Hive-local README in this tree, so Cargo metadata, OpenAPI, config, and code are the main documentation sources.
- The OpenAPI contract, registered routes, and command registry do not currently describe the same breadth of operations. Conflict to resolve. ^[ambiguous]

## Sources

- [[projects/hive/references/hive-service]]
- [[projects/hive/references/hive-runtime-and-configuration]]
- [[projects/hive/references/hive-http-api-and-openapi-drift]]
- [[projects/hive/references/hive-command-execution]]
- [[projects/hive/references/hive-data-model-and-schema]]
- [[projects/hive/references/hive-testing-and-api-fixtures]]