# Input Validation Guide

This guide covers the input validation implementation using the `axum_valid` crate in the IAM system's HTTP layer.

## Table of Contents

- [Overview](#overview)
- [Dependencies](#dependencies)
- [Validation Architecture](#validation-architecture)
- [Basic Usage](#basic-usage)
- [Validation Patterns](#validation-patterns)
- [Custom Validators](#custom-validators)
- [Error Handling](#error-handling)
- [Testing Validation](#testing-validation)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

The IAM system uses `axum_valid` with the `validator` crate to provide robust input validation for all HTTP endpoints. This ensures that:

- Invalid data is rejected before processing
- Consistent error responses are returned
- Security vulnerabilities from malformed input are prevented
- API contracts are enforced at the handler level

### Key Features

- **Declarative Validation**: Use derive macros and attributes to define validation rules
- **Type Safety**: Validation happens at compile time and runtime
- **Consistent Error Responses**: Returns 422 Unprocessable Entity with JSON error details
- **Custom Validators**: Support for domain-specific validation logic
- **Performance**: Minimal overhead with early validation

## Dependencies

The following dependencies are used for input validation:

```toml
[dependencies]
# Validation
axum-valid = { version = "0.23", features = ["validator", "422", "into_json"] }
validator = { version = "0.20", features = ["derive"] }
regex = "1.10"
lazy_static = "1.4"
```

### Feature Flags Explained

- `validator`: Enables the `Valid<E>` extractor using the validator crate
- `422`: Returns HTTP 422 instead of 400 for validation errors
- `into_json`: Serializes validation errors into JSON format

## Validation Architecture

The validation system is organized into several layers:

```
┌─────────────────────────────────────────────────────────────────┐
│                        HTTP Request                             │
└─────────────────────┬───────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────────┐
│                 axum_valid::Valid<E>                            │
│   - Extracts and validates input                               │
│   - Returns 422 on validation failure                          │
└─────────────────────┬───────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────────┐
│                 Validation Rules                               │
│   - Length constraints                                          │
│   - Format validation (regex)                                  │
│   - Custom business logic                                      │
└─────────────────────┬───────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────────┐
│                 Handler Logic                                  │
│   - Processes validated input                                  │
│   - No additional validation needed                            │
└─────────────────────────────────────────────────────────────────┘
```

## Basic Usage

### 1. Define a Validated Struct

```rust
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 100, message = "Username must be 1-100 characters"))]
    #[validate(regex(path = "USERNAME_REGEX", message = "Invalid username format"))]
    pub username: String,
    
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}
```

### 2. Use in Handler

```rust
use axum::{Json, extract::State};

pub async fn create_user(
    State(state): State<AppState>,
    Valid(Json(request)): Valid<Json<CreateUserRequest>>,
) -> Result<Json<UserResponse>, ApiError> {
    // Input is guaranteed to be valid at this point
    let user = state.user_service.create_user(request).await?;
    Ok(Json(user.into()))
}
```

## Validation Patterns

### Length Validation

```rust
#[derive(Validate, Deserialize)]
pub struct Example {
    #[validate(length(min = 1, max = 50))]
    pub short_string: String,
    
    #[validate(length(min = 10, max = 1000))]
    pub long_string: String,
    
    #[validate(length(equal = 36))] // For UUIDs
    pub uuid_string: String,
}
```

### Regex Validation

```rust
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    static ref PROVIDER_REGEX: Regex = Regex::new(r"^[a-z]+$").unwrap();
}

#[derive(Validate, Deserialize)]
pub struct Example {
    #[validate(regex(path = "*USERNAME_REGEX"))]
    pub username: String,
    
    #[validate(regex(path = "*PROVIDER_REGEX"))]
    pub provider: String,
}
```

### Range Validation

```rust
#[derive(Validate, Deserialize)]
pub struct PaginationRequest {
    #[validate(range(min = 1, max = 100))]
    pub page_size: usize,
    
    #[validate(range(min = 1))]
    pub page_number: usize,
}
```

### Email Validation

```rust
#[derive(Validate, Deserialize)]
pub struct ContactForm {
    #[validate(email)]
    pub email: String,
    
    #[validate(email)]
    pub backup_email: Option<String>, // Optional emails are supported
}
```

### URL Validation

```rust
#[derive(Validate, Deserialize)]
pub struct CallbackRequest {
    #[validate(url)]
    pub callback_url: String,
    
    #[validate(url)]
    pub webhook_url: Option<String>,
}
```

## Custom Validators

### Creating Custom Validation Functions

```rust
use validator::ValidationError;

pub fn validate_provider_name(provider: &str) -> Result<(), ValidationError> {
    let valid_providers = ["github", "gitlab"];
    
    if !valid_providers.contains(&provider.to_lowercase().as_str()) {
        return Err(ValidationError::new("invalid_provider"));
    }
    
    Ok(())
}

pub fn validate_refresh_token(token: &str) -> Result<(), ValidationError> {
    if token.trim().is_empty() {
        return Err(ValidationError::new("empty_refresh_token"));
    }
    
    if token.len() < 10 || token.len() > 1000 {
        return Err(ValidationError::new("invalid_refresh_token_length"));
    }
    
    Ok(())
}
```

### Using Custom Validators

```rust
#[derive(Validate, Deserialize)]
pub struct OAuthRequest {
    #[validate(custom(function = "validate_provider_name"))]
    pub provider: String,
    
    #[validate(custom(function = "validate_refresh_token"))]
    pub refresh_token: String,
}
```

### Validation with Context

For more complex validation that requires external data:

```rust
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub allowed_providers: Vec<String>,
    pub max_token_length: usize,
}

impl FromRef<AppState> for ValidationContext {
    fn from_ref(state: &AppState) -> Self {
        Self {
            allowed_providers: state.config.oauth.allowed_providers.clone(),
            max_token_length: state.config.security.max_token_length,
        }
    }
}

fn validate_with_context(value: &str, context: &ValidationContext) -> Result<(), ValidationError> {
    if !context.allowed_providers.contains(&value.to_lowercase()) {
        return Err(ValidationError::new("provider_not_allowed"));
    }
    Ok(())
}
```

## Error Handling

### Default Error Response

When validation fails, `axum_valid` automatically returns a 422 response with JSON error details:

```json
{
  "errors": [
    {
      "field": "username",
      "code": "length",
      "message": "Username must be 1-100 characters",
      "params": {
        "min": 1,
        "max": 100,
        "value": ""
      }
    },
    {
      "field": "email",
      "code": "email",
      "message": "Invalid email format"
    }
  ]
}
```

### Custom Error Messages

```rust
#[derive(Validate, Deserialize)]
pub struct LoginRequest {
    #[validate(
        length(min = 1, message = "Username cannot be empty"),
        length(max = 100, message = "Username is too long")
    )]
    pub username: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}
```

### Handling Validation Errors in Tests

```rust
#[tokio::test]
async fn test_invalid_input_returns_422() {
    let app = create_test_app().await;
    
    let invalid_request = json!({
        "username": "", // Invalid: empty
        "email": "not-an-email", // Invalid: not email format
        "password": "short" // Invalid: too short
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/users")
                .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(invalid_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let error_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(error_response["errors"].is_array());
    assert_eq!(error_response["errors"].as_array().unwrap().len(), 3);
}
```

## Testing Validation

### Unit Testing Validation Rules

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_valid_request() {
        let request = CreateUserRequest {
            username: "valid_user".to_string(),
            email: "user@example.com".to_string(),
            password: "securepassword".to_string(),
        };
        
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_invalid_username_too_long() {
        let request = CreateUserRequest {
            username: "a".repeat(101), // Too long
            email: "user@example.com".to_string(),
            password: "securepassword".to_string(),
        };
        
        let result = request.validate();
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_invalid_email_format() {
        let request = CreateUserRequest {
            username: "valid_user".to_string(),
            email: "not-an-email".to_string(),
            password: "securepassword".to_string(),
        };
        
        let result = request.validate();
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }
}
```

### Integration Testing with axum_test

```rust
use axum_test::TestServer;

#[tokio::test]
async fn test_validation_integration() {
    let app = create_app().await;
    let server = TestServer::new(app).unwrap();
    
    // Test valid request
    let response = server
        .post("/api/users")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "securepassword"
        }))
        .await;
    
    response.assert_status(StatusCode::CREATED);
    
    // Test invalid request
    let response = server
        .post("/api/users")
        .json(&json!({
            "username": "",
            "email": "invalid-email",
            "password": "short"
        }))
        .await;
    
    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    
    let errors: serde_json::Value = response.json();
    assert!(errors["errors"].is_array());
}
```

## Best Practices

### 1. Use Meaningful Error Messages

```rust
// Good
#[validate(length(min = 8, message = "Password must be at least 8 characters for security"))]
pub password: String,

// Bad
#[validate(length(min = 8))]
pub password: String,
```

### 2. Validate at the Boundary

Always validate input as early as possible in your handlers:

```rust
// Good
pub async fn handler(
    Valid(Json(request)): Valid<Json<MyRequest>>,
) -> Result<Json<Response>, ApiError> {
    // Input is already validated
}

// Bad
pub async fn handler(
    Json(request): Json<MyRequest>,
) -> Result<Json<Response>, ApiError> {
    // Manual validation needed
    if request.field.is_empty() {
        return Err(ApiError::BadRequest("Field is required".to_string()));
    }
}
```

### 3. Centralize Validation Rules

Keep validation utilities in a dedicated module:

```rust
// src/validation.rs
pub mod validation {
    use lazy_static::lazy_static;
    use regex::Regex;
    
    lazy_static! {
        pub static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
        pub static ref PROVIDER_REGEX: Regex = Regex::new(r"^[a-z]+$").unwrap();
    }
    
    pub fn validate_username(username: &str) -> Result<(), ValidationError> {
        // Centralized username validation logic
    }
}
```

### 4. Use Type-Safe Extractors

Combine validation with other extractors:

```rust
pub async fn oauth_callback(
    State(state): State<AppState>,
    Valid(Path(provider_path)): Valid<Path<ProviderPath>>,
    Valid(Query(query)): Valid<Query<OAuthCallbackQuery>>,
) -> Result<Json<OAuthResponse>, AuthError> {
    // Both path and query parameters are validated
}
```

### 5. Document Validation Rules

Use clear documentation for your validation rules:

```rust
/// OAuth provider path parameter
#[derive(Debug, Deserialize, Validate)]
pub struct ProviderPath {
    /// Provider name (github, gitlab, etc.)
    /// Must be lowercase letters only and match supported providers
    #[validate(length(min = 1, max = 50, message = "Provider name must be between 1 and 50 characters"))]
    #[validate(regex(path = "*PROVIDER_REGEX", message = "Provider name can only contain lowercase letters"))]
    #[validate(custom(function = "validate_provider_name", message = "Invalid provider name"))]
    pub provider_name: String,
}
```

## Examples

### Complete OAuth Handler Example

```rust
// handlers/auth.rs
use axum::{Json, extract::{State, Path, Query}};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ProviderPath {
    #[validate(length(min = 1, max = 50))]
    #[validate(regex(path = "*PROVIDER_REGEX"))]
    #[validate(custom(function = "validate_provider_name"))]
    pub provider_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct OAuthCallbackQuery {
    #[validate(length(max = 1000))]
    pub code: Option<String>,
    
    #[validate(length(max = 2000))]
    pub state: Option<String>,
    
    #[validate(length(max = 500))]
    pub error: Option<String>,
    
    #[validate(length(max = 1000))]
    pub error_description: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Valid(Path(provider_path)): Valid<Path<ProviderPath>>,
    Valid(Query(query)): Valid<Query<OAuthCallbackQuery>>,
) -> Result<Json<OAuthResponse>, AuthError> {
    // All input is validated at this point
    let provider = parse_provider(&provider_path.provider_name)?;
    
    if let Some(error) = query.error {
        return Err(AuthError::oauth_provider_error(error));
    }
    
    let code = query.code.ok_or(AuthError::missing_code())?;
    
    // Process OAuth callback...
    Ok(Json(response))
}
```

### Token Refresh Example

```rust
// handlers/token.rs
use axum::{Json, extract::State};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    #[validate(custom(function = "validate_refresh_token", message = "Invalid refresh token format"))]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Valid(Json(request)): Valid<Json<RefreshTokenRequest>>,
) -> Result<Json<TokenResponse>, ApiError> {
    let response = state
        .token_service
        .refresh(request.refresh_token)
        .await?;
    
    Ok(Json(TokenResponse {
        token: response.access_token,
        expires_in: response.expires_in,
    }))
}
```

## Troubleshooting

### Common Issues

#### 1. Regex Validation Not Working

**Problem**: `AsRegex` trait not implemented
```rust
error[E0277]: the trait bound `PROVIDER_REGEX: AsRegex` is not satisfied
```

**Solution**: Dereference the regex in the validation attribute:
```rust
// Wrong
#[validate(regex(path = "PROVIDER_REGEX"))]

// Correct
#[validate(regex(path = "*PROVIDER_REGEX"))]
```

#### 2. Custom Validator Not Found

**Problem**: Function not in scope
```rust
error[E0425]: cannot find function `validate_provider_name` in this scope
```

**Solution**: Import the validation module:
```rust
use crate::validation::*;
```

#### 3. Validation Not Triggering

**Problem**: Using regular extractor instead of `Valid`
```rust
// Wrong - no validation
Json(request): Json<MyRequest>

// Correct - with validation
Valid(Json(request)): Valid<Json<MyRequest>>
```

#### 4. Multiple Validation Errors on Same Field

**Problem**: Want to apply multiple validation rules
```rust
// Correct way to chain validations
#[validate(
    length(min = 1, message = "Cannot be empty"),
    length(max = 100, message = "Too long"),
    regex(path = "*USERNAME_REGEX", message = "Invalid format")
)]
pub username: String,
```

### Debug Mode

To see detailed validation information during development:

```rust
// In your test or development code
let validation_result = request.validate();
match validation_result {
    Ok(_) => println!("Validation passed"),
    Err(errors) => {
        for (field, field_errors) in errors.field_errors() {
            for error in field_errors {
                println!("Field '{}': {} (code: {:?})", field, error.message.as_ref().unwrap_or(&"Unknown error".to_string()), error.code);
            }
        }
    }
}
```

### Performance Considerations

1. **Regex Compilation**: Use `lazy_static!` to compile regexes once:
```rust
lazy_static! {
    static ref EXPENSIVE_REGEX: Regex = Regex::new(r"complex pattern").unwrap();
}
```

2. **Custom Validator Caching**: Cache expensive validation operations:
```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;

static VALIDATION_CACHE: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn validate_with_cache(value: &str) -> Result<(), ValidationError> {
    let mut cache = VALIDATION_CACHE.lock().unwrap();
    
    if let Some(&is_valid) = cache.get(value) {
        return if is_valid { Ok(()) } else { Err(ValidationError::new("cached_invalid")) };
    }
    
    // Expensive validation logic here
    let is_valid = expensive_validation(value);
    cache.insert(value.to_string(), is_valid);
    
    if is_valid { Ok(()) } else { Err(ValidationError::new("validation_failed")) }
}
```

## Related Documentation

- [Error Handling Guide](ERROR_HANDLING_GUIDE.md) - How validation errors integrate with the overall error handling system
- [API Reference](API_REFERENCE.md) - Complete API documentation including validation rules
- [Testing Guide](TESTING_GUIDE.md) - Comprehensive testing strategies including validation testing
- [Architecture](ARCHITECTURE.md) - How validation fits into the overall system architecture

---

This guide provides comprehensive coverage of input validation in the IAM system. For additional examples and patterns, refer to the source code in the `http/src/handlers` directory and the test files in `http/src/validation.rs`. 