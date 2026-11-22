---
id: STORY-006.01
title: Align existing provider signup outcomes for supplied email
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:54:35Z'
labels:
  - iam
  - backend-alignment
dependencies:
  - STORY-007.01
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-006
priority: high
type: bug
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-006 E1, E3 to E5 and E7. High value, medium effort after the federation contract. The existing signup flow may associate by matching email and withholds first issuance for an unverified supplied address. Reuse current registration and connector boundaries. Coordinate release with the linked-login correction. No new missing-email collection, existing-account handoff, or username prefilling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A registered address yields a non-identifying existing-account outcome without creating an account, draft, link, or resumption email. An already linked identity yields a sign-in outcome rather than a duplicate.
- [ ] #2 Completing eligible signup with a supplied available address creates one account and grants its first session. Only trusted proof of the exact address marks it verified.
- [ ] #3 Abandoning or failing to complete signup grants no ordinary access. Integration evidence observes the first session at completion, not on later sign-in.
<!-- AC:END -->
