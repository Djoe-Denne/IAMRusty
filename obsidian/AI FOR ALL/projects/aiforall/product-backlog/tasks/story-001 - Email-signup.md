---
id: STORY-001
title: Email signup
status: Accepted intent
assignee: []
created_date: '2026-11-21T14:58:04Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
  - docs/functional/projet.md
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person without an account can sign up with an email address and password, confirm the address, and then sign in separately. Reading an active public project does not require an account.

## Delivery scope

This is approved product intent v2 for the complete journey. Keep the existing backend behavior; no backend change is requested. The interface described in the examples is part of the story but deferred from delivery. A future delivery plan must scope backend and interface work separately; no delivery tasks are scheduled here.

General sign-in, account recovery, and adding a sign-in method are separate stories. Verification link lifetime and exact resend feedback are not specified here.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) and [project visibility guide](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/projet.md) for the existing behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A visitor can open and read an active public project without an account.
- [ ] #2 E2 - After entering an email address and password, the person chooses a username. Completing that step triggers the verification email without signing them in. The deferred interface prompts them to check their inbox and offers resend immediately, without a sign-in attempt.
- [ ] #3 E3 - A sign-in attempt before the username is chosen reports incomplete registration. After that step, IAM refuses password sign-in until the address is confirmed; neither case creates a session. For the unconfirmed-address result, the deferred interface shows the verification message and offers resend.
- [ ] #4 E4 - A valid verification link confirms the address without opening a session or issuing a sign-in cookie. The deferred confirmation screen invites the person to sign in separately with their password.
- [ ] #5 E5 - An expired link is refused without confirming the address. The existing resend operation can request a new email without sign-in; the deferred expired-link page offers that action directly.
<!-- AC:END -->
