# Email/Password User Creation - Success

## What we want to test
Complete email/password signup flow with email verification and username completion.

## Why
Verify email/password primary user creation works with all required steps.

## How
1. POST `/api/auth/signup` with email/password → extract registration_token
2. POST `/api/auth/verify` with email + verification_token → confirm email
3. POST `/api/auth/complete-registration` with registration_token + username → extract access_token
4. Verify user can login with email/password

## Expectation
- 201 with registration_token on signup
- 200 on email verification
- 200 with access_token on completion
- Subsequent login works with email/password 