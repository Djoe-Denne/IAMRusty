---
title: Component-Instance Permissions
category: concepts
tags: [permissions, components, projects, openfga, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - manifesto-events/src/authz.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: "ACL génériques et par instance component ; la future couche de capacités Apparatus conserve ces identités et ajoute le consentement."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
created: 2026-04-20T00:00:00Z
updated: 2026-09-09T17:50:00Z
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

## Apparatus — droits futurs

La proposition [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]] conserve `binding_id == component_id` et `component:{id}` via une extension 1:1. Les événements de phase runtime ne créent ni ne suppriment de tuples ; les événements ownership existants conservent leur rôle. ^[inferred]

Les capacités du manifeste et le consentement d’installation ajoutent une restriction ; ils ne remplacent ni l’ACL d’instance ni la gate DB de membre actif. La lecture publique actuelle des composants ne doit pas exposer implicitement le stockage, les secrets ou l’invoke du futur plugin. Voir [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]. ^[inferred]

## Related

- [[projects/manifesto/references/manifesto-api-and-permission-flows]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[projects/sentinel-sync/references/openfga-model]]
- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[concepts/openfga-as-authorization-engine]]
