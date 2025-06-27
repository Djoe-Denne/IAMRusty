# Add Password to OAuth User

## What we want to test
Adding email/password authentication to user who signed up via OAuth.

## Why
Verify users can add password authentication as backup to OAuth providers.

## How
1. Create user via GitHub OAuth flow → extract access_token
2. User tries POST `/api/auth/signup` with their email → expect 409 (already exists)
3. User must add password via password reset flow:
   - POST `/api/auth/password/reset-request` with email
   - POST `/api/auth/password/reset-confirm` with token + new password

## Expectation
- 409 on signup attempt (email exists without password)
- 200 on reset request (silent success)
- 200 on reset confirm with tokens
- User can subsequently login with email/password 