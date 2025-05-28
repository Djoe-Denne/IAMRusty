# Error Handling Guide

## Overview

The IAM service implements a comprehensive error handling system using `tower_http::error_handling` and centralized error types. This system provides consistent error responses, proper HTTP status code mapping, and robust panic recovery while maintaining API contracts.

## Table of Contents

- [Architecture](#architecture)
- [Error Types](#error-types)
- [HTTP Error Mapping](#http-error-mapping)
- [OAuth Error Handling](#oauth-error-handling)
- [Panic Recovery](#panic-recovery)
- [Error Response Formats](#error-response-formats)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Architecture

The error handling system follows a layered approach with centralized error management:

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Layer                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   ApiError  │  │ AuthError   │  │ tower_http Panic    │  │
│  │   (General) │  │ (OAuth)     │  │     Recovery        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │ IntoResponse
┌─────────────────────▼───────────────────────────────────────┐
│                Command/Application Layer                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ CommandError│  │UserError   │  │    TokenError       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │ From implementations
┌─────────────────────▼───────────────────────────────────────┐
│                   Domain Layer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │DomainError │  │ Repository   │  │  Business Logic     │  │
│  │            │  │   Errors     │  │     Errors          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Error Types

### Core Error Hierarchies

#### 1. Domain Errors (`domain/src/error.rs`)

**Purpose**: Business logic and domain rule violations

```rust
#[derive(Debug, Error)]
pub enum DomainError {
    /// User not found
    #[error("User not found")]
    UserNotFound,
    
    /// Provider not supported
    #[error("Provider not supported: {0}")]
    ProviderNotSupported(String),
    
    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,
    
    /// Token expired
    #[error("Token expired")]
    TokenExpired,
    
    /// Authorization error
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    /// OAuth2 error
    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),
    
    /// User profile error
    #[error("Failed to get user profile: {0}")]
    UserProfileError(String),
    
    /// No token found for provider and user
    #[error("No token found for provider {0} and user {1}")]
    NoTokenForProvider(String, String),
    
    /// Token generation failed
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),
    
    /// Token validation failed
    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),
    
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(String),
}
```

#### 2. Application Errors (`application/src/error.rs`)

**Purpose**: Application layer orchestration errors

```rust
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] DomainError),
    
    /// Repository error
    #[error("Repository error: {0}")]
    Repository(String),
    
    /// Service error
    #[error("Service error: {0}")]
    Service(String),
    
    /// OAuth2 error
    #[error("OAuth2 error: {0}")]
    OAuth2(String),
    
    /// Token error
    #[error("Token error: {0}")]
    Token(String),
    
    /// User profile error
    #[error("User profile error: {0}")]
    UserProfile(String),
}
```

#### 3. Command Errors (`application/src/command/mod.rs`)

**Purpose**: Command pattern execution errors

```rust
#[derive(Debug, Error)]
pub enum CommandError {
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Business logic error
    #[error("Business error: {0}")]
    Business(String),
    
    /// Infrastructure error (database, external services, etc.)
    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
    
    /// Timeout error
    #[error("Command execution timeout")]
    Timeout,
    
    /// Retry exhausted error
    #[error("Maximum retries exhausted: {0}")]
    RetryExhausted(String),
}
```

### HTTP Layer Error Types

#### 1. ApiError (`http/src/error.rs`)

**Purpose**: General API errors with automatic conversion from lower layers

```rust
#[derive(Debug, Error)]
pub enum ApiError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] DomainError),
    
    /// Application error
    #[error(transparent)]
    Application(#[from] ApplicationError),
    
    /// Command error
    #[error(transparent)]
    Command(#[from] CommandError),
    
    /// User use case error
    #[error(transparent)]
    User(#[from] UserError),
    
    /// Token use case error
    #[error(transparent)]
    Token(#[from] TokenError),
    
    /// Authentication required
    #[error("Authentication required")]
    AuthenticationRequired,
    
    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    /// Internal server error
    #[error("Internal server error: {0}")]
    InternalServerError(String),
}
```

#### 2. AuthError (`http/src/error.rs`)

**Purpose**: OAuth-specific errors with context-aware responses

```rust
#[derive(Debug, Error)]
pub enum AuthError {
    /// OAuth error with specific response format
    #[error("{message}")]
    OAuth {
        operation: String,
        error_code: String,
        message: String,
        status: StatusCode,
    },
    
    /// General API error
    #[error(transparent)]
    Api(#[from] ApiError),
}
```

## HTTP Error Mapping

### Status Code Mapping

The error mapping follows REST conventions and OAuth2 specifications:

```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            // Domain errors
            ApiError::Domain(e) => match e {
                DomainError::UserNotFound => (StatusCode::NOT_FOUND, e.to_string()),
                DomainError::ProviderNotSupported(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                DomainError::InvalidToken => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::TokenExpired => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::AuthorizationError(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::OAuth2Error(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                DomainError::UserProfileError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                DomainError::TokenGenerationFailed(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                DomainError::TokenValidationFailed(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::RepositoryError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            
            // Command errors
            ApiError::Command(e) => match e {
                CommandError::Business(msg) if msg.contains("Authentication failed") => {
                    (StatusCode::UNAUTHORIZED, "Authentication failed".to_string())
                }
                CommandError::Business(msg) if msg.contains("User not found") => {
                    (StatusCode::NOT_FOUND, "User not found".to_string())
                }
                CommandError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            
            // Use case specific errors
            ApiError::User(e) => match e {
                UserError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user".to_string()),
            },
            
            ApiError::Token(e) => match e {
                TokenError::TokenNotFound => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenInvalid => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenExpired => (StatusCode::UNAUTHORIZED, "Expired refresh token".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to refresh token".to_string()),
            },
            
            // Generic errors
            ApiError::AuthenticationRequired => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "status": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}
```

### Error Response Format

**Standard API Error Response**:
```json
{
  "error": {
    "message": "User not found",
    "status": 404
  }
}
```

**OAuth Error Response** (OAuth endpoints):
```json
{
  "operation": "login",
  "error": "authentication_failed",
  "message": "Authentication failed"
}
```

## OAuth Error Handling

### Context-Aware Error Methods

OAuth errors include operation context to distinguish between start and callback operations:

```rust
impl AuthError {
    pub fn oauth_invalid_provider(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_provider".to_string(),
            message: "Invalid provider".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
    
    pub fn oauth_login_failed(operation: &str, command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Business(msg) if msg.contains("Authentication failed") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Validation(msg) => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "validation_failed".to_string(),
                    message: msg.clone(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            _ => Self::OAuth {
                operation: operation.to_string(),
                error_code: "login_failed".to_string(),
                message: "Login failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
```

### OAuth Error Scenarios

**Start Operation Errors**:
- `invalid_provider`: Unsupported OAuth provider
- `invalid_authorization_header`: Malformed Authorization header (linking)
- `invalid_token`: Invalid JWT token (linking)
- `state_encoding_failed`: Failed to create OAuth state
- `url_generation_failed`: Failed to generate authorization URL

**Callback Operation Errors**:
- `missing_code`: Missing authorization code parameter
- `invalid_state`: Invalid or missing state parameter
- `authentication_failed`: OAuth authentication failed
- `validation_failed`: Request validation failed

**Provider Linking Conflicts**:
- `provider_already_linked_to_same_user`: Provider already linked to user
- `provider_already_linked`: Provider linked to different user

## Panic Recovery

### tower_http Integration

The HTTP layer uses `CatchPanicLayer` for comprehensive panic recovery:

```rust
pub async fn serve(state: AppState, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/auth/{provider}/start", get(oauth_start))
        .route("/api/auth/{provider}/callback", get(oauth_callback))
        .route("/api/token/refresh", post(refresh_token))
        .route(
            "/api/me",
            get(get_user).route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware)),
        )
        .layer(CatchPanicLayer::custom(handle_panic))  // Panic recovery
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Custom Panic Handler

```rust
fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> axum::response::Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    tracing::error!("Service panicked: {}", details);

    let body = Json(json!({
        "error": {
            "message": "Internal server error",
            "status": 500,
        }
    }));

    (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
}
```

### Benefits

1. **Graceful Degradation**: Panics don't crash the server
2. **Consistent Response Format**: Even panics return JSON errors
3. **Logging**: All panics are logged for debugging
4. **Security**: No panic details leaked to clients

## Error Response Formats

### Standard Responses

#### Success Response Example
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

#### Error Response Examples

**Authentication Error (401)**:
```json
{
  "operation": "login",
  "error": "authentication_failed",
  "message": "Authentication failed"
}
```

**Validation Error (400)**:
```json
{
  "operation": "start",
  "error": "invalid_provider",
  "message": "Invalid provider"
}
```

**Conflict Error (409)**:
```json
{
  "operation": "link",
  "error": "provider_already_linked",
  "message": "This GitHub account is already linked to another user"
}
```

**General API Error (404)**:
```json
{
  "error": {
    "message": "User not found",
    "status": 404
  }
}
```

**Internal Server Error (500)**:
```json
{
  "error": {
    "message": "Internal server error",
    "status": 500
  }
}
```

## Best Practices

### 1. Error Handling in Handlers

```rust
pub async fn oauth_start(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
) -> Result<Redirect, AuthError> {
    // Use context-aware error methods
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err(AuthError::oauth_invalid_provider("start")),
    };
    
    // Chain errors with ? operator
    let encoded_state = oauth_state.encode()
        .map_err(|_| AuthError::oauth_state_encoding_failed("start"))?;
    
    // Use command service with error conversion
    let base_auth_url = state.command_service
        .generate_login_start_url(provider, context)
        .await
        .map_err(|_| AuthError::oauth_url_generation_failed("start"))?;
    
    Ok(Redirect::to(url.as_str()))
}
```

### 2. Error Conversion

**From Lower Layers**:
```rust
// Domain -> Application
impl From<DomainError> for ApplicationError {
    fn from(err: DomainError) -> Self {
        ApplicationError::Domain(err)
    }
}

// Application -> HTTP
impl From<ApplicationError> for ApiError {
    fn from(err: ApplicationError) -> Self {
        ApiError::Application(err)
    }
}
```

**Use Case Specific**:
```rust
// Use case errors to command errors
LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg))
LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e))
```

### 3. Logging

```rust
// Error logging with context
error!("OAuth error from provider: {} - {}", error, description);

// Debug logging for troubleshooting
debug!("Creating link state for user: {}", user_id);

// Structured logging
tracing::error!(
    user_id = %user_id,
    provider = %provider,
    error = %e,
    "Failed to link provider"
);
```

### 4. Testing Error Scenarios

```rust
#[tokio::test]
async fn test_invalid_provider_returns_400() {
    let response = client
        .get(&format!("{}/api/auth/invalid/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 400);
    
    let error_response: OAuthErrorResponse = response
        .json()
        .await
        .expect("Failed to parse error response");
    
    assert_eq!(error_response.operation, "start");
    assert_eq!(error_response.error, "invalid_provider");
}
```

## Troubleshooting

### Common Error Scenarios

#### 1. OAuth State Errors

**Problem**: `invalid_state` errors in OAuth callback

**Possible Causes**:
- State parameter tampering
- URL encoding/decoding issues
- State expiration (if implemented)

**Investigation**:
```bash
# Check logs for state creation and validation
grep "oauth_state" logs/app.log

# Look for encoding/decoding errors
grep "StateEncodingFailed\|InvalidState" logs/app.log
```

**Resolution**:
- Verify OAuth state encoding/decoding implementation
- Check for URL encoding issues in redirects
- Ensure state parameter is preserved during redirects

#### 2. Provider Authentication Failures

**Problem**: `authentication_failed` errors

**Possible Causes**:
- Invalid OAuth credentials
- Network connectivity issues
- Provider API changes

**Investigation**:
```bash
# Check provider-specific logs
grep "provider.*github\|provider.*gitlab" logs/app.log

# Look for network/HTTP errors
grep "OAuth2Error\|UserProfileError" logs/app.log
```

**Resolution**:
- Verify OAuth client credentials
- Check provider status and API documentation
- Test network connectivity to provider APIs

#### 3. Token Validation Errors

**Problem**: `invalid_token` or `token_expired` errors

**Possible Causes**:
- JWT secret mismatch
- Clock skew
- Token format issues

**Investigation**:
```bash
# Check token-related errors
grep "TokenValidationFailed\|InvalidToken" logs/app.log

# Look for JWT-specific issues
grep "jwt" logs/app.log -i
```

**Resolution**:
- Verify JWT secret configuration
- Check system clock synchronization
- Validate token generation and validation logic

#### 4. Database Connection Issues

**Problem**: `RepositoryError` errors

**Possible Causes**:
- Database connectivity issues
- Connection pool exhaustion
- SQL errors

**Investigation**:
```bash
# Check database-related errors
grep "RepositoryError\|Database error" logs/app.log

# Look for connection issues
grep "connection\|pool" logs/app.log -i
```

**Resolution**:
- Verify database connectivity
- Check connection pool configuration
- Monitor database performance

### Error Monitoring

#### Log Analysis Queries

**Error Rate by Type**:
```bash
# Count errors by type
grep "ERROR" logs/app.log | grep -o '"error_type":"[^"]*"' | sort | uniq -c
```

**OAuth Errors by Provider**:
```bash
# OAuth errors by provider
grep "OAuth.*error" logs/app.log | grep -o '"provider":"[^"]*"' | sort | uniq -c
```

**Response Time Analysis**:
```bash
# Find slow operations
grep "duration_ms" logs/app.log | awk -F'"duration_ms":' '{print $2}' | awk -F',' '{print $1}' | sort -n
```

#### Metrics to Monitor

1. **Error Rates**:
   - Overall error rate
   - Error rate by endpoint
   - Error rate by operation type

2. **Error Types**:
   - Authentication failures
   - Validation errors
   - Infrastructure errors
   - Provider-specific errors

3. **Response Times**:
   - 95th percentile response time
   - Error response time vs success
   - Command execution duration

4. **Security Events**:
   - Invalid token attempts
   - State parameter tampering
   - Suspicious authentication patterns

### Performance Impact

#### Error Handling Overhead

The centralized error handling system has minimal performance impact:

1. **No Allocations**: Error variants use static strings where possible
2. **Fast Path**: Success cases bypass error handling entirely
3. **Lazy Logging**: Error details only computed when errors occur
4. **Efficient Conversion**: `From` implementations use zero-cost abstractions

#### Monitoring Recommendations

1. **Response Time**: Monitor P95 response times
2. **Error Rates**: Track error rates by endpoint and operation
3. **Panic Recovery**: Monitor panic frequency (should be zero)
4. **Resource Usage**: Monitor memory and CPU during error conditions

This error handling system provides robust, consistent error management while maintaining high performance and excellent observability for debugging and monitoring production systems. 