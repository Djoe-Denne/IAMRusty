---
id: STORY-003
title: Recover account access by resetting the password
status: Accepted intent
assignee: []
created_date: '2026-11-21T16:33:04Z'
updated_date: '2026-11-27T15:53:52Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
  - services/IAMRusty/qa/scenarii/16_password_reset_unauthenticated_success.md
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person who has forgotten the password for an existing email/password account and still has access to its inbox can set a new password and regain access. A signed-in person who knows their current password can change it directly instead.

## Delivery scope

This is approved product intent v4. After a successful reset from a link, keep the existing confirmation response and separate sign-in. Earlier sessions cannot renew from the reset commit, and their platform access tokens stop granting new protected access on all supported paths within five seconds of that commit, including during replication failure. If admission cannot verify a sufficiently fresh decision, it fails temporarily. This revision changes E3 only; the signed-in current-password flow and its existing reauthentication policy are unchanged.

The interface is part of the story but deferred from delivery. The backend work for E3 is tracked under STORY-003.03; the one-minute reset email limit remains separate under STORY-003.01.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for the existing API behavior. The referenced QA scenario is an evidence candidate, not an additional product rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Anonymous requests for known and unknown addresses receive the same acknowledgement without disclosing account existence. An anonymous request does not end sessions or lock the account. A repeat request for the same address within one minute does not send another reset email, even from a different source.
- [ ] #2 E2 - Given a valid reset link, successfully saving a new password confirms the reset without creating a session. The interface invites the person to sign in separately with the new password.
- [ ] #3 E3 - From the reset commit, earlier sessions, including on other devices, cannot renew. Within five seconds of that commit, their platform access tokens no longer grant new protected access on any supported path, even during replication failure. If admission cannot verify a sufficiently fresh decision, it fails temporarily. This does not wait for token expiry or another sign-in.
- [ ] #4 E4 - An expired or already used link cannot reset the password. The interface offers a new reset request on that page without requiring sign-in.
- [ ] #5 E5 - A signed-in person who knows their current password can enter it with a valid new password and change it directly. AIForAll confirms the change without sending a reset email or opening a new session. An incorrect current password leaves the password unchanged.
- [ ] #6 E6 - If consecutive failures disabled password sign-in, verified recovery binds a new password and restores that method without signing the person in. Waiting alone does not restore password sign-in.
<!-- AC:END -->
