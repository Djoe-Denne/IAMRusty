# Password Reset (Unauthenticated) - Success

## What we want to test
Complete password reset flow for users who forgot their password.

## Why
Verify users can recover access via email-based password reset.

## How
1. POST `/api/auth/password/reset-request` with email → generic success
2. POST `/api/auth/password/reset-validate` with token → verify validity
3. POST `/api/auth/password/reset-confirm` with token + new password → extract tokens
4. Login with new password → verify success

## Expectation
- 200 generic message on request (anti-enumeration)
- 200 with masked email on validation
- 200 with tokens on confirmation
- Old refresh tokens invalidated 