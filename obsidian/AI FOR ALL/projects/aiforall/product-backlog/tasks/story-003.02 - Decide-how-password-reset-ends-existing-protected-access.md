---
id: STORY-003.02
title: Decide how password reset ends existing protected access
status: Done
assignee: []
created_date: '2026-11-22T15:54:43Z'
updated_date: '2026-11-26T16:45:27Z'
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
- [x] #1 The architect records the chosen authority, alternatives, compatibility and failure behavior in a Proposed ADR and referenced contract.
- [x] #2 Specify when reset takes effect for each access and renewal path, including concurrent renewal, and define integration oracles.
- [x] #3 Keep requesting a reset distinct from successfully changing the password through the link.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ADR-0316 and the password-recovery session-liveness contract are Accepted in commit 69b3359. They define the durable account epoch, reset-commit boundary, direct/mesh/concurrent-renewal oracles, legacy rule, and fail-closed behavior. STORY-003.03 remains open pending shared RustyCog direct admission, IAM liveness wiring, real mesh evidence, outage/optional-auth/legacy oracles, and deterministic late-response renewal ordering.
<!-- SECTION:FINAL_SUMMARY:END -->
