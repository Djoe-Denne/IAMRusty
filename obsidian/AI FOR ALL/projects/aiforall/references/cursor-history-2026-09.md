---
title: >-
  Cursor history September 2026
category: references
tags: [reference, cursor, aiforall, manifesto]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/54cbe718-8042-4a4e-8a15-767a7c0e15ba/54cbe718-8042-4a4e-8a15-767a7c0e15ba.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23280216-f48f-426e-8a11-6b42b8184a6e/23280216-f48f-426e-8a11-6b42b8184a6e.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/208e7f4b-556a-478d-80c2-dcdd79e8475b/208e7f4b-556a-478d-80c2-dcdd79e8475b.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/7d3c467c-eaeb-49f6-a6e3-f6b4ef5a68b2/7d3c467c-eaeb-49f6-a6e3-f6b4ef5a68b2.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/e9a1f9f4-d707-4a08-a607-638340586b89/e9a1f9f4-d707-4a08-a607-638340586b89.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/2924d20c-07e2-4741-a4aa-f3eb9e48c5ca/2924d20c-07e2-4741-a4aa-f3eb9e48c5ca.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/3c82c7d9-a42d-448f-8573-90ed739ba6b6/3c82c7d9-a42d-448f-8573-90ed739ba6b6.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
summary: >-
  Distilled eight Cursor parent sessions after 2026-09-02: visibility/join, Manifesto AuthZ correction, rustycog main pin, CI tuple reads.
provenance:
  extracted: 0.70
  inferred: 0.25
  ambiguous: 0.05
created: 2026-09-09T16:45:00Z
updated: 2026-09-09T16:45:00Z
---

# Cursor history September 2026

Follows [[projects/aiforall/references/cursor-history-2026-04-to-08]]. Subagent jsonl skipped. Distilled by topic.

## Visibility and join (2–5 Sep)

- Public is world-read (`viewer@user:*`). `publish` is lifecycle only; `ProjectPublished` is a tuple no-op.
- `POST /api/projects/{id}/join` is live: public + active, `MemberSource::Direct`, grant `project`/`read`.
- Authenticated list/GET aligned for org Internal/admin. Anonymous list can show public SQL rows without the wildcard; GET still needs it.
- Red-team on uncommitted tests: do not fake sentinel-sync e2e inside Manifesto HTTP ITs. Component-catalog stubs use [[projects/rustycog/references/isolated-wiremock-fixture]].

## Manifesto AuthZ correction (9 Sep)

Durable contracts:

- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[projects/manifesto/concepts/membership-restore-and-cas]]
- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]]
- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]]
- [[projects/manifesto/concepts/component-instance-permissions]] — `component_editor` / `component_viewer`, not `member from project`

A leftover OpenFGA Write without an **active** `project_members` row is 403. Tests that only `grant_write` without a member must expect 403.

## RustyCog pin and CI

- Develop SDK in the sibling repo on **`main`**. Detached HEAD in `AIForAll/rustycog/` is normal; do not leave SDK commits there.
- Manifesto `org_scope` needs `RelationshipTuple` and `OpenFgaPermissionChecker::read_tuples` **on that pin**. Checking out `main` without cherry-picking those APIs breaks Manifesto and Monolith CI.

## Sonar

Clippy/Sonar passes on unpushed files and a coverage-to-80% test plan ran the same day. Treat as quality work, not a product-rule change. ^[inferred]

## Related

- [[journal/2026-09-09]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]
- [[projects/aiforall/concepts/rustycog-git-submodule]]
