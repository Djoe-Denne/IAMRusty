---
id: STORY-002
title: Sign back in with email and password
status: Accepted intent
assignee: []
created_date: '2026-11-21T15:12:49Z'
updated_date: '2026-11-21T16:07:49Z'
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

This is approved product intent v1 for the complete journey. Basic password sign-in and incorrect-credentials rejection are existing backend behavior. The protections below are requirements, not claims about what the backend already does. The interface, including messages and post-sign-in navigation, is part of the story but deferred from delivery. No delivery tasks are scheduled here; keep backend and interface work separate if either is later requested.

STORY-001 covers incomplete registration and unconfirmed email addresses. Password recovery, adding another sign-in method, and session renewal are separate stories.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for the existing API behavior. The referenced QA scenarios are evidence candidates, not additional product rules.

## Sign-in protections

The successful sign-in examples assume that password sign-in is available and that the attempt is allowed under the ordinary interval rule.

- Require a short minimum interval between password attempts, measured in seconds. The length and scope of the interval remain open.
- Below the failure ceiling, failures from another connection alone must not delay or prevent a valid sign-in from the account holder's own connection. Failed attempts never lock the whole account. Below the ceiling, failed attempts alone do not force a password change.
- Count consecutive failures against password sign-in for the same account across sources. At the ceiling, disable password sign-in only. Recovery and other previously configured and independently valid sign-in methods remain available. Restore password sign-in through verified recovery that binds a password to the account again, not by waiting. The ceiling remains open within the [NIST SP 800-63B-4 bound](https://pages.nist.gov/800-63-4/sp800-63b/authenticators/#throttle).
- Email the account holder about repeated failed sign-ins, not an isolated failure. Count failed sign-ins separately from password-reset requests. The alert threshold, time window, and repeat cadence remain open.

These decisions stay in force when backend behavior differs. The NIST guidance above and [OWASP ASVS 5.0.0 authentication requirements](https://github.com/OWASP/ASVS/blob/v5.0.0/5.0/en/0x15-V6-Authentication.md) guide verification; this story makes no claim of compliance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Given a completed account with a confirmed address, when the person selects Sign in from an AIForAll page such as a public project and enters the correct email and password, they sign in and return to that same page.
- [ ] #2 E2 - Given a completed account with a confirmed address and no originating page, when the person opens sign-in directly and enters the correct email and password, they sign in and land on the AIForAll home page.
- [ ] #3 E3 - Given an unknown email or an incorrect password, when the person tries to sign in, no session is created. The interface reports that the email address or password is incorrect without disclosing which one.
<!-- AC:END -->
