# Email/Password Login - Success

## What we want to test
Standard email/password login flow for existing registered users.

## Why
Verify completed users can authenticate using email/password credentials.

## How
1. Create and complete user registration → store email/password
2. POST `/api/auth/login` with correct email + password → extract tokens
3. GET `/api/me` with access_token → verify user data
4. Test case sensitivity and trimming

## Expectation
- 200 with access_token and refresh_token
- User object includes id, username, email
- Token allows API access
- Email matching is case-insensitive 