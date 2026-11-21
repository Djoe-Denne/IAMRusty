---
id: STORY-011
title: Sign out of AIForAll
status: Accepted intent
assignee: []
created_date: '2026-11-21T19:34:07Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person signed in to AIForAll can deliberately end their current session so it cannot grant protected access or be renewed afterward. Signing out affects only that session, not separate sessions in other browsers or on other devices.

## Scope

This is approved product intent v1. Signing out everywhere, signing out of an external provider, and ending sessions during password recovery are separate from ordinary sign-out. Signing out does not delete the account or remove a sign-in method. Closing and reopening a browser without signing out is covered in STORY-010.

The interface is part of the intended journey. The exact feedback when sign-out cannot be completed is not specified. No delivery is scheduled. See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md#session) for existing session behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - When sign-out succeeds, the ended session no longer grants protected access and cannot be renewed. Returning to a protected page requires a new sign-in.
- [ ] #2 E2 - If multiple tabs share the current session, signing out in one ends protected access and renewal for all of them. Separate sessions in other browsers or on other devices remain active.
- [ ] #3 E3 - If the person signs out from a public page, that page remains open in its signed-out state.
- [ ] #4 E4 - If the person signs out from a protected page, AIForAll takes them to its public home page instead of continuing to show the protected page.
<!-- AC:END -->
