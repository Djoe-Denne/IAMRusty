# Complete Multi-Provider Workflow

## What we want to test
End-to-end workflow combining all authentication methods and provider management.

## Why
Verify all authentication methods work together seamlessly in realistic user scenarios.

## How
1. Create user with GitHub OAuth → complete registration
2. Link GitLab provider → verify multiple emails
3. Add password authentication → test all login methods
4. Reset password → verify all methods still work
5. Revoke GitHub → test remaining authentications
6. Relink GitHub → verify full functionality restored

## Expectation
- All authentication methods work independently
- Provider linking/unlinking maintains data integrity
- Password operations don't affect OAuth providers
- User maintains consistent identity throughout 