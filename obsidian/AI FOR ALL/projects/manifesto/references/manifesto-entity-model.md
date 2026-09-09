---
title: Manifesto Entity Model
category: references
tags: [reference, entities, projects, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/domain/src/entity/project.rs
  - Manifesto/domain/src/entity/project_component.rs
  - Manifesto/domain/src/entity/project_member.rs
  - Manifesto/domain/src/entity/permission.rs
  - Manifesto/domain/src/entity/resource.rs
  - Manifesto/domain/src/entity/role_permission.rs
  - Manifesto/domain/src/entity/project_member_role_permission.rs
summary: "Entités actuelles de Manifesto et distinction avec les futurs Apparatus, releases, bindings 1:1, opérations et instances runtime."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
created: 2026-04-14T20:28:20.9129598Z
updated: 2026-09-09T17:50:00Z
---

# Manifesto Entity Model

This page lists the main entities `[[projects/manifesto/manifesto]]` owns in its project-service domain.

## Key Entities

- `Project` is the aggregate root and combines ownership, lifecycle, visibility, classification, and collaboration flags.
- `ProjectComponent` attaches one typed component instance to a project and tracks its own status lifecycle.
- `ProjectMember` stores user membership, source of addition, removal state, last access, and project-scoped permissions. `MemberSource` includes `OrgCascade` and `Invitation`, but live writes use `Direct` only.
- `Permission`, `Resource`, `RolePermission`, and `ProjectMemberRolePermission` mirror a project-scoped RBAC model beneath the project aggregate.
- Compared with Hive, Manifesto repeats the same broad authorization pattern at project scope instead of organization scope. ^[inferred]

## Entités Apparatus futures

`Apparatus`, `Release`, `ProjectApparatusBinding`, `Operation` et `Instance` sont des entités proposées dans [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]], absentes du modèle actuel. La V1 ajoute un binding 1:1 à `ProjectComponent`, conserve son UUID/ACL et maintient une installation par type canonique et projet. ^[inferred]

La release fournit le digest ; le binding exprime l’intention ; l’instance représente l’exécution. Le projet ne possède pas directement un pod. Le backfill historique ne doit pas créer de workload sans résolution de release et consentement. ^[inferred]

## Open Questions

- The current wiki still does not fully separate which Manifesto entities are stable MVP records and which are scaffolding for the broader component-service architecture described in its ADRs. ^[ambiguous]

## Sources

- [[entities/project]] - Canonical project entity page.
- [[entities/membership]] - Shared membership pattern across Manifesto and Hive.
- [[projects/manifesto/concepts/component-instance-permissions]] - How the RBAC entities and component entities interact.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - API behavior built on top of these entities.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] - Unused sources, flags, and org-owned access limits.
