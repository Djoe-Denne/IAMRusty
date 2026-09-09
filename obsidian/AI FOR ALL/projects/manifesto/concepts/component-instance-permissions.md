---
title: Component-Instance Permissions
category: concepts
tags: [permissions, components, projects, openfga, visibility/internal]
sources:
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - manifesto-events/src/authz.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Generic component grants live on the project as component_editor/viewer. Instances inherit those, not project.member. Per-instance tuples attach to component:{id}.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-04-20T00:00:00Z
updated: 2026-09-09T16:45:00Z
---

# Component-Instance Permissions

Manifesto models components as children of projects in OpenFGA. A project **member** does **not** automatically view every instance.

From [openfga/model.fga](../../../../openfga/model.fga):

- `project.component_editor` / `project.component_viewer` — generic grants (admin implies editor; editor implies viewer).
- `component.editor` = direct user **or** `admin from project` **or** `component_editor from project`.
- `component.viewer` = direct user **or** `editor` **or** `component_viewer from project`.

`exact_user_tuple` in `manifesto-events` maps generic `resource == "component"` to those project relations. An instance UUID maps to `component:{id}#editor|viewer`.

## Event -> tuple mapping

See [[projects/sentinel-sync/references/event-to-tuple-mapping]]. Component-specific rows:

| Event | Tuples |
|--------------------------|---------------------------------------------------------------|
| `ComponentAdded` | `component:{component_id}#project@project:{project_id}` |
| `ComponentRemoved` | delete tuples on `component:{component_id}` |
| `PermissionGranted` | generic component → `project:{id}#component_viewer|component_editor`; instance → `component:{id}#viewer|editor` |
| `PermissionRevoked` | exact matching delete (v2). Incomplete v1 revoke is a **no-op**. |

## HTTP vs graph

Component routes still often use `with_permission_on(_, "project")` because the deepest UUID in the path may be the project id. Instance ACL is enforced in the use case (`caller_can_read_component`): owner, generic component grant, exact UUID grant, org admin, or world-readable public+active. Leftover FGA without an active member does not grant mutation ([[projects/manifesto/concepts/immediate-membership-acl]]).

## Related

- [[projects/manifesto/references/manifesto-api-and-permission-flows]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[projects/sentinel-sync/references/openfga-model]]
- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[concepts/openfga-as-authorization-engine]]
