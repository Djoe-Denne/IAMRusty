# Link GitHub to GitLab-Primary User

## What we want to test
Linking GitHub provider to existing user who signed up with GitLab.

## Why
Verify bidirectional provider linking works regardless of primary provider.

## How
1. Create user via GitLab OAuth flow → extract access_token
2. GET `/api/auth/github/login` with Authorization: Bearer {access_token} → extract redirect URL
3. Follow GitHub OAuth → get callback with code
4. GET `/api/auth/github/callback?code={code}&state={state}` → expect link success

## Expectation
- 303 redirect on authenticated start
- 200 with operation:"link" on callback
- User has both GitLab and GitHub providers
- New email added if different 