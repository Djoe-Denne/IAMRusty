# User Profile Management

## What we want to test
User profile retrieval and email management across multiple providers.

## Why
Verify user profile data consistency across different authentication methods.

## How
1. Create user with GitHub OAuth → GET `/api/me` → verify profile
2. Link GitLab with different email → GET `/api/me` → check email handling
3. Add password auth → verify primary email remains correct
4. Test avatar URL updates from different providers

## Expectation
- Consistent user profile structure
- Primary email correctly identified
- Avatar URLs from latest provider
- Email list properly managed 