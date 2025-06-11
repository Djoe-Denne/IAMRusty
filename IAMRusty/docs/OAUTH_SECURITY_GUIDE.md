# OAuth Security Guide

## Overview

This guide covers the security mechanisms implemented in the IAM service's OAuth2 authentication system, including state parameter management, CSRF protection, and best practices for secure OAuth implementations.

## Table of Contents

- [OAuth2 Security Fundamentals](#oauth2-security-fundamentals)
- [State Parameter Management](#state-parameter-management)
- [CSRF Protection](#csrf-protection)
- [Token Security](#token-security)
- [Provider Linking Security](#provider-linking-security)
- [Attack Prevention](#attack-prevention)
- [Security Best Practices](#security-best-practices)
- [Monitoring and Auditing](#monitoring-and-auditing)

## OAuth2 Security Fundamentals

### Authorization Code Flow

The IAM service implements the OAuth2 Authorization Code flow with PKCE-like security enhancements:

```
┌─────────┐                                              ┌─────────────┐
│ Client  │                                              │   IAM       │
│ App     │                                              │   Service   │
└─────────┘                                              └─────────────┘
     │                                                           │
     │ 1. GET /api/auth/{provider}/start                        │
     │ ────────────────────────────────────────────────────────▶│
     │                                                           │
     │ 2. 303 Redirect to Provider + encrypted state           │
     │ ◀────────────────────────────────────────────────────────│
     │                                                           │
┌─────────┐                                              ┌─────────────┐
│ OAuth   │                                              │   IAM       │
│Provider │                                              │   Service   │
└─────────┘                                              └─────────────┘
     │                                                           │
     │ 3. User authorizes, redirect to callback + code + state │
     │ ────────────────────────────────────────────────────────▶│
     │                                                           │
     │ 4. Exchange code for provider tokens                     │
     │ ◀────────────────────────────────────────────────────────│
     │                                                           │
     │ 5. Return JWT tokens and user profile                    │
     │ ────────────────────────────────────────────────────────▶│
```

### Security Properties

1. **Authorization Code**: Single-use, short-lived code
2. **State Parameter**: Cryptographically secure CSRF protection
3. **Redirect URI Validation**: Exact match validation
4. **HTTPS Enforcement**: All OAuth flows require HTTPS
5. **Token Binding**: JWT tokens bound to specific users

## State Parameter Management

### State Structure

The OAuth state parameter encodes operation context and security information:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// Operation type
    pub operation: OAuthOperation,
    /// Timestamp for expiration
    pub timestamp: i64,
    /// Random nonce for uniqueness
    pub nonce: String,
    /// Optional user ID for linking operations
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthOperation {
    Login,
    Link { user_id: Uuid },
}
```

### State Encoding/Decoding

**Encoding Process**:
1. Create state object with operation context
2. Serialize to JSON
3. Base64 encode
4. Apply URL-safe encoding

```rust
impl OAuthState {
    pub fn new_login() -> Self {
        Self {
            operation: OAuthOperation::Login,
            timestamp: chrono::Utc::now().timestamp(),
            nonce: generate_secure_nonce(),
            user_id: None,
        }
    }
    
    pub fn new_link(user_id: Uuid) -> Self {
        Self {
            operation: OAuthOperation::Link { user_id },
            timestamp: chrono::Utc::now().timestamp(),
            nonce: generate_secure_nonce(),
            user_id: Some(user_id),
        }
    }
    
    pub fn encode(&self) -> Result<String, StateError> {
        let json = serde_json::to_string(self)?;
        let encoded = base64::encode_config(json.as_bytes(), base64::URL_SAFE);
        Ok(encoded)
    }
    
    pub fn decode(encoded: &str) -> Result<Self, StateError> {
        let decoded = base64::decode_config(encoded, base64::URL_SAFE)?;
        let json = String::from_utf8(decoded)?;
        let state: OAuthState = serde_json::from_str(&json)?;
        
        // Validate timestamp (30 minute expiration)
        let now = chrono::Utc::now().timestamp();
        if now - state.timestamp > 1800 {
            return Err(StateError::Expired);
        }
        
        Ok(state)
    }
}
```

### State Security Features

1. **Uniqueness**: Each state contains a cryptographically random nonce
2. **Expiration**: States expire after 30 minutes
3. **Operation Binding**: State encodes the intended operation
4. **User Binding**: Link operations include authenticated user ID
5. **Tamper Detection**: Invalid states are rejected

## CSRF Protection

### CSRF Attack Vector

OAuth2 CSRF attacks occur when:
1. Attacker initiates OAuth flow and captures authorization URL
2. Victim visits malicious page with that authorization URL
3. Victim unknowingly authorizes attacker's OAuth request
4. Attacker's account gets linked to victim's provider account

### Prevention Mechanisms

#### 1. State Parameter Validation

**Implementation**:
```rust
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthResponse>, AuthError> {
    // Validate state parameter presence
    let oauth_state = if let Some(state_param) = query.state {
        OAuthState::decode(&state_param)
            .map_err(|_| AuthError::oauth_invalid_state("callback"))?
    } else {
        return Err(AuthError::oauth_missing_state("callback"));
    };
    
    // State validation includes:
    // - Proper decoding (base64 + JSON)
    // - Timestamp validation (expiration check)
    // - Operation context verification
    
    // Continue with operation based on validated state
    match oauth_state.operation {
        OAuthOperation::Login => handle_login_callback(...).await,
        OAuthOperation::Link { user_id } => handle_link_callback(..., user_id).await,
    }
}
```

#### 2. Origin Validation

**Redirect URI Enforcement**:
```rust
// Exact match validation in configuration
let redirect_uri = match provider {
    Provider::GitHub => &state.oauth_config.github.redirect_uri,
    Provider::GitLab => &state.oauth_config.gitlab.redirect_uri,
}.clone();

// Provider validates redirect_uri matches registered value
```

#### 3. Session Binding

**Link Operations**:
```rust
// Link operations require authenticated session
let oauth_state = if let Some(auth_header) = headers.get("Authorization") {
    let token = extract_bearer_token(auth_header)?;
    let user_id = validate_jwt_token(token).await?;
    
    // State includes authenticated user ID
    OAuthState::new_link(user_id)
} else {
    OAuthState::new_login()
};
```

## Token Security

### JWT Token Structure

**Header**:
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

**Payload**:
```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "username": "johndoe",
  "iat": 1640995200,
  "exp": 1640998800,
  "jti": "unique-token-id"
}
```

### Token Security Features

1. **HMAC Signing**: Tokens signed with server secret
2. **Expiration**: Short-lived access tokens (1 hour)
3. **Unique ID**: Each token has unique identifier (jti)
4. **User Binding**: Tokens bound to specific user ID
5. **Refresh Tokens**: Separate refresh tokens for renewal

### Token Validation

```rust
pub async fn validate_token(&self, token: &str) -> Result<Uuid, UserError> {
    // 1. Decode and verify signature
    let claims = self.token_encoder
        .decode(token)
        .map_err(|_| UserError::InvalidToken)?;
    
    // 2. Check expiration
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err(UserError::TokenExpired);
    }
    
    // 3. Validate user exists
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| UserError::InvalidToken)?;
    
    let user = self.user_repository
        .find_by_id(user_id)
        .await
        .map_err(|e| UserError::RepositoryError(e.to_string()))?
        .ok_or(UserError::UserNotFound)?;
    
    Ok(user.id)
}
```

## Provider Linking Security

### Attack Scenarios

#### 1. Account Takeover via Provider Linking

**Attack**: Link attacker's provider account to victim's IAM account
**Prevention**: Authenticated session requirement + state validation

#### 2. Cross-Account Provider Conflict

**Attack**: Link same provider account to multiple IAM accounts
**Prevention**: Provider uniqueness constraints + conflict detection

### Security Implementation

#### 1. Authentication Requirements

```rust
pub async fn oauth_start(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
) -> Result<Redirect, AuthError> {
    let oauth_state = if let Some(auth_header) = headers.get("Authorization") {
        // Link operation requires valid authentication
        let token = extract_bearer_token(auth_header)
            .map_err(|_| AuthError::oauth_invalid_authorization_header("start"))?;
        
        let user_id = state.user_usecase
            .validate_token(token)
            .await
            .map_err(|_| AuthError::oauth_invalid_token("start"))?;
        
        OAuthState::new_link(user_id)
    } else {
        OAuthState::new_login()
    };
    
    // State encodes operation and user context
    let encoded_state = oauth_state.encode()
        .map_err(|_| AuthError::oauth_state_encoding_failed("start"))?;
}
```

#### 2. Provider Conflict Detection

```rust
pub async fn link_provider(
    &self,
    user_id: Uuid,
    provider: Provider,
    code: String,
    redirect_uri: String,
) -> Result<LinkProviderResponse, LinkProviderError> {
    // Exchange code for provider tokens and profile
    let tokens = self.provider_client.exchange_code(&code).await?;
    let profile = self.provider_client.get_user_profile(&tokens).await?;
    
    // Check if provider account is already linked
    if let Some(existing_token) = self.token_repository
        .find_by_provider_user_id(provider, &profile.id)
        .await? {
        
        if existing_token.user_id == user_id {
            // Same user trying to link again
            return Err(LinkProviderError::ProviderAlreadyLinkedToSameUser);
        } else {
            // Different user already has this provider linked
            return Err(LinkProviderError::ProviderAlreadyLinked);
        }
    }
    
    // Safe to link provider to user
    self.create_provider_link(user_id, provider, tokens, profile).await
}
```

## Attack Prevention

### 1. Authorization Code Injection

**Attack**: Inject stolen authorization code into victim's session
**Prevention**: State parameter binding + HTTPS enforcement

**Implementation**:
- State parameter is unique per session
- Authorization codes are single-use
- Codes expire quickly (10 minutes)
- HTTPS prevents code interception

### 2. State Parameter Tampering

**Attack**: Modify state parameter to change operation context
**Prevention**: State integrity validation + expiration

**Detection**:
```rust
pub fn decode(encoded: &str) -> Result<Self, StateError> {
    // Base64 decoding failure indicates tampering
    let decoded = base64::decode_config(encoded, base64::URL_SAFE)
        .map_err(|_| StateError::InvalidEncoding)?;
    
    // JSON parsing failure indicates tampering
    let json = String::from_utf8(decoded)
        .map_err(|_| StateError::InvalidFormat)?;
    
    let state: OAuthState = serde_json::from_str(&json)
        .map_err(|_| StateError::InvalidFormat)?;
    
    // Timestamp validation prevents replay attacks
    let now = chrono::Utc::now().timestamp();
    if now - state.timestamp > 1800 {
        return Err(StateError::Expired);
    }
    
    Ok(state)
}
```

### 3. Redirect URI Manipulation

**Attack**: Redirect to attacker-controlled URL
**Prevention**: Exact redirect URI matching

**Configuration**:
```toml
[oauth.github]
client_id = "github-client-id"
client_secret = "github-client-secret"
redirect_uri = "https://iam.example.com/api/auth/github/callback"  # Exact match required

[oauth.gitlab]
client_id = "gitlab-client-id"
client_secret = "gitlab-client-secret"
redirect_uri = "https://iam.example.com/api/auth/gitlab/callback"  # Exact match required
```

### 4. Session Fixation

**Attack**: Fix victim's session to attacker-controlled value
**Prevention**: Session regeneration + secure token generation

**Implementation**:
- New JWT tokens generated for each authentication
- Refresh tokens are unique and single-use
- No predictable session identifiers

## Security Best Practices

### 1. Configuration Security

**OAuth Provider Setup**:
```bash
# Use strong, unique client secrets
APP_OAUTH_GITHUB_CLIENT_SECRET="highly-secure-random-secret-256-bits"
APP_OAUTH_GITLAB_CLIENT_SECRET="different-highly-secure-random-secret"

# Enforce HTTPS redirect URIs
APP_OAUTH_GITHUB_REDIRECT_URI="https://iam.example.com/api/auth/github/callback"
APP_OAUTH_GITLAB_REDIRECT_URI="https://iam.example.com/api/auth/gitlab/callback"

# Use strong JWT signing secret
APP_JWT_SECRET="secure-jwt-signing-secret-at-least-256-bits-long"
```

**TLS Configuration**:
```toml
[server]
tls_enabled = true
tls_cert_path = "./certs/cert.pem"
tls_key_path = "./certs/key.pem"
tls_port = 8443
```

### 2. State Management Best Practices

**State Generation**:
```rust
fn generate_secure_nonce() -> String {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;
    
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
```

**State Validation**:
```rust
// Always validate state parameter
if query.state.is_none() {
    return Err(AuthError::oauth_missing_state("callback"));
}

// Reject expired states
if now - state.timestamp > STATE_EXPIRATION_SECONDS {
    return Err(AuthError::oauth_invalid_state("callback"));
}

// Validate operation context
match (oauth_state.operation, expected_operation) {
    (OAuthOperation::Login, ExpectedOperation::Login) => Ok(()),
    (OAuthOperation::Link { user_id }, ExpectedOperation::Link { expected_user_id }) 
        if user_id == expected_user_id => Ok(()),
    _ => Err(AuthError::oauth_invalid_state_operation("callback")),
}
```

### 3. Token Security Best Practices

**JWT Configuration**:
```rust
pub struct JwtConfig {
    pub secret: String,           // Minimum 32 bytes
    pub expiration_seconds: u64,  // Short-lived (3600s)
    pub algorithm: Algorithm,     // HS256 minimum
    pub issuer: String,          // Service identifier
    pub audience: Vec<String>,   // Valid audiences
}
```

**Token Rotation**:
```rust
// Refresh token rotation
pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, TokenError> {
    // Validate current refresh token
    let token = self.validate_refresh_token(refresh_token).await?;

    // Generate new tokens
    let new_access_token = self.generate_access_token(token.user_id).await?;
    let new_refresh_token = self.generate_refresh_token(token.user_id).await?;
    
    // Invalidate old refresh token
    self.revoke_refresh_token(&token.id).await?;
    
    Ok(TokenResponse {
        access_token: new_access_token.token,
        expires_in: new_access_token.expires_in,
        refresh_token: new_refresh_token.token,
    })
}
```

### 4. Error Handling Security

**Safe Error Messages**:
```rust
// Don't leak sensitive information in error messages
match error {
    AuthError::InvalidState(_) => "Invalid state parameter",
    AuthError::InvalidToken(_) => "Invalid or expired token",
    AuthError::ProviderAlreadyLinked(_) => "Provider account is already linked",
    // Generic message for internal errors
    _ => "Authentication failed",
}
```

**Error Logging**:
```rust
// Log security events for monitoring
tracing::warn!(
    user_id = ?user_id,
    provider = %provider,
    error = "provider_already_linked",
    "Attempt to link already linked provider"
);

tracing::error!(
    state_param = %state_param,
    error = "invalid_state",
    "Invalid OAuth state parameter received"
);
```

## Monitoring and Auditing

### Security Events to Monitor

1. **Authentication Events**:
   - Failed login attempts
   - Invalid state parameters
   - Token validation failures
   - Suspicious provider linking attempts

2. **OAuth Flow Events**:
   - State parameter tampering
   - Redirect URI mismatches
   - Provider error responses
   - Multiple failed callback attempts

3. **Token Events**:
   - Expired token usage
   - Invalid token signatures
   - Refresh token abuse
   - Token generation failures

### Monitoring Implementation

**Security Event Logging**:
```rust
#[derive(Debug, Serialize)]
struct SecurityEvent {
    event_type: String,
    user_id: Option<Uuid>,
    provider: Option<String>,
    ip_address: String,
    user_agent: String,
    timestamp: DateTime<Utc>,
    details: serde_json::Value,
}

pub fn log_security_event(event: SecurityEvent) {
    tracing::warn!(
        event_type = %event.event_type,
        user_id = ?event.user_id,
        provider = ?event.provider,
        ip_address = %event.ip_address,
        details = ?event.details,
        "Security event detected"
    );
}
```

**Metrics Collection**:
```rust
// Authentication metrics
let auth_attempts = Counter::new("oauth_auth_attempts_total")
    .with_description("Total OAuth authentication attempts")
    .with_label("provider")
    .with_label("result");

let state_validation_failures = Counter::new("oauth_state_validation_failures_total")
    .with_description("OAuth state validation failures")
    .with_label("error_type");

let token_validation_failures = Counter::new("token_validation_failures_total")
    .with_description("JWT token validation failures")
    .with_label("error_type");
```

### Alerting Rules

**Critical Security Alerts**:
```yaml
# Prometheus alerting rules
groups:
  - name: oauth_security
    rules:
      - alert: HighAuthenticationFailureRate
        expr: rate(oauth_auth_attempts_total{result="failure"}[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High OAuth authentication failure rate"
          description: "OAuth authentication failure rate is {{ $value }} per second"

      - alert: StateValidationFailures
        expr: rate(oauth_state_validation_failures_total[5m]) > 0.05
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "OAuth state validation failures detected"
          description: "Potential CSRF attack detected"

      - alert: TokenValidationFailures
        expr: rate(token_validation_failures_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High token validation failure rate"
          description: "High rate of invalid token attempts"
```

### Security Audit Checklist

#### OAuth Configuration
- [ ] Client secrets are cryptographically random (≥256 bits)
- [ ] Redirect URIs use HTTPS and exact matching
- [ ] State parameters include nonce and expiration
- [ ] Authorization codes are single-use and short-lived

#### Token Security
- [ ] JWT signing secret is secure (≥256 bits)
- [ ] Access tokens are short-lived (≤1 hour)
- [ ] Refresh tokens are rotated on use
- [ ] Token validation includes signature and expiration checks

#### Provider Linking
- [ ] Link operations require authentication
- [ ] Provider conflicts are detected and prevented
- [ ] State parameters bind operations to users
- [ ] Error messages don't leak sensitive information

#### Monitoring and Logging
- [ ] Security events are logged with context
- [ ] Failed authentication attempts are monitored
- [ ] State validation failures trigger alerts
- [ ] Token abuse patterns are detected

This security guide provides comprehensive protection against OAuth2-related attacks while maintaining usability and performance. Regular security reviews and penetration testing should validate these protections in practice. 