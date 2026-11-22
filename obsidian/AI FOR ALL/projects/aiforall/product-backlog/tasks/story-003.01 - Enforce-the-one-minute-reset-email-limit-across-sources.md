---
id: STORY-003.01
title: Enforce the one-minute reset email limit across sources
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:53:59Z'
labels:
  - iam
  - backend-alignment
dependencies: []
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-003
priority: high
type: bug
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-003 E1. High value, medium effort. The existing reset use case generates an event per eligible request, and the router limits admission rather than email issuance. Reuse the IAM writer and outbox under ADR 0301. Decide durable cooldown placement and retry semantics before tests. No new mail provider, password disablement, or alerts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Known and unknown addresses receive the same acknowledgement. A request does not end sessions or lock the account.
- [ ] #2 Concurrent eligible requests for the same normalized address from different sources produce at most one reset email in one minute.
- [ ] #3 An eligible request after the interval can produce a new reset email. Integration evidence covers the authoritative boundary and retry behavior.
<!-- AC:END -->
