# Provider Token Revocation

## What we want to test
Revoking user's OAuth provider tokens via internal endpoint.

## Why
Verify users can revoke provider access and tokens are properly deleted.

## How
1. Create user with GitHub OAuth → verify token exists
2. DELETE `/internal/github/revoke` with Authorization: Bearer {access_token}
3. Verify 200 success message
4. POST `/internal/github/token` → expect 404 (token removed)
5. Test revoking non-existent token → expect 404

## Expectation
- 200 success on first revocation
- 404 on subsequent revocations (idempotent)
- Provider token completely removed
- User still exists but provider unlinked 