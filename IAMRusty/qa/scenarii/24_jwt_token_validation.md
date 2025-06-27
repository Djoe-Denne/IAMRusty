# JWT Token Validation

## What we want to test
JWT token format, signatures, and validation via JWKS endpoint.

## Why
Verify JWT tokens are properly signed and can be validated by external services.

## How
1. Login user → extract access_token (JWT)
2. GET `/.well-known/jwks.json` → extract public keys
3. Validate JWT signature using public key
4. Test with expired token → expect 401
5. Test with malformed token → expect 401

## Expectation
- Valid JWT structure (header.payload.signature)
- RSA signature validates with public key
- Expired tokens rejected with 401
- Malformed tokens rejected with proper errors 