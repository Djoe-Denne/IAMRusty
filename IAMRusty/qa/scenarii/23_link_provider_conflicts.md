# Provider Linking Conflicts

## What we want to test
Conflict scenarios when linking providers already associated with other users.

## Why
Ensure provider linking prevents account takeover and maintains data integrity.

## How
1. Create User A with GitHub OAuth
2. Create User B with email/password
3. User B tries to link same GitHub account → expect 409
4. Test linking provider already linked to same user → expect 409
5. Verify error messages are informative

## Expectation
- 409 with "provider_already_linked" for cross-user conflicts
- 409 with "provider_already_linked_to_same_user" for self-conflicts
- Clear error messages explaining the conflict
- No data corruption or account merge 