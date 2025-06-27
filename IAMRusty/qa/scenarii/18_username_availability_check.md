# Username Availability Check

## What we want to test
Username availability checking and suggestion generation.

## Why
Verify users can check username availability before registration.

## How
1. GET `/api/auth/username/check?username=available_name` → expect available:true
2. Create user with username → complete registration
3. GET `/api/auth/username/check?username=taken_name` → expect available:false + suggestions
4. Test edge cases (too short, invalid characters)

## Expectation
- 200 with available:true for free usernames
- 200 with available:false + suggestions for taken ones
- 400/422 for invalid username formats
- Helpful alternative suggestions provided 