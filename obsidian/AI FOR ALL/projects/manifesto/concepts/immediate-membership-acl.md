---
title: >-
  Immediate membership ACL
category: concepts
tags: [authorization, membership, manifesto, visibility/internal]
sources:
  - Manifesto/application/src/usecase/world_read.rs
  - Manifesto/application/src/usecase/member.rs
  - Manifesto/application/src/usecase/project.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Project mutations need an active DB membership or org admin. Leftover OpenFGA tuples do not authorize. Suspended is owner/admin only.
provenance:
  extracted: 0.85
  inferred: 0.12
  ambiguous: 0.03
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# Immediate membership ACL

`[[projects/manifesto/manifesto]]` treats **Postgres membership** as the source of who may mutate a project. OpenFGA remains the Check engine for org inheritance and world-read, but a stale Write tuple is not enough.

## Key Ideas

- `require_project_mutation_actor` allows a caller when they have an **active** `project_members` row **or** they are an org admin on an org-owned project.
- Granting OpenFGA `write` in a test without inserting a member yields **403**. That is the intended contract, not a fixture bug.
- `created_by` / `owner_id` on the project row is not an ACL bypass for component or member routes.
- **Suspended** projects: GET, list details, members, and mutations are limited to the project owner / project admin / org admin. Other members and the public are denied even if older tuples remain.
- World-read (`visibility=public` and live status) still uses the use-case gate plus `user:*`; it does not grant write.

## Why

Authorization events are async. Between revoke and sentinel-sync, OpenFGA can lag. Immediate ACL closes that window for writes. ^[inferred]

## Related

- [[projects/manifesto/concepts/membership-restore-and-cas]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]
- [[projects/manifesto/concepts/component-instance-permissions]]
- [[projects/manifesto/references/manifesto-api-and-permission-flows]]
- [[concepts/centralized-authorization-service]]
