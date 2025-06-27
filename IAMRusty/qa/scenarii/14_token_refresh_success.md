# Token Refresh - Success

## What we want to test
Token refresh flow with valid refresh tokens and rotation mechanism.

## Why
Verify refresh token rotation works and users can maintain sessions.

## How
1. Login user → extract access_token + refresh_token
2. POST `/api/token/refresh` with refresh_token → extract new tokens
3. Verify old refresh_token is invalidated
4. Use new access_token to access protected endpoint

## Expectation
- 200 with new access_token and refresh_token
- Old refresh_token becomes invalid
- New tokens have extended expiration
- Token rotation works correctly 