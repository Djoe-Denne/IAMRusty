---
id: STORY-003.03.04
title: Qualify bounded session revocation through deployed admission paths
status: Done
assignee: []
created_date: '2026-11-27T15:54:29Z'
updated_date: '2026-11-28T13:00:58Z'
labels:
  - iam
  - session-liveness
dependencies:
  - STORY-003.03.03
documentation:
  - docs/contracts/password-recovery-session-liveness.md
  - docs/missions/20261128-password-recovery-replica-qualification.md
parent_task_id: STORY-003.03
priority: high
type: task
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire the chosen release shape into the approved test environment and verify the backend flow across supported direct, monolith and Envoy mesh paths. Preserve separate evidence for normal replica decisions, fallback and refusal. Scope cloud provisioning separately where applicable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A confirmed reset denies unexpired prior tokens within five seconds on supported protected paths, including mesh requests to downstream services; a new login works. The normal mesh run proves that the decision came from the copy without calling IAM per admission.
- [x] #2 When application of replication changes stalls while the copy stays reachable, verify capped IAM fallback or temporary denial by the deadline, then verify return to replica admission after catch-up.
- [x] #3 Complete the boundary inventory and the applicable legacy, optional-auth, unavailable-authority and concurrent-renewal checks. Keep results tied to the deployed revision and record coverage gaps.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed at the owner's request. The [hosted qualification](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46144607978) passed the tested direct, nested and Kind mesh cases and the single-worker stalled-replica, outage and catch-up checks. Aggregate fallback saturation and a separate IAM capacity reservation were not verified; they remain required before rollout. See [PR #62](https://github.com/Djoe-Denne/IAMRusty/pull/62).
<!-- SECTION:FINAL_SUMMARY:END -->
