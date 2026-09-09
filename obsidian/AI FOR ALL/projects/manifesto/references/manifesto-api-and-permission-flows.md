---
title: Manifesto API and Permission Flows
category: references
tags: [reference, api, permissions, openfga, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/http/src/lib.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/application/src/usecase/project.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/application/src/usecase/member.rs
  - manifesto-events/src/lib.rs
  - sentinel-sync/src/translator/manifesto.rs
  - openfga/model.fga
summary: "Routes et ACL Manifesto actuelles ; les API candidates de binding/invoke Apparatus restent futures et gardent le préfixe /manifesto."
provenance:
  extracted: 0.75
  inferred: 0.23
  ambiguous: 0.02
created: 2026-04-19T12:00:00Z
updated: 2026-09-09T17:50:00Z
---

# Manifesto API and Permission Flows

This page assumes the shared [[projects/rustycog/references/rustycog-http]] and [[concepts/centralized-authorization-service]] patterns are already familiar. It keeps the route, command, and authorization details that are specific to [[projects/manifesto/manifesto]].

## RustyCog Baseline

- [[projects/rustycog/references/rustycog-http]] explains `RouteBuilder`, authentication modes, command-context extraction, and the centralized permission middleware.
- [[concepts/centralized-authorization-service]] explains why every check goes through one shared `Arc<dyn PermissionChecker>` and how tuples reach OpenFGA via [[projects/sentinel-sync/sentinel-sync]].
- [[projects/rustycog/references/rustycog-command]] covers the shared command execution runtime that the handlers delegate into.

## Service-Specific Differences

- [Manifesto/http/src/lib.rs](../../../../../Manifesto/http/src/lib.rs) registers project, component, and member routes against the same shared `permission_checker` on `AppState`. There is no per-resource fetcher anymore.
- Project get/detail and component list/get routes are `.might_be_authenticated()` plus project `Read`. `optional_permission_middleware` resolves anonymous callers as `Subject::wildcard()`. The use-case world-read gate then requires `visibility=public` and status `draft` or `active`. A leftover `viewer@user:*` after private / internal / archive is ignored on those surfaces. sentinel-sync writes that tuple on **create-as-public** and on a real visibility flip (`ProjectVisibilityChanged`). Manifesto does not write FGA. `ProjectPublished` does not write it. Authenticated reads succeed for owner, project member, or organization `Read` on org-owned projects.
- Authenticated `GET /api/projects` SQL matches GET for org-inherited access. Anonymous list can show live public rows without the wildcard. See [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].
- `POST /api/projects/{id}/join` is JWT-only. It inserts or restores membership with `project`/`read`. Adding another user is still `Admin` on the project.
- Mutations require an active DB member or org admin ([[projects/manifesto/concepts/immediate-membership-acl]]). Suspended GET/members/mutations are owner / project admin / org admin only.
- Component routes use `"project"` as the OpenFGA object type when the deepest UUID is the project id. Instance ACL is enforced in the use case. Generic vs instance grants: [[projects/manifesto/concepts/component-instance-permissions]].
- Member routes are project-scoped (`with_permission_on(Permission::Admin, "project")` for writes; list/get require `Read` plus the Suspended gate).
- Permission grant/revoke endpoints emit `PermissionGrantedEvent` / `PermissionRevokedEvent`. v2 tuples are exact; incomplete v1 destructives are no-ops in sentinel-sync.
- `ComponentUseCaseImpl` keeps domain state and emitted events synchronized so the OpenFGA tuple graph stays consistent.
- `ProjectDetailResponse` and `ComponentResponse` still leave `endpoint` and `access_token` as `None`, so the API currently exposes component attachment metadata rather than a provisioning handoff.

## Apparatus — API candidates futures

[[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]] propose des commandes asynchrones de binding et une API invoke médiatisée. Ces routes ne sont pas présentes aujourd’hui ; la compatibilité de `/components` et les gates projet/instance doivent être maintenues. `endpoint` et `access_token` restent non renseignés dans le DTO actuel.

Le futur host [[projects/manifesto/references/apparatus-ui-and-protocol]] ne reçoit pas d’endpoint privé ni ne transmet de bearer IAM au plugin. Les méthodes SDK passent par un gateway qui vérifie de nouveau droits, consentement et état du binding. ^[inferred]

## Open Questions

- Should Manifesto eventually surface a richer operator-facing story for component provisioning and component-scoped tokens?
- Should component routes adopt UUID `{component_id}` parameters so the middleware can guard against `"component"` directly?

## Sources

- [[projects/manifesto/manifesto]]
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]]
- [[projects/manifesto/concepts/component-instance-permissions]]
- [[concepts/centralized-authorization-service]]
- [[concepts/openfga-as-authorization-engine]]
- [[projects/sentinel-sync/references/event-to-tuple-mapping]]
- [[projects/manifesto/references/manifesto-event-model]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] — join, list/GET, partnership gap.
- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[concepts/anonymous-public-read-via-wildcard-subject]]
