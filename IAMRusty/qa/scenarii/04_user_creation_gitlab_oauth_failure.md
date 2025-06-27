# GitLab OAuth User Creation - Failure

## What we want to test
GitLab OAuth flow failure scenarios (invalid code, missing state, network errors).

## Why
Ensure robust error handling for GitLab OAuth provider communication failures.

## How
1. GET `/api/auth/gitlab/login` → extract redirect URL
2. GET `/api/auth/gitlab/callback?code=invalid_code` → expect error
3. GET `/api/auth/gitlab/callback` (no code) → expect error
4. Test with malformed state parameter

## Expectation
- 400/401/500 errors with descriptive messages
- No user creation on failures
- No tokens generated
- Graceful error handling 