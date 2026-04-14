---
title: Organization-Resource Authorization
category: concepts
tags: [authorization, permissions, organizations, visibility/internal]
sources:
  - Hive/http/src/lib.rs
  - Hive/domain/src/service/permission_service.rs
  - Hive/infra/src/repository/entity/hive_database_schema.sql
  - Hive/resources/permissions/organization.conf
  - Hive/tests/members_api_tests.rs
  - Hive/tests/organization_api_tests.rs
summary: Hive combines RustyCog route guards with runtime organization, member, role, and resource permission lookups instead of using static route-level policies alone.
provenance:
  extracted: 0.76
  inferred: 0.14
  ambiguous: 0.10
created: 2026-04-14T18:56:22.3888182Z
updated: 2026-04-14T20:28:20.9129598Z
---

# Organization-Resource Authorization

`[[projects/hive/hive]]` uses Casbin-style route policies, but the real authorization work happens in domain-backed permission fetchers. That gives Hive a richer organization/member/resource model than a static route ACL, while still fitting inside `rustycog_http::RouteBuilder`. It is Hive's organization-scoped specialization of `[[concepts/resource-scoped-permission-fetchers]]`.

## Key Ideas

- `RouteBuilder` wires separate permission fetchers for `organization`, `member`, and `external_link` resources, then combines them with `Permission::Read`, `Permission::Write`, or `Permission::Admin` checks on individual routes.
- `ResourcePermissionFetcher` loads the organization from the first extracted `ResourceId`, resolves the current member and that member's roles, filters role permissions to the configured resource names, and maps the resulting domain permission levels back into `rustycog_permission::Permission` values.
- Hive's schema models organizations, members, roles, permissions, resources, and role-permission junctions as first-class tables, so route guards are backed by persisted organization membership state rather than ad hoc role strings.
- Some organization routes are intentionally public or partially public through `might_be_authenticated()`, while most mutation and listing routes require authenticated users plus resource-scoped permission checks.
- The integration tests confirm the behavior users actually feel: unauthenticated requests get `401`, read-only members get `403` on admin or write operations, and owners can modify organization or membership state.
- Conflict to resolve: Hive's permission integration is organization-scoped and multi-table, whereas Telegraph uses a narrower notification-ownership fetcher and the current IAMRusty pages focus more on auth flows than on Casbin-backed resource resolution. ^[ambiguous]

## Open Questions

- `fetch_permissions()` currently anchors authorization on the first extracted resource ID, which works for today's route patterns but would need care if future routes reordered or mixed resource IDs. ^[inferred]
- The Casbin model files are intentionally simple, so the real complexity lives in fetchers and repositories rather than in the policy DSL itself. ^[inferred]

## Sources

- [[projects/hive/hive]] - Service where this authorization model is used.
- [[projects/hive/references/hive-http-api-and-openapi-drift]] - Live route surface that consumes the permission model.
- [[projects/hive/references/hive-data-model-and-schema]] - Tables that back the role and resource model.
- [[concepts/resource-scoped-permission-fetchers]] - Shared RouteBuilder plus PermissionsFetcher pattern across services.
- [[projects/hive/skills/building-organization-management-services]] - Practical workflow that applies this pattern.