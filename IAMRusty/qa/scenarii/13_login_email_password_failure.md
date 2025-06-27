# Email/Password Login - Failure

## What we want to test
Email/password login failure scenarios (wrong credentials, incomplete registration).

## Why
Ensure proper authentication security and error handling.

## How
1. POST `/api/auth/login` with wrong password → expect 401
2. POST `/api/auth/login` with non-existent email → expect 401
3. POST `/api/auth/login` for incomplete user → expect 423
4. POST `/api/auth/login` with malformed data → expect 422

## Expectation
- 401 for wrong credentials (no user enumeration)
- 423 for incomplete registration (with registration_token)
- 422 for validation errors
- No access tokens issued on failures 