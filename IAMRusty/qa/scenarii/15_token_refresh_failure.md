# Token Refresh - Failure

## What we want to test
Token refresh failure scenarios (invalid, expired, already used tokens).

## Why
Ensure token security and proper error handling for refresh failures.

## How
1. POST `/api/token/refresh` with invalid token → expect 401
2. POST `/api/token/refresh` with expired token → expect 401
3. Use refresh token twice → first succeeds, second fails
4. POST `/api/token/refresh` with malformed request → expect 400/422

## Expectation
- 401 for invalid/expired tokens
- 400/422 for malformed requests
- Clear error messages
- No new tokens issued on failures 