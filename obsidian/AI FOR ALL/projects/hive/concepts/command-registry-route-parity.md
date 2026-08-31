---
title: >-
  Command registry route parity
category: concepts
tags: [hive, http, commands, visibility/internal]
sources:
  - Hive/application/src/command/factory.rs
  - Hive/http/src/lib.rs
  - docs/reviews/iam-architecture-comparison.md
  - cursor-conversation/hive-roles-registry-2026-08-29
  - projects/hive/hive.md
summary: >-
  Hive live routes must match create_hive_registry. A route without a command, or a command without a route, is a defect — /roles was the 500 case.
provenance:
  extracted: 0.82
  inferred: 0.14
  ambiguous: 0.04
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Command registry route parity

Hive historically advertised more HTTP than it executed. The 2026-08-29 review called the OpenAPI contract **divergent**: invitations / `update_member` existed as handlers or spec paths without being wired, and `GET …/roles` was live while `create_hive_registry` had no matching command — the route returned 500.

## Rule

- Every live route in `create_router` must resolve through a registered command / use case.
- Every implemented command that is part of the public surface must have a route.
- Extract `register_*` / `setup_*` helpers instead of `#[allow(clippy::too_many_lines)]` on `create_hive_registry`. See [[projects/aiforall/skills/fixing-sonar-clippy-in-services]].

The older wiki note that “registry breadth is larger than the live route table” still describes the OpenAPI leftover. The operational invariant is now the other direction too: no live route without a command. ^[inferred]

## Related

- [[projects/hive/hive]]
- [[projects/hive/references/hive-http-api-and-openapi-drift]]
- [[projects/hive/references/hive-command-execution]]
- [[entities/command-registry]]
- [[concepts/command-registry-and-retry-policies]]
