---
id: STORY-015
title: Resolve my community responsibilities before deleting my account
status: Accepted intent
assignee: []
created_date: '2026-11-22T12:29:25Z'
labels: []
dependencies: []
documentation:
  - docs/guides/product-story-workflow.md
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person preparing to delete their AIForAll account can resolve indispensable responsibilities per organization or project so shared resources do not remain active without someone able to take responsibility.

## Scope

This is approved product intent v1 and a production prerequisite for account deletion, not a delivery commitment. Hive determines responsibilities and allowed actions for organizations; Manifesto does so for projects. IAM presents their results without redefining those rules. A role name alone does not make a responsibility indispensable. Archive authorized resources instead of deleting them; archival is not a backup of the erased account. An eligible invitation that survives erasure must not retain a copy of the erased account or its personal data.

The invitation lifetime, handling of notification delivery failures, and authority to arrange a new transfer after refusal or expiry remain open. A successful transfer and cancellation of the same invitation cannot both occur; the owning service arbitrates. Individual messages and contributions are separate, as is voluntary suspension.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - When a person starts account deletion, Hive and Manifesto report which organizations or projects would lose a required responsible person and which actions are allowed. IAM displays results as they arrive. No response is not a completed check.
- [ ] #2 E2 - For each blocking responsibility, the person can nominate an eligible successor or choose archival when authorized. Notify the nominee by email and in-app. Sending the invitation does not itself archive the resource or make the transfer effective.
- [ ] #3 E3 - Before IAM makes the account inaccessible and hidden, each affected resource must have an effective transfer to a valid successor or be archived. Queuing these actions does not allow the account to be hidden.
- [ ] #4 E4 - A designated successor can accept a valid pending invitation after the account has been erased. Once the transfer is effective, the authorized successor can explicitly unarchive the project. Nothing unarchives automatically; if the invitation expires or is declined and no responsible person is in place, the resource stays archived.
- [ ] #5 E5 - Before erasure, the initiator can prove control of the account and cancel a pending transfer without unarchiving the project. If the project is still active during preparation, they can explicitly select archival instead. Limited follow-up does not open an ordinary session for a hidden account, and cancellation cannot undo an effective transfer.
- [ ] #6 E6 - Explain before accepting deletion that cancellation restores the account state but does not undo completed transfers or archives or automatically cancel a transfer invitation.
<!-- AC:END -->
