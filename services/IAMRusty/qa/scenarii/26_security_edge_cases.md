# Security Edge Cases

## What we want to test
Security vulnerabilities and edge cases (CSRF, enumeration, injection).

## Why
Ensure the API is secure against common attacks and provides proper error handling.

## How
1. Test user enumeration via timing attacks on login/signup
2. Test OAuth state parameter validation and CSRF protection
3. Test SQL injection in username/email fields
4. Test malformed JSON and oversized payloads
5. Test rate limiting on sensitive endpoints

## Expectation
- Consistent response times (no user enumeration)
- Proper state validation prevents CSRF
- Input sanitization prevents injection
- Graceful handling of malformed requests
- Rate limiting protects against brute force 