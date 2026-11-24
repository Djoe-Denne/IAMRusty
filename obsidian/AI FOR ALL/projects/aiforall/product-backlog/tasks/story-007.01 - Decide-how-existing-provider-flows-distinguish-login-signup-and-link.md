---
id: STORY-007.01
title: Decide how existing provider flows distinguish login signup and link
status: Done
assignee: []
created_date: '2026-11-22T15:54:17Z'
updated_date: '2026-11-24T15:14:03Z'
labels:
  - iam
  - backend-alignment
  - architecture
dependencies: []
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-007
priority: high
type: spike
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-007 E1 to E3 and STORY-006 E1, E3 to E5 and E7. Preserve STORY-005 E1 to E3. High value, medium effort. The current OAuth flow combines linked lookup, email matching, and incomplete account creation. The architect compares intent in the current transaction with separate entry points, retaining OAuthConnect and browser binding under ADR 0412. Decide identity authority, mutation boundaries, and rollout. No implementation or new dependency in this ticket.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Record the selected design and rejected alternatives in a Proposed ADR and observable contract.
- [x] #2 Specify identity scope, support for omitted email in linked login, outcomes for each intent, and what may change on refusal or cancellation.
- [x] #3 Identify consumers, rollout constraints, and integration oracles for dependent tickets.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design deliverables only: [PR #51](https://github.com/Djoe-Denne/IAMRusty/pull/51) merged [ADR-0414](https://github.com/Djoe-Denne/IAMRusty/blob/5750672d/docs/adr/0414-oauth-provider-intents.md) and its [observable contract](https://github.com/Djoe-Denne/IAMRusty/blob/5750672d/docs/contracts/oauth-provider-intents.md). The documents compare alternatives, describe provider-scoped identity and no-mutation boundaries, and record affected consumers, cutover gates, and integration oracles. The ADR and contract remain Proposed and Unimplemented; open design decisions and contract approval still gate STORY-007.02 and STORY-006.01. No IAM implementation or future oracle pass is claimed.
<!-- SECTION:NOTES:END -->
