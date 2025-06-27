# Add OAuth to Password User

## What we want to test
Linking OAuth providers to user who signed up with email/password.

## Why
Verify email/password users can add OAuth providers for convenience.

## How
1. Create user via email/password signup → extract access_token
2. GET `/api/auth/github/login` with Authorization: Bearer {access_token} → start linking
3. Follow GitHub OAuth flow → get callback
4. GET `/api/auth/github/callback` → expect successful linking

## Expectation
- User created with email/password
- GitHub provider successfully linked
- User can login via either method
- Email addresses managed correctly 