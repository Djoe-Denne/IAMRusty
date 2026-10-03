# Password Reset (Authenticated) - Success

## What we want to test
Password change flow for authenticated users who know their current password.

## Why
Verify users can securely change passwords while logged in.

## How
1. Login user → extract access_token
2. POST `/api/auth/password/reset-authenticated` with Authorization header, current + new password
3. Extract new tokens from response
4. Verify old refresh tokens invalidated
5. Login with new password

## Expectation
- 200 with new access_token and refresh_token
- Current password verification required
- All existing refresh tokens invalidated
- New password works for subsequent logins 