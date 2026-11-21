---
id: STORY-009
title: Recommend two-factor authentication after unusual sign-in attempts
status: Accepted intent
assignee: []
created_date: '2026-11-21T18:49:50Z'
labels: []
dependencies: []
documentation:
  - >-
    https://cheatsheetseries.owasp.org/cheatsheets/Credential_Stuffing_Prevention_Cheat_Sheet.html#notify-users-about-unusual-security-events
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person who has not enabled two-factor authentication (2FA) gets a clear recommendation to protect their AIForAll account after unusual sign-in attempts. This is separate from enabling 2FA in STORY-004.

## Trigger and presentation

This is approved product intent v1. Reuse the account-level security alert for repeated failed sign-ins. At the next successful sign-in, show a non-blocking notice if 2FA is still off. Describe unusual activity without claiming an account takeover. Offer Enable 2FA and Not now. Show each qualifying alert at most once in this journey; group multiple pending alerts into one notice. Do not send an extra email for the recommendation or suppress existing security alerts.

## Delivery scope

The interface is part of the intended journey. No delivery is scheduled. This story does not choose alert thresholds, change security controls, force 2FA activation, or specify exact interface copy. See [OWASP guidance on unusual security events](https://cheatsheetseries.owasp.org/cheatsheets/Credential_Stuffing_Prevention_Cheat_Sheet.html#notify-users-about-unusual-security-events) for notification guidance, not product thresholds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - When repeated failed sign-ins produce a qualifying security alert for an account without 2FA, show a recommendation to enable it at the next successful sign-in without blocking navigation.
- [ ] #2 E2 - Enable 2FA opens the separate activation journey. Choosing Not now or leaving activation incomplete does not change account access or enable 2FA.
- [ ] #3 E3 - Do not show the recommendation again for the same alert after it has been shown. Several qualifying alerts pending at sign-in result in one notice.
- [ ] #4 E4 - A single failed sign-in, an alert only about password-reset requests, or an account that already has 2FA enabled does not trigger this recommendation.
<!-- AC:END -->
