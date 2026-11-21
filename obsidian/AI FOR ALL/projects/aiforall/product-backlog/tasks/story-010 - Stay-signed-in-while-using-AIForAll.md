---
id: STORY-010
title: Stay signed in while using AIForAll
status: Accepted intent
assignee: []
created_date: '2026-11-21T19:21:41Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person already signed in to AIForAll can continue using it or return in the same browser without signing in again while their session is still valid or can be renewed. Closing the browser is not signing out.

## Scope

This is approved product intent v1. The journey starts after successful sign-in. Signing out and recovering account access are separate journeys. Session lifetime, idle expiration, and what happens during a temporary connection failure are not specified here. The interface is part of the intended journey; no delivery is scheduled.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md#session) for existing session API behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - While the person is using AIForAll, if their short-lived access expires but their session can still be renewed, they continue without a new sign-in prompt or an unexpected change of page.
- [ ] #2 E2 - If the session has ended and cannot be renewed, require normal sign-in before further protected activity. Renewal cannot reopen a session ended by sign-out or password recovery.
- [ ] #3 E3 - After the person closes the browser without signing out and returns in the same browser, resume their session without a sign-in prompt if it is still valid or renewable. Otherwise require sign-in before protected activity.
<!-- AC:END -->
