# Provider Token Retrieval

## What we want to test
Internal endpoint for retrieving user's OAuth provider tokens.

## Why
Verify internal services can get provider tokens for authenticated users.

## How
1. Create user with GitHub OAuth → extract access_token
2. POST `/internal/github/token` with Authorization: Bearer {access_token}
3. Verify GitHub access token returned
4. Test with user who has no GitHub token → expect 404

## Expectation
- 200 with provider access_token for linked providers
- 404 for non-linked providers
- 401 for unauthenticated requests
- 422 for unsupported providers 