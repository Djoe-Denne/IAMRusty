# Provider Relinking - Success

## What we want to test
Relinking OAuth providers to update credentials for existing linked provider.

## Why
Verify users can update provider credentials when tokens expire or are revoked.

## How
1. Create user with GitHub OAuth → verify provider linked
2. GET `/api/auth/github/relink-start` → extract auth URL
3. Follow OAuth flow → get new authorization code
4. GET `/api/auth/github/relink-callback?code={code}` with Authorization header
5. Verify provider token updated

## Expectation
- 200 with auth_url on relink start
- 200 with updated user profile on callback
- New provider token replaces old one
- User maintains same identity 