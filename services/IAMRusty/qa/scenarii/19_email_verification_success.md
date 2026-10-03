# Email Verification - Success

## What we want to test
Email verification flow during user registration.

## Why
Verify email verification is working correctly to confirm user ownership.

## How
1. POST `/api/auth/signup` with email/password → email verification sent
2. POST `/api/auth/verify` with email + verification_token → confirm email
3. Continue with registration completion
4. Test resend verification functionality

## Expectation
- 200 on successful email verification
- Verification token consumed after use
- Email marked as verified in system
- Resend functionality works with token invalidation 