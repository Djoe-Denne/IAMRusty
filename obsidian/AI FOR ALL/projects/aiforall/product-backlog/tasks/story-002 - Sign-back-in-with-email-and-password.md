---
id: STORY-002
title: Sign back in with email and password
status: Accepted intent
assignee: []
created_date: '2026-11-21T15:12:49Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
  - services/IAMRusty/qa/scenarii/12_login_email_password_success.md
  - services/IAMRusty/qa/scenarii/13_login_email_password_failure.md
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A returning person with a completed account and confirmed email address can regain access with their email address and password.

## Delivery scope

This is approved product intent v1 for the complete journey. Password sign-in and incorrect-credentials rejection are existing backend behavior. The interface, including messages and post-sign-in navigation, is part of the story but deferred from delivery. No delivery tasks are scheduled here; keep backend and interface work separate if either is later requested.

STORY-001 covers incomplete registration and unconfirmed email addresses. Password recovery, adding another sign-in method, and session renewal are separate stories.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for the existing API behavior. The referenced QA scenarios are evidence candidates, not additional product rules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Given a completed account with a confirmed address, when the person selects Sign in from an AIForAll page such as a public project and enters the correct email and password, they sign in and return to that same page.
- [ ] #2 E2 - Given a completed account with a confirmed address and no originating page, when the person opens sign-in directly and enters the correct email and password, they sign in and land on the AIForAll home page.
- [ ] #3 E3 - Given an unknown email or an incorrect password, when the person tries to sign in, no session is created. The interface reports that the email address or password is incorrect without disclosing which one.
<!-- AC:END -->
