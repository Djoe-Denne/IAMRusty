---
title: Anonymous Public-Read via Wildcard Subject
category: concepts
tags: [concept, permissions, openfga, public-read, visibility/internal]
sources:
  - rustycog/rustycog-permission/src/lib.rs
  - rustycog/rustycog-permission/src/checker.rs
  - rustycog/rustycog-http/src/middleware_permission.rs
  - rustycog/rustycog-testing/src/permission/service.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - manifesto-events/src/project.rs
  - Manifesto/application/src/usecase/project.rs
summary: How anonymous read of public projects flows through OpenFGA `user:*` — middleware, create-as-public, and ProjectVisibilityChanged writes are live; publish is lifecycle only.
provenance:
  extracted: 0.78
  inferred: 0.16
  ambiguous: 0.06
created: 2026-04-22T18:30:00Z
updated: 2026-09-02T19:45:00Z
---

# Anonymous Public-Read via Wildcard Subject

The platform's authorization story is "every decision goes through the centralized [[concepts/openfga-as-authorization-engine]] checker, no per-route bypass." That sentence works cleanly for authenticated callers, but until 2026-04-22 it had a hard limit: the `optional_permission_middleware` rejected anonymous callers with `403 FORBIDDEN` whenever the request path carried a resource UUID, **before** consulting the checker. Public-read of a specific project worked only in `tests/public_acl_api_tests.rs` unit tests against the read repository — never end-to-end through HTTP.

This page documents the **wildcard subject pattern**, what shipped in Phase 1 (2026-04-22), and the 2026-09-02 runtime status of the tuple writes. Org-owned participation limits sit on [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].

## The wildcard subject

`[[projects/rustycog/references/rustycog-permission]]` ships a `Subject` that can model the special "any user" subject:

```rust
pub enum SubjectKind { User, Wildcard }

pub struct Subject {
    pub user_id: Uuid,
    pub kind: SubjectKind,
}

impl Subject {
    pub fn new(user_id: Uuid) -> Self { /* User */ }
    pub fn wildcard() -> Self { /* Wildcard, user_id: Uuid::nil() */ }
}

impl Display for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SubjectKind::Wildcard => write!(f, "user:*"),
            SubjectKind::User => write!(f, "user:{}", self.user_id),
        }
    }
}
```

`user:*` is the OpenFGA wire form for "any user". Combined with a model relation declared as `[user, user:*]`, a single tuple `project:{id}#viewer@user:*` grants every caller (authenticated or not) the `viewer` relation on that project — and `viewer` derives `read` per the model.

## Phase 1 — what's shipped today (2026-04-22)

```mermaid
flowchart LR
    Anon["Anonymous request"] --> Mw["optional_permission_middleware"]
    Mw -- "Subject::wildcard()" --> Checker["OpenFgaPermissionChecker.check"]
    Checker -- 'POST /check { user: "user:*" }' --> Fga["OpenFGA store"]
    Fga -- "no viewer@user:* tuple" --> Deny["allowed: false -> 403"]
    Fga -- "create-as-public or visibility flip to public" --> Allow["allowed: true -> 200"]
```

The four shared-layer changes:

1. **`Subject::wildcard()` constructor and `SubjectKind` discriminant** in `rustycog-permission`. The struct shape is preserved (existing `Subject::new(uuid)` call sites in Hive, Telegraph, IAMRusty, Manifesto are untouched). `#[serde(default)]` on the new `kind` field keeps wire compatibility with payloads serialized before the field existed.
2. **`CachedPermissionChecker` bypasses the cache for wildcard subjects.** The cache key is `(user_id, permission, object_type, object_id)` — wildcard reuses `Uuid::nil()`, which would collide across every anonymous request and let one project's public-read decision answer for another. Skipping the cache also means a public→private flip (when sentinel-sync removes the wildcard tuple in Phase 2) is observed on the very next request rather than after the TTL window. ^[inferred]
3. **`optional_permission_middleware` consults the checker with `Subject::wildcard()`** instead of short-circuiting with 403 on missing JWT. Fail-closed semantics are preserved: relations without a wildcard tuple still return `allowed: false` and the request 403s.
4. **OpenFGA model** declares `project.viewer: [user, user:*, organization#member] or member` so the store will accept `viewer@user:*` writes and Internal org-member usersets.

The `[[projects/rustycog/references/openfga-mock-service]]` test fake gained `mock_check_allow_wildcard(action, resource)` and `mock_check_deny_wildcard(action, resource)` helpers so test suites can arrange anonymous-read decisions without constructing `Subject::wildcard()` themselves.

### Status as of 2026-09-02

Phase 1 middleware is live. Tuple writes for public-read:

- `ProjectCreated` with `visibility == "public"` writes `viewer@user:*`.
- `ProjectVisibilityChanged` writes the wildcard on a flip **to** public and deletes it on a flip **from** public.
- `ProjectPublished` is a tuple no-op (lifecycle, not visibility).
- `ProjectArchived` and `ProjectDeleted` delete the wildcard.
- `Tuple::wildcard_user` exists on the sentinel-sync client.

Still open:

- **Anonymous list vs GET.** The list SQL shows `visibility=public` without asking OpenFGA. GET `{id}` needs the wildcard tuple (or another viewer relation). Authenticated org list/GET are aligned.
- Public world-read is unchanged. Self-join is a separate route. See [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].
- Historical private-published wildcards may remain until `OpenFgaWriteClient::reconcile_wildcards`.

## Remaining work (was Phase 2)

Shipped: `ProjectVisibilityChangedEvent`, `update_project` emit-on-flip, translator arm, `ProjectPublished` no-op.

Still needed for correct public-read:

1. **Revert remaining Phase 1 test authentications** in `Manifesto/tests/project_api_tests.rs` back to anonymous where the scenario is public-read, and arrange `openfga.mock_check_allow_wildcard(Permission::Read, project_resource)`.
2. **Add a true end-to-end public-read test** that creates a public project, asserts the `viewer@user:*` tuple gets written, then issues an anonymous GET and asserts `200`. Also cover publish-must-not-grant-wildcard on a private project.
3. **Production data backfill** for leftover wildcards written by historical `ProjectPublished` on private projects.

## Cleanup invariants (the hard part)

`sentinel-sync` now removes `user:*` on `ProjectVisibilityChanged` (from public) and on `ProjectArchived`. Three failure modes remain:

- **Crash between DB write and event publish.** Manifesto's `update_project` writes the DB row first, then publishes the event. If the publish fails after the DB succeeds, the OpenFGA tuple stays out of sync. GET/details/components now ignore a leftover wildcard when the row is no longer world-readable (`visibility != public` or status `archived|suspended`). Anonymous list also hides `public`+`archived`. Remaining drift is a live public row visible in the list before the wildcard exists; a periodic reconciler still covers that.
- **Race between two concurrent `update_project` calls.** Both publish events; sentinel-sync sees them in some order. As long as `(old, new)` is in the payload, the second event's translation is "idempotent in terms of the final state" — the last-applied flips OpenFGA to whatever the last DB write was. The dedicated event payload is what makes this work; an enriched `ProjectUpdatedEvent` with only `updated_fields: ["visibility"]` would not. ^[inferred]
- **Replay of a `ProjectVisibilityChanged` event.** The translator must be safe to apply twice. `Write` and `Delete` against OpenFGA are idempotent (writing an existing tuple is a no-op; deleting an absent tuple errors but can be swallowed). ^[inferred]

## Not in scope (Phase 1 or 2)

- **`component.viewer` wildcard.** The model derives `component.viewer` from `member from project`, so a project's `viewer@user:*` doesn't currently propagate to its components. A separate model edit + sentinel-sync change is needed if Manifesto wants public-component-read on private projects.
- **`organization.viewer` wildcard.** No use case in any service today.
- **`Visibility::Internal` semantics.** Means "anyone in the same org", which is a different tuple shape (`viewer@organization:{id}#member`) and a separate design conversation.

## Sources

- [[projects/rustycog/references/rustycog-permission]] — `Subject::wildcard()` and the cache bypass.
- [[projects/rustycog/references/openfga-mock-service]] — `mock_check_*_wildcard` helpers.
- [[projects/rustycog/references/wiremock-mock-server-fixture]] — singleton listener that the wildcard tests share.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] — Manifesto routes that depend on this work.
- [[projects/manifesto/references/manifesto-testing-and-fixtures]] — current test wiring + the temporary auth on the 3 GET tests.
- [[projects/sentinel-sync/references/event-to-tuple-mapping]] — live translator rows, including `ProjectVisibilityChanged`.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] — org-owned list/GET split, missing partnership/join, dead Internal/OrgCascade flags.
- [[concepts/openfga-as-authorization-engine]] — surrounding architecture.
- [[concepts/centralized-authorization-service]] — the contract this pattern satisfies.
