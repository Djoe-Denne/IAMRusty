---
title: Using RustyCog Permission
category: skills
tags: [rustycog, permissions, skills, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/casbin.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
summary: Procedure for implementing PermissionsFetcher and applying RustyCog permission checks in route middleware.
provenance:
  extracted: 0.89
  inferred: 0.05
  ambiguous: 0.06
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Using RustyCog Permission

Use this guide when integrating `<!-- [[projects/rustycog/references/rustycog-permission]] -->`.

## Workflow

- Implement `PermissionsFetcher` in your domain/service layer so it resolves permissions for `(user_id, resource_ids)`.
- Model resource IDs in route paths as UUIDs so middleware can extract them into `ResourceId`.
- Define Casbin model files per protected resource and keep them in a dedicated permissions directory.
- In route setup, attach permission fetchers and required permissions through `RouteBuilder`.
- Validate key authorization flows with integration tests that exercise real fetcher logic, not mock booleans.

## Common Pitfalls

- Treating fetchers as global role lookup only and ignoring resource-specific context.
- Using inconsistent path/resource-ID conventions across routes.
- Assuming permission inheritance behavior without checking the Casbin policy expansion path.

## Sources

- <!-- [[projects/rustycog/references/rustycog-permission]] -->
- <!-- [[entities/permissions-fetcher]] -->
- <!-- [[concepts/resource-scoped-permission-fetchers]] -->
