# Testing Guide

Comprehensive guide for understanding, extending, and implementing tests in the IAM service.

## Table of Contents

- [Overview](#overview)
- [Technologies & Libraries](#technologies--libraries)
- [Test Architecture](#test-architecture)
- [Mocking & Fixtures](#mocking--fixtures)
- [Running Tests](#running-tests)
- [Writing New Tests](#writing-new-tests)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Quick Reference](#quick-reference)

## Overview

The IAM service uses a comprehensive integration testing approach that validates OAuth authentication flows end-to-end. Tests are designed to be:

- **Fast**: Shared database containers, efficient cleanup
- **Reliable**: Isolated test environments, comprehensive mocking
- **CI/CD Optimized**: Automatic environment detection and optimization
- **Developer Friendly**: Rich fixtures, clear assertions, modern tooling

### Test Coverage Status

#### 🔁 /auth/{provider}/start
✅ Redirects to provider with proper OAuth parameters  
✅ Handles both login and link operations based on authentication state  
✅ Stores state & purpose securely in session or signed parameter  
✅ Returns 400 for unsupported providers  
✅ Case-insensitive provider names  

#### 🔁 /auth/{provider}/callback
✅ Handles successful OAuth2 flow and creates JWT for known user  
✅ Links external account if state correspond to a link and user is authenticated  
✅ Associates new provider if same user logs in via another provider  
✅ Detects and prevents linking a provider already bound to another user  
✅ Fails on invalid or expired code  
✅ Returns 400 on missing or invalid state/purpose  
✅ Returns 401 if provider refuses or rejects user  

**All OAuth callback error scenarios are now fully implemented and tested!**

## Technologies & Libraries

### Core Testing Framework

| Library | Version | Purpose |
|---------|---------|---------|
| `tokio` | Latest | Async runtime for test execution |
| `tokio-test` | Latest | Async testing utilities |

### HTTP Testing

| Library | Version | Purpose |
|---------|---------|---------|
| `axum-test` | 17.3.0 | HTTP testing framework for Axum applications |
| `reqwest` | Latest | HTTP client for external API testing |
| `serde_json` | Latest | JSON serialization/deserialization |

### Database Testing

| Library | Version | Purpose |
|---------|---------|---------|
| `testcontainers` | 0.23.1 | Containerized PostgreSQL for testing |
| `testcontainers-modules` | 0.11.0 | Pre-built container modules |
| `sea-orm` | Latest | Database ORM for test data management |
| `sqlx` | Latest | Direct SQL operations for cleanup |

### Mocking & Simulation

| Library | Version | Purpose |
|---------|---------|---------|
| `wiremock` | 0.6.3 | HTTP service mocking for OAuth providers |
| `uuid` | Latest | Test data generation |
| `url` | Latest | URL parsing and validation |

### Task Management

| Library | Version | Purpose |
|---------|---------|---------|
| `just` | Latest | Modern task runner (recommended) |
| `cargo-make` | Latest | Alternative Rust-native task runner |

## Test Architecture

### Directory Structure

```
tests/
├── integration_auth_oauth_flow.rs  # Main OAuth integration tests
├── test_config.rs                  # Configuration testing utilities
├── README.md                       # Test-specific documentation
└── common/                         # Shared test utilities
    ├── mod.rs                      # Database container setup
    └── fixtures.rs                 # Test fixtures and mocking
```

### Test Lifecycle

```rust
#[tokio::test]
async fn test_oauth_feature() {
    // 1. Setup - Create clean test environment
    let fixture = TestFixture::new().await;
    
    // 2. Arrange - Setup mocks and test data
    fixture.mock_provider.setup_github_success().await;
    
    // 3. Act - Execute the test scenario
    let response = fixture.server.get("/auth/github/start").await;
    
    // 4. Assert - Verify expected behavior
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    // 5. Cleanup - Automatic via TestFixture::drop
}
```

### Shared Database Strategy

```rust
// Single database container shared across all tests
static DATABASE: OnceCell<Arc<DatabaseContainer>> = OnceCell::const_new();

// Efficient cleanup: TRUNCATE tables (not container recreation)
impl DatabaseContainer {
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Fast table cleanup instead of container restart
        sqlx::query("TRUNCATE TABLE users, oauth_accounts, user_emails RESTART IDENTITY CASCADE")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

## Mocking & Fixtures

### OAuth Provider Mocking

#### GitHub OAuth Flow

```rust
// Setup complete GitHub OAuth simulation
let mock_provider = MockOAuthProvider::new().await;
let auth_code = mock_provider.setup_github_success().await;

// This creates wiremock endpoints for:
// 1. Token exchange: POST /login/oauth/access_token
// 2. User info: GET /user  
// 3. User emails: GET /user/emails
```

#### GitLab OAuth Flow

```rust
// Setup complete GitLab OAuth simulation
let auth_code = mock_provider.setup_gitlab_success().await;

// This creates wiremock endpoints for:
// 1. Token exchange: POST /oauth/token
// 2. User info: GET /api/v4/user
```

#### Error Scenarios

```rust
// Simulate OAuth provider errors
mock_provider.setup_oauth_error("access_denied", "User denied access").await;

// Test various error conditions:
// - access_denied
// - invalid_request  
// - invalid_client
// - unauthorized_client
```

### Test Fixtures

#### OAuth State Management

```rust
// Login operation state
let login_state = OAuthStateFixtures::valid_login_state();

// Provider linking state (authenticated users)
let user_id = UserFixtures::test_user_id();
let link_state = OAuthStateFixtures::valid_link_state(user_id);

// Security testing
let tampered_state = OAuthStateFixtures::tampered_state();
let invalid_state = OAuthStateFixtures::invalid_state();
```

#### User Data

```rust
// Consistent test user ID
let user_id = UserFixtures::test_user_id(); // UUID

// Mock JWT token for authenticated requests
let jwt_token = UserFixtures::test_jwt_token();
let (header_name, header_value) = TestRequestBuilder::auth_header(&jwt_token);
```

#### Request Building

```rust
// OAuth callback simulation
let callback_params = TestRequestBuilder::oauth_callback_query("auth_code", Some("state"));

// Error callback simulation  
let error_params = TestRequestBuilder::oauth_error_query("access_denied", Some("User denied"));

// Authenticated requests
let response = fixture
    .server
    .get("/auth/github/start")
    .add_header(header_name, header_value)
    .await;
```

### Response Assertions

#### Standard Assertions

```rust
// Status code validation
response.assert_status(StatusCode::TEMPORARY_REDIRECT);
response.assert_status_ok();

// JSON response validation
let body: Value = response.json();
ResponseAssertions::assert_oauth_success_response(&body);
ResponseAssertions::assert_oauth_error_response(&body, "Invalid provider");
```

#### Redirect Validation

```rust
// Validate OAuth redirect URLs
let location = response.header("location").to_str().unwrap();
ResponseAssertions::assert_redirect_has_params(
    location,
    &["client_id", "redirect_uri", "state", "scope"]
);

// Validate state parameter integrity
ResponseAssertions::assert_valid_state(&state_param);
```

## Running Tests

### Using Modern Task Runner (Recommended)

```bash
# Install just (modern task runner)
cargo install just

# Run all OAuth integration tests
just test-integration

# Run specific test with debugging
just test-single test_oauth_start_github_redirects_properly

# Watch mode - auto-run on changes
just watch

# Debug mode with full logging
just test-debug
```

### Direct Cargo Commands

```bash
# Run all integration tests
cargo test --test integration_auth_oauth_flow

# Run specific test pattern
cargo test --test integration_auth_oauth_flow oauth_start

# Debug with full logging
RUST_LOG=debug cargo test --test integration_auth_oauth_flow -- --nocapture
```

### Environment Configuration

```bash
# CI/CD optimization
CI=true TEST_VERBOSE=true TEST_MAX_CONCURRENCY=2 cargo test

# Custom timeouts for slower systems
TEST_DB_TIMEOUT=60 TEST_DB_RETRIES=50 cargo test

# Docker configuration
TEST_USE_DOCKER=true cargo test
```

## Writing New Tests

### 1. Basic Test Template

```rust
#[tokio::test]
async fn test_your_new_feature() {
    // Setup clean test environment
    let fixture = TestFixture::new().await;
    
    // Arrange: Setup mocks and test data
    let auth_code = fixture.mock_provider.setup_github_success().await;
    let state = OAuthStateFixtures::valid_login_state();
    
    // Act: Execute the test scenario
    let response = fixture
        .server
        .get("/auth/github/your-endpoint")
        .add_query_params(&[
            ("code", &auth_code),
            ("state", &state)
        ])
        .await;
    
    // Assert: Verify expected behavior
    response.assert_status_ok();
    let body: Value = response.json();
    
    // Custom assertions
    assert_eq!(body["operation"], "your_operation");
    assert!(body["data"].is_object());
}
```

### 2. Testing Authenticated Endpoints

```rust
#[tokio::test]
async fn test_authenticated_feature() {
    let fixture = TestFixture::new().await;
    
    // Create authenticated request
    let token = UserFixtures::test_jwt_token();
    let (header_name, header_value) = TestRequestBuilder::auth_header(&token);
    
    let response = fixture
        .server
        .post("/your-authenticated-endpoint")
        .add_header(header_name, header_value)
        .json(&json!({ "data": "test" }))
        .await;
    
    response.assert_status_ok();
}
```

### 3. Testing Error Scenarios

```rust
#[tokio::test]
async fn test_error_handling() {
    let fixture = TestFixture::new().await;
    
    // Setup error scenario
    fixture.mock_provider.setup_oauth_error("invalid_request", "Missing parameter").await;
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[("error", "invalid_request")])
        .await;
    
    // Verify error handling
    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    ResponseAssertions::assert_oauth_error_response(&body, "invalid_request");
}
```

### 4. Testing State Management

```rust
#[tokio::test]
async fn test_state_security() {
    let fixture = TestFixture::new().await;
    
    // Test tampered state
    let tampered_state = OAuthStateFixtures::tampered_state();
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[
            ("code", "test_code"),
            ("state", &tampered_state)
        ])
        .await;
    
    // Should reject tampered state
    response.assert_status(StatusCode::BAD_REQUEST);
}
```

### 5. Adding New Fixtures

#### OAuth State Fixtures

```rust
// In tests/common/fixtures.rs
impl OAuthStateFixtures {
    pub fn custom_state_scenario() -> String {
        let state = http_server::oauth_state::OAuthState {
            operation: http_server::oauth_state::OAuthOperation::CustomOp,
            nonce: Uuid::new_v4().to_string(),
            user_id: Some(UserFixtures::test_user_id()),
        };
        state.encode().expect("Failed to encode custom state")
    }
}
```

#### Mock Provider Extensions

```rust
// In tests/common/fixtures.rs
impl MockOAuthProvider {
    pub async fn setup_custom_provider(&self) -> String {
        let auth_code = "custom_auth_code";
        
        Mock::given(method("POST"))
            .and(path_regex("/custom/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "custom_token",
                "token_type": "bearer"
            })))
            .mount(&self.server)
            .await;
        
        auth_code.to_string()
    }
}
```

### 6. Database Integration Tests

```rust
#[tokio::test]
async fn test_database_operations() {
    let fixture = TestFixture::new().await;
    
    // Direct database operations for complex scenarios
    let user_id = Uuid::new_v4();
    
    // Insert test data
    sqlx::query!(
        "INSERT INTO users (id, username, email) VALUES ($1, $2, $3)",
        user_id,
        "testuser",
        "test@example.com"
    )
    .execute(&fixture.database.pool)
    .await
    .expect("Failed to insert test user");
    
    // Test your endpoint
    let response = fixture
        .server
        .get(&format!("/users/{}", user_id))
        .await;
    
    response.assert_status_ok();
    
    // Database is automatically cleaned up via fixture.database.cleanup()
}
```

## Best Practices

### Test Organization

1. **One Feature Per Test**: Each test should validate one specific feature or scenario
2. **Clear Test Names**: Use descriptive names that explain what is being tested
3. **Group Related Tests**: Use modules or comments to group related functionality
4. **Test Both Success and Failure**: Always test error scenarios alongside happy paths

### Performance Optimization

1. **Use Shared Database**: Always use `TestFixture::new()` for database efficiency
2. **Minimal Setup**: Only setup what each test actually needs
3. **Fast Assertions**: Use helper methods for common assertion patterns
4. **Parallel Safe**: Ensure tests can run concurrently without interference

### Security Testing

1. **State Tampering**: Always test OAuth state parameter security
2. **Authentication**: Test both authenticated and unauthenticated scenarios  
3. **Authorization**: Verify proper access controls
4. **Input Validation**: Test malformed or malicious inputs

### Maintainability

1. **Use Fixtures**: Prefer fixtures over inline test data
2. **Consistent Patterns**: Follow established patterns for similar tests
3. **Clear Assertions**: Use descriptive assertion messages
4. **Mock Realistic Data**: Ensure mocks match real provider responses

## Troubleshooting

### Common Issues

#### Docker/Testcontainers Issues

```bash
# Check Docker status
docker ps
systemctl status docker  # Linux
open -a Docker           # macOS

# Clean up containers
docker container prune -f
just clean-docker
```

#### Database Connection Issues

```bash
# Increase timeouts for slower systems
TEST_DB_TIMEOUT=120 just test-integration

# Check container logs
docker logs <container_id>
```

#### Mock Server Issues

```bash
# Verify wiremock setup
RUST_LOG=wiremock=debug just test-debug

# Check port conflicts
netstat -an | grep LISTEN
```

#### CI/CD Issues

```bash
# GitHub Actions
CI=true TEST_VERBOSE=true just test-ci

# Reduce resource usage  
TEST_MAX_CONCURRENCY=1 just test-integration
```

### Debugging Tools

```bash
# Maximum verbosity
RUST_LOG=trace just test-single your_test_name

# Database query logging
RUST_LOG=sqlx=debug,sea_orm=debug just test-debug

# HTTP request/response logging
RUST_LOG=axum_test=debug,reqwest=debug just test-debug
```

### Test Development Workflow

1. **Start with Simple Test**: Begin with basic happy path
2. **Add Error Cases**: Systematically add error scenarios
3. **Use Watch Mode**: `just watch` for rapid iteration
4. **Debug Step by Step**: Use single test debugging when issues arise
5. **Verify in CI**: Test changes in CI environment before merging

## Quick Reference

### Essential Commands

```bash
# Development workflow
just test-integration          # Run all OAuth tests
just test-single <test_name>   # Debug specific test
just watch                     # Auto-run on changes
just test-debug               # Full logging

# Specific test groups
just test-start               # OAuth start endpoints
just test-callback            # OAuth callbacks  
just test-state              # State management
```

### Copy-Paste Test Templates

#### Basic OAuth Endpoint Test

```rust
#[tokio::test]
async fn test_oauth_new_endpoint() {
    let fixture = TestFixture::new().await;
    
    let response = fixture
        .server
        .get("/auth/github/new-endpoint")
        .await;
    
    response.assert_status(StatusCode::OK);
    let body: Value = response.json();
    assert_eq!(body["status"], "success");
}
```

#### Authenticated Endpoint Test

```rust
#[tokio::test]
async fn test_authenticated_endpoint() {
    let fixture = TestFixture::new().await;
    
    let token = UserFixtures::test_jwt_token();
    let (header_name, header_value) = TestRequestBuilder::auth_header(&token);
    
    let response = fixture
        .server
        .get("/protected-endpoint")
        .add_header(header_name, header_value)
        .await;
    
    response.assert_status_ok();
}
```

#### OAuth Callback with Mock Data

```rust
#[tokio::test]
async fn test_oauth_callback_flow() {
    let fixture = TestFixture::new().await;
    
    let auth_code = fixture.mock_provider.setup_github_success().await;
    let state = OAuthStateFixtures::valid_login_state();
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[
            ("code", &auth_code),
            ("state", &state)
        ])
        .await;
    
    response.assert_status_ok();
    let body: Value = response.json();
    ResponseAssertions::assert_oauth_success_response(&body);
}
```

#### Error Handling Test

```rust
#[tokio::test]
async fn test_error_scenario() {
    let fixture = TestFixture::new().await;
    
    fixture.mock_provider.setup_oauth_error("access_denied", "User denied").await;
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[("error", "access_denied")])
        .await;
    
    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    assert!(body["error"].as_str().unwrap().contains("access_denied"));
}
```

### Most Used Fixtures

```rust
// OAuth states
let login_state = OAuthStateFixtures::valid_login_state();
let link_state = OAuthStateFixtures::valid_link_state(user_id);
let tampered_state = OAuthStateFixtures::tampered_state();

// User data
let user_id = UserFixtures::test_user_id();
let jwt_token = UserFixtures::test_jwt_token();

// Mock setups
let auth_code = fixture.mock_provider.setup_github_success().await;
let auth_code = fixture.mock_provider.setup_gitlab_success().await;
fixture.mock_provider.setup_oauth_error("error_type", "description").await;

// Response assertions
ResponseAssertions::assert_oauth_success_response(&body);
ResponseAssertions::assert_oauth_error_response(&body, "expected_error");
ResponseAssertions::assert_redirect_has_params(location, &["state", "code"]);
```

This comprehensive testing framework ensures robust, maintainable, and reliable OAuth authentication functionality while providing excellent developer experience for test development and debugging. 