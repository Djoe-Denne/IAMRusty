# Email/Password User Creation - Failure

## What we want to test
Email/password signup failure scenarios (weak password, duplicate email, invalid format).

## Why
Ensure proper validation and error handling for email/password registration.

## How
1. POST `/api/auth/signup` with weak password → expect 422
2. POST `/api/auth/signup` with invalid email format → expect 422
3. POST `/api/auth/signup` with existing email → expect 409
4. POST `/api/auth/verify` with invalid token → expect 400

## Expectation
- 422 for validation errors
- 409 for email conflicts
- 400 for invalid verification tokens
- Descriptive error messages 