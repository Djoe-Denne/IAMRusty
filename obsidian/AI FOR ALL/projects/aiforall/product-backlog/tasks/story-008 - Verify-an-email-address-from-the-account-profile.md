---
id: STORY-008
title: Verify an email address from the account profile
status: Accepted intent
assignee: []
created_date: '2026-11-21T18:37:57Z'
labels: []
dependencies: []
documentation:
  - >-
    https://cheatsheetseries.owasp.org/cheatsheets/Email_Validation_and_Verification_Cheat_Sheet.html#email-ownership-verification
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A signed-in account holder can confirm an unverified email already saved in their AIForAll profile, whether it is primary or secondary. This is separate from signup and linking an external provider.

## Confirmation rules

This is approved product intent v1. A signed-in person selects an address on their own profile. The link sent to that address is bound to that account and address, expires, and can succeed only once. Merely opening the link does not verify the address or consume the link. Confirmation requires a valid session on the same account and an explicit action. Signing in follows the normal rules, including any required 2FA; the email link does not satisfy sign-in or 2FA.

The profile interface is part of the intended journey. No delivery is scheduled. Adding or changing an address, enabling it for sign-in or recovery, and the exact link lifetime and resend limits are outside this story. See [OWASP email ownership verification](https://cheatsheetseries.owasp.org/cheatsheets/Email_Validation_and_Verification_Cheat_Sheet.html#email-ownership-verification) for the link safeguards.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A signed-in person selects a saved unverified primary or secondary address in their profile and requests verification. AIForAll sends a confirmation link to that address without marking it verified.
- [ ] #2 E2 - Opening a valid link does not change the address. Explicit confirmation while signed in to the correct account marks only the selected address as verified. It does not open a session, link a provider, add a sign-in method, or change the primary address.
- [ ] #3 E3 - An expired or already used link, or one for an address no longer on the account, does not verify anything. If the address is still eligible, direct the account holder back to their profile.
- [ ] #4 E4 - Opening the link while signed out or signed in to another account does not verify the address or reveal the intended account. The person can sign in to the right AIForAll account and resume if the link is still valid.
- [ ] #5 E5 - From the profile, the signed-in account holder can explicitly request a new link for a saved unverified address. No email is sent automatically just because a link is opened, rejected, or expires.
<!-- AC:END -->
