# OAuth Login - Existing User

## What we want to test
Login flow for existing users via OAuth providers.

## Why
Verify users can login with their existing OAuth providers without re-registration.

## How
1. Create user via GitHub OAuth → complete registration
2. Later: GET `/api/auth/github/login` (no Authorization header) → start login
3. Follow OAuth flow → get callback with code
4. GET `/api/auth/github/callback?code={code}` → expect direct login

## Expectation
- 303 redirect on start
- 200 with operation:"login" on callback
- Direct access_token + refresh_token response
- No registration flow triggered 