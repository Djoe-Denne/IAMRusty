# Link GitLab to GitHub-Primary User

## What we want to test
Linking GitLab provider to existing user who signed up with GitHub.

## Why
Verify multi-provider account linking works correctly and maintains user identity.

## How
1. Create user via GitHub OAuth flow → extract access_token
2. GET `/api/auth/gitlab/login` with Authorization: Bearer {access_token} → extract redirect URL
3. Follow GitLab OAuth → get callback with code
4. GET `/api/auth/gitlab/callback?code={code}&state={state}` → expect link success

## Expectation
- 303 redirect on authenticated start
- 200 with operation:"link" on callback
- User has both GitHub and GitLab providers
- New email added if different 