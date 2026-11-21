---
id: STORY-012
title: View and update my account profile
status: Accepted intent
assignee: []
created_date: '2026-11-21T19:37:58Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A signed-in person can see their own AIForAll profile and update the username chosen at signup and their avatar. They can see their email address without changing it in this flow.

## Scope

This is approved product intent v1. Confirming a saved email address belongs to STORY-008. Changing an address or its eligibility for sign-in or recovery is outside this story. Avatar formats, reuse of old usernames, and what other people can see are not specified here.

The interface is part of the intended journey. No delivery is scheduled. See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md#compte) for existing profile behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A signed-in person can open their own profile and see their saved username, avatar, and email address. The email address is not editable in this flow.
- [ ] #2 E2 - When the person saves a valid change to their username, avatar, or both, the profile shows the new values. Untouched fields stay the same.
- [ ] #3 E3 - If the proposed username is already used by another account or a submitted value is invalid, explain what needs correcting without changing the saved profile.
- [ ] #4 E4 - If the person cancels before saving, discard the pending changes and leave the saved profile unchanged.
<!-- AC:END -->
