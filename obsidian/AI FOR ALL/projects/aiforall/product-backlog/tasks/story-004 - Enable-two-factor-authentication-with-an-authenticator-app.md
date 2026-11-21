---
id: STORY-004
title: Enable two-factor authentication with an authenticator app
status: Accepted intent
assignee: []
created_date: '2026-11-21T16:33:16Z'
labels: []
dependencies: []
documentation:
  - https://guide.duo.com/third-party-accounts
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A signed-in account holder can choose to protect future password sign-ins with a second factor from an authenticator app. The app adds a factor; it is not a substitute for the password.

## Method

Enrollment is optional and does not require a security incident. The method uses standard time-based one-time passcodes (TOTP). [Duo Mobile](https://guide.duo.com/third-party-accounts) is an example of a compatible app in code mode, not a required provider or an integration with Duo Push.

## Delivery scope

This is approved product intent v1. The interface is part of the intended journey, but no delivery is scheduled. Loss or replacement of the authenticator and the recommendation after a detected intrusion attempt are separate stories. Password recovery must not bypass an enabled second factor; this does not define how a lost authenticator is recovered.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A signed-in account holder can choose to set up a TOTP authenticator app. Two-factor authentication is enabled only after they enter a valid first code from the app.
- [ ] #2 E2 - If the code is invalid or setup is cancelled or left incomplete, two-factor authentication stays off.
- [ ] #3 E3 - After activation, a correct password alone does not open a session; the person must also enter a valid app code. A person who has not activated two-factor authentication is not asked for that code.
<!-- AC:END -->
