---
id: STORY-003.02
title: Decide how password reset ends existing protected access
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:54:43Z'
labels:
  - iam
  - backend-alignment
  - architecture
dependencies: []
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-003
priority: high
type: spike
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-003 E3 and STORY-010 E2, limited to the existing reset behavior. High value, medium effort. The IAM repository purges refresh rows but access JWTs remain valid on direct and mesh routes as documented in docs/platform/authn-jwt.md. Compare durable session liveness with other compatible enforcement designs for latency, availability, migration, and boundary coverage. No logout feature, browser binding, factor flow, or change to signed-in password change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The architect records the chosen authority, alternatives, compatibility and failure behavior in a Proposed ADR and referenced contract.
- [ ] #2 Specify when reset takes effect for each access and renewal path, including concurrent renewal, and define integration oracles.
- [ ] #3 Keep requesting a reset distinct from successfully changing the password through the link.
<!-- AC:END -->
