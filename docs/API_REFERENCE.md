# API Reference

## Overview

The IAM Service API provides OAuth2 authentication, JWT token management, and user profile access. This reference documents all endpoints, request/response formats, authentication requirements, and error codes.

**Base URL**: `https://iam.example.com`  
**API Version**: 1.3.0  
**Protocol**: HTTP/HTTPS  
**Data Format**: JSON  
**Authentication**: Bearer tokens (JWT)  

## Table of Contents

- [Authentication](#authentication)
- [Endpoints](#endpoints)
  - [OAuth Authentication](#oauth-authentication)
  - [Token Management](#token-management)
  - [User Management](#user-management)
  - [Utility Endpoints](#utility-endpoints)
- [Data Models](#data-models)
- [Error Codes](#error-codes)
- [Rate Limiting](#rate-limiting)
- [Examples](#examples)

## Authentication

### Bearer Token Authentication

Most endpoints require authentication via JWT tokens in the Authorization header:

```http
Authorization: Bearer <jwt_token>
```

### Token Lifecycle

1. **Obtain Token**: Use OAuth2 flow via `/api/auth/{provider}/start` and `/api/auth/{provider}/callback`
2. **Use Token**: Include in Authorization header for protected endpoints
3. **Refresh Token**: Use `/api/token/refresh` before expiration
4. **Token Expiry**: Default 3600 seconds (1 hour)

## Endpoints

### OAuth Authentication

#### Start OAuth Flow

```http
GET /api/auth/{provider}/start
```

Initiates OAuth2 authentication flow. Supports both login and provider linking operations.

**Path Parameters:**
- `provider` (string, required): OAuth2 provider (`github`, `gitlab`)

**Headers:**
- `Authorization` (string, optional): Bearer token for provider linking

**Operation Types:**
- **Login**: No Authorization header → Creates/authenticates user
- **Link Provider**: With Authorization header → Links provider to existing user

**Response:**
- **Status**: `303 See Other`
- **Headers**: `Location: <provider_oauth_url>`

**Error Responses:**
- `400 Bad Request`: Invalid provider
- `401 Unauthorized`: Invalid token (for linking)

**Example:**
```bash
# Login flow
curl "https://iam.example.com/api/auth/github/start"

# Provider linking flow
curl -H "Authorization: Bearer eyJ..." \
     "https://iam.example.com/api/auth/github/start"
```

#### OAuth Callback

```http
GET /api/auth/{provider}/callback
```

Handles OAuth2 provider callback and processes authorization code.

**Path Parameters:**
- `provider` (string, required): OAuth2 provider (`github`, `gitlab`)

**Query Parameters:**
- `code` (string, required): Authorization code from provider
- `state` (string, required): OAuth state parameter
- `error` (string, optional): Error code from provider
- `error_description` (string, optional): Error description from provider

**Response Format:**

**Login Success (200)**:
```json
{
  "operation": "login",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.github.com/u/123456"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600,
  "refresh_token": "def502004f8c7..."
}
```

**Link Provider Success (200)**:
```json
{
  "operation": "link",
  "message": "GitHub successfully linked",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.github.com/u/123456"
  },
  "emails": [
    {
      "id": "email-uuid-1",
      "email": "john@example.com",
      "is_primary": true,
      "is_verified": true
    },
    {
      "id": "email-uuid-2",
      "email": "john.github@example.com",
      "is_primary": false,
      "is_verified": false
    }
  ],
  "new_email_added": true,
  "new_email": "john.github@example.com"
}
```

**Error Responses:**
- `400 Bad Request`: Invalid parameters, missing code/state
- `401 Unauthorized`: Authentication failed
- `409 Conflict`: Provider already linked

### Token Management

#### Refresh Access Token

```http
POST /api/token/refresh
```

Exchanges a valid refresh token for a new JWT access token.

**Request Body:**
```json
{
  "refresh_token": "def502004f8c7..."
}
```

**Response (200)**:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600
}
```

**Error Responses:**
- `400 Bad Request`: Malformed request body
- `401 Unauthorized`: Invalid/expired refresh token

**Example:**
```bash
curl -X POST "https://iam.example.com/api/token/refresh" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "def502004f8c7..."}'
```

### User Management

#### Get Current User

```http
GET /api/me
```

Returns the authenticated user's profile information.

**Headers:**
- `Authorization` (required): Bearer JWT token

**Response (200)**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "johndoe",
  "email": "john@example.com",
  "avatar_url": "https://avatars.github.com/u/123456"
}
```

**Fields:**
- `id`: Unique user identifier (UUID)
- `username`: User's display name
- `email`: Primary email address
- `avatar_url`: Profile picture URL (nullable)

**Error Responses:**
- `401 Unauthorized`: Invalid/expired token
- `404 Not Found`: User not found

**Example:**
```bash
curl -H "Authorization: Bearer eyJ..." \
     "https://iam.example.com/api/me"
```

### Utility Endpoints

#### Health Check

```http
GET /health
```

Service health status endpoint.

**Response (200)**:
```
OK
```

#### JSON Web Key Set (JWKS)

```http
GET /.well-known/jwks.json
```

Public keys for JWT token verification.

**Response (200)**:
```json
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "abc123",
      "use": "sig",
      "alg": "RS256",
      "n": "...base64_modulus...",
      "e": "AQAB"
    }
  ]
}
```

#### Get Provider Access Token (Internal)

```http
POST /internal/{provider}/token
```

Returns OAuth2 access token for authenticated user and provider. **Internal use only**.

**Path Parameters:**
- `provider` (string, required): OAuth2 provider (`github`, `gitlab`)

**Headers:**
- `Authorization` (required): Bearer JWT token

**Response (200)**:
```json
{
  "access_token": "gho_xyz...",
  "expires_in": 3600
}
```

**Error Responses:**
- `400 Bad Request`: Unsupported provider
- `401 Unauthorized`: Invalid token
- `404 Not Found`: No token for user/provider

## Data Models

### User Object

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "johndoe",
  "email": "john@example.com",
  "avatar_url": "https://avatars.github.com/u/123456"
}
```

**Fields:**
- `id` (string): UUID identifier
- `username` (string): Display name
- `email` (string|null): Primary email address
- `avatar_url` (string|null): Profile picture URL

### Email Object

```json
{
  "id": "email-uuid",
  "email": "john@example.com",
  "is_primary": true,
  "is_verified": true
}
```

**Fields:**
- `id` (string): UUID identifier
- `email` (string): Email address
- `is_primary` (boolean): Whether this is the primary email
- `is_verified` (boolean): Email verification status

### Token Response

```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600,
  "refresh_token": "def502004f8c7..."
}
```

**Fields:**
- `access_token` (string): JWT access token
- `expires_in` (number): Token expiration in seconds
- `refresh_token` (string): Refresh token for renewal

## Error Codes

### Standard Error Response

```json
{
  "error": {
    "message": "Error description",
    "status": 400
  }
}
```

### OAuth Error Response

```json
{
  "operation": "login",
  "error": "error_code",
  "message": "Error description"
}
```

### HTTP Status Codes

| Code | Meaning | When Used |
|------|---------|-----------|
| 200 | OK | Successful request |
| 303 | See Other | OAuth redirect |
| 400 | Bad Request | Invalid parameters, validation errors |
| 401 | Unauthorized | Authentication required/failed |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict (e.g., provider already linked) |
| 500 | Internal Server Error | Server-side errors |

### OAuth Error Codes

#### Start Operation Errors
| Error Code | Description | HTTP Status |
|------------|-------------|-------------|
| `invalid_provider` | Unsupported OAuth provider | 400 |
| `invalid_authorization_header` | Malformed Authorization header | 400 |
| `invalid_token` | Invalid/expired JWT token | 401 |
| `state_encoding_failed` | Failed to create OAuth state | 500 |
| `url_generation_failed` | Failed to generate authorization URL | 500 |

#### Callback Operation Errors
| Error Code | Description | HTTP Status |
|------------|-------------|-------------|
| `missing_code` | Missing authorization code | 400 |
| `invalid_state` | Invalid/missing state parameter | 400 |
| `authentication_failed` | OAuth authentication failed | 401 |
| `validation_failed` | Request validation failed | 400 |

#### Provider Linking Errors
| Error Code | Description | HTTP Status |
|------------|-------------|-------------|
| `provider_already_linked_to_same_user` | Provider already linked to user | 409 |
| `provider_already_linked` | Provider linked to different user | 409 |
| `user_not_found` | User account not found | 404 |
| `link_failed` | Provider linking failed | 500 |

### General Error Codes

| Error Type | Description | HTTP Status |
|------------|-------------|-------------|
| Validation Error | Invalid request parameters | 400 |
| Authentication Error | Missing/invalid credentials | 401 |
| Authorization Error | Insufficient permissions | 403 |
| Not Found | Resource not found | 404 |
| Conflict | Resource conflict | 409 |
| Internal Error | Server-side error | 500 |

## Rate Limiting

The API implements rate limiting to prevent abuse:

**Limits:**
- **General Endpoints**: 100 requests per minute per IP
- **OAuth Endpoints**: 10 requests per minute per IP
- **Token Refresh**: 5 requests per minute per user

**Headers:**
- `X-RateLimit-Limit`: Requests allowed per window
- `X-RateLimit-Remaining`: Requests remaining in window
- `X-RateLimit-Reset`: Window reset time (Unix timestamp)

**Rate Limit Exceeded (429)**:
```json
{
  "error": {
    "message": "Rate limit exceeded",
    "status": 429
  }
}
```

## Examples

### Complete OAuth Login Flow

**1. Start OAuth Flow**
```bash
curl -i "https://iam.example.com/api/auth/github/start"
```

**Response:**
```http
HTTP/1.1 303 See Other
Location: https://github.com/login/oauth/authorize?client_id=...&state=...
```

**2. User Authorizes (Browser)**
User visits GitHub, authorizes application, gets redirected to callback URL.

**3. Handle Callback**
```bash
curl "https://iam.example.com/api/auth/github/callback?code=abc123&state=xyz789"
```

**Response:**
```json
{
  "operation": "login",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.github.com/u/123456"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600,
  "refresh_token": "def502004f8c7..."
}
```

### Using Access Token

**Get User Profile**
```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
     "https://iam.example.com/api/me"
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "johndoe",
  "email": "john@example.com",
  "avatar_url": "https://avatars.github.com/u/123456"
}
```

### Token Refresh

**Refresh Expired Token**
```bash
curl -X POST "https://iam.example.com/api/token/refresh" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "def502004f8c7..."}'
```

**Response:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600
}
```

### Provider Linking Flow

**1. Start Linking (Authenticated)**
```bash
curl -H "Authorization: Bearer eyJ..." \
     "https://iam.example.com/api/auth/gitlab/start"
```

**2. Complete Linking**
```bash
curl "https://iam.example.com/api/auth/gitlab/callback?code=def456&state=abc123"
```

**Response:**
```json
{
  "operation": "link",
  "message": "GitLab successfully linked",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.github.com/u/123456"
  },
  "emails": [
    {
      "id": "email-uuid-1",
      "email": "john@example.com",
      "is_primary": true,
      "is_verified": true
    },
    {
      "id": "email-uuid-2",
      "email": "john.gitlab@example.com",
      "is_primary": false,
      "is_verified": false
    }
  ],
  "new_email_added": true,
  "new_email": "john.gitlab@example.com"
}
```

### Error Handling Examples

**Invalid Provider**
```bash
curl "https://iam.example.com/api/auth/invalid/start"
```

**Response (400)**:
```json
{
  "operation": "start",
  "error": "invalid_provider",
  "message": "Invalid provider"
}
```

**Expired Token**
```bash
curl -H "Authorization: Bearer expired_token" \
     "https://iam.example.com/api/me"
```

**Response (401)**:
```json
{
  "error": {
    "message": "Invalid or expired token",
    "status": 401
  }
}
```

**Provider Already Linked**
```bash
curl "https://iam.example.com/api/auth/github/callback?code=abc&state=xyz"
```

**Response (409)**:
```json
{
  "operation": "link",
  "error": "provider_already_linked",
  "message": "This GitHub account is already linked to another user"
}
```

## SDKs and Libraries

### JavaScript/Node.js

```javascript
const iam = new IAMClient({
  baseURL: 'https://iam.example.com',
  clientId: 'your-client-id'
});

// Start OAuth flow
const authUrl = await iam.getAuthURL('github');
window.location.href = authUrl;

// Handle callback
const tokens = await iam.handleCallback(code, state);

// Use access token
const user = await iam.getUser(tokens.access_token);
```

### Python

```python
import requests

class IAMClient:
    def __init__(self, base_url, client_id):
        self.base_url = base_url
        self.client_id = client_id
    
    def get_auth_url(self, provider):
        response = requests.get(f"{self.base_url}/api/auth/{provider}/start")
        return response.headers['Location']
    
    def get_user(self, access_token):
        headers = {'Authorization': f'Bearer {access_token}'}
        response = requests.get(f"{self.base_url}/api/me", headers=headers)
        return response.json()
```

### Go

```go
package iam

import (
    "encoding/json"
    "fmt"
    "net/http"
)

type Client struct {
    BaseURL  string
    ClientID string
}

func (c *Client) GetUser(accessToken string) (*User, error) {
    req, _ := http.NewRequest("GET", c.BaseURL+"/api/me", nil)
    req.Header.Set("Authorization", "Bearer "+accessToken)
    
    resp, err := http.DefaultClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    var user User
    json.NewDecoder(resp.Body).Decode(&user)
    return &user, nil
}
```

This API reference provides comprehensive documentation for integrating with the IAM service, covering all endpoints, data formats, error handling, and practical examples for common use cases. 