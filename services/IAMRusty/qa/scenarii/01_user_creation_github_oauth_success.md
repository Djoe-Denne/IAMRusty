# GitHub OAuth User Creation - Success

## What we want to test
Complete GitHub OAuth flow for new user registration with username selection.

## Why
Verify primary user creation via GitHub OAuth works end-to-end, including token generation and username completion.

## How
1. GET `/api/auth/github/login` → extract redirect URL
2. Follow OAuth flow → get callback with code
3. GET `/api/auth/github/callback?code={code}` → extract registration_token
4. POST `/api/auth/complete-registration` with registration_token + username → extract access_token

## Expectation
- 303 redirect on start
- 202 with registration_token on callback
- 200 with access_token on completion
- User has GitHub provider linked 