# Testing Guide

Comprehensive guide for writing tests in the IAM (Identity Access Management) system. This guide covers integration testing patterns, test server management, database testing, and best practices.

## Table of Contents

- [Overview](#overview)
- [Test Architecture](#test-architecture)
- [Test Server Management](#test-server-management)
- [HTTP Testing Utilities](#http-testing-utilities)
- [Integration Testing Patterns](#integration-testing-patterns)
- [Database Testing](#database-testing)
- [OAuth and Authentication Tests](#oauth-and-authentication-tests)
- [Test Utilities and Helpers](#test-utilities-and-helpers)
- [Best Practices](#best-practices)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

The testing system is designed for comprehensive integration testing with real database and HTTP server instances. It provides:

- **Isolated Test Environment**: Each test runs with a clean database state
- **Real Server Testing**: Tests run against actual HTTP server instances
- **External Service Mocking**: Comprehensive fixture system for OAuth providers
- **Serial Test Execution**: Ensures proper isolation and resource management
- **Automatic Cleanup**: Database tables are truncated between tests
- **Configuration Management**: Test-specific configuration with random ports

### Key Components

1. **Test Server Management** (`test_server.rs`): Global test server lifecycle management
2. **HTTP Test Utilities** (`http_test.rs`): Server spawning and configuration
3. **Database Testing** (`database.rs`): Database container and cleanup utilities
4. **Fixtures System**: External service mocking (documented in [FIXTURES_GUIDE.md](FIXTURES_GUIDE.md))
5. **Integration Tests**: Complete OAuth flow testing patterns

## Test Architecture

### Directory Structure

```
tests/
├── auth_oauth_start.rs          # OAuth authentication flow tests
├── common/
│   ├── mod.rs                   # Common test utilities exports
│   ├── test_server.rs           # Test server lifecycle management
│   ├── http_test.rs             # HTTP server spawning utilities
│   └── database.rs              # Database testing utilities
├── fixtures/                    # External service mocking system
│   ├── mod.rs
│   ├── github/                  # GitHub API mocking
│   ├── gitlab/                  # GitLab API mocking
│   └── db/                      # Database fixtures
└── example/                     # Example tests and documentation
```

### Dependencies and Imports

All tests should include these common imports:

```rust
// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::get_test_server;
use fixtures::{GitHubFixtures, GitLabFixtures};
use reqwest::Client;
use serde_json::Value;
use serial_test::serial;
```

### Test Attributes

All integration tests should use these attributes:

```rust
#[tokio::test]  // For async test execution
#[serial]       // For sequential execution to ensure isolation
async fn test_name() {
    // Test implementation
}
```

## Test Server Management

The test server management system (`test_server.rs`) provides a global test server instance that automatically handles server lifecycle, database setup, and health checking.

### Architecture

```rust
/// Global test server instance that starts only once
static TEST_SERVER: OnceLock<Arc<Mutex<Option<JoinHandle<()>>>>> = OnceLock::new();

/// Get or create the global test server instance
pub async fn get_test_server() -> Result<String, Box<dyn std::error::Error>>
```

### Key Features

1. **Single Instance**: Only one server instance runs across all tests
2. **Health Checking**: Automatic server readiness verification
3. **Lifecycle Management**: Detects dead servers and respawns automatically
4. **Database Integration**: Ensures test database is ready before server start
5. **Configuration-Based URLs**: Returns correct base URL from test configuration

### Server Lifecycle

The server management system follows this lifecycle:

1. **Check Existing Server**: Determine if a server handle exists and is alive
2. **Database Setup**: Ensure test database container is running
3. **Server Spawning**: Launch new server if needed
4. **Health Verification**: Wait for server to respond to health checks
5. **URL Resolution**: Return the correct base URL for test clients

### Usage Pattern

```rust
#[tokio::test]
#[serial]
async fn test_oauth_endpoint() {
    // Get test server (automatically handles setup)
    let base_url = get_test_server().await.expect("Failed to start test server");
    
    // Create HTTP client
    let client = create_test_client();
    
    // Make requests to test server
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // Assertions...
}
```

### Server Health Checking

The system includes robust health checking with retries:

```rust
// Try to connect to the server with retries
for attempt in 1..=10 {
    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempt)).await;
    
    if let Ok(response) = client.get(&format!("{}/health", base_url)).send().await {
        if response.status().is_success() {
            println!("✅ Server is ready after {} attempts", attempt);
            break;
        }
    }
    
    if attempt == 10 {
        eprintln!("❌ Server failed to start after 10 attempts");
        return Err("Server failed to start".into());
    }
    
    println!("⏳ Waiting for server to start (attempt {}/10)...", attempt);
}
```

### Server State Detection

The system intelligently manages server state:

```rust
// Check if we need to start a new server
let needs_new_server = match server_guard.as_ref() {
    None => true, // No server handle exists
    Some(handle) => handle.is_finished(), // Server handle exists but task is finished
};

if needs_new_server {
    // If the old handle is finished, clear it
    if server_guard.is_some() {
        println!("🔄 Previous server has stopped, starting a new one...");
        *server_guard = None;
    }
    
    // Start new server...
} else {
    println!("♻️  Reusing existing server instance");
}
```

### Error Handling

The server management includes comprehensive error handling:

- **Database Setup Failures**: Proper error propagation with context
- **Server Start Failures**: Detailed error messages with debugging info
- **Health Check Timeouts**: Configurable retry logic with exponential backoff
- **Configuration Errors**: Clear error messages for config loading issues

## HTTP Testing Utilities

The HTTP testing utilities (`http_test.rs`) provide the core server spawning functionality used by the test server management system.

### Core Function

```rust
pub async fn spawn_test_server() -> anyhow::Result<()> {
    // Use your real config loading logic
    let config = load_config().expect("failed to load test config");

    eprintln!("🚀 Starting test server with configuration:");
    eprintln!("   Server host: {}", config.server.host);
    eprintln!("   Server port: {}", config.server.port);
    eprintln!("   TLS enabled: {}", config.server.tls_enabled);
    eprintln!("   Database URL: {}", config.database.url());
    eprintln!("   Database actual port: {}", config.database.actual_port());

    // Create server configuration
    let server_config = config::ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled { Some(config.server.tls_cert_path.clone()) } else { None },
        tls_key_path: if config.server.tls_enabled { Some(config.server.tls_key_path.clone()) } else { None },
        tls_port: if config.server.tls_enabled { Some(config.server.tls_port) } else { None },
    };

    eprintln!("🌐 Test server will listen on: http://{}:{}", server_config.host, server_config.port);

    // Build and run the application - this should run indefinitely
    eprintln!("🔄 Starting server...");
    app::build_and_run(config, server_config).await
}
```

### Configuration Integration

The HTTP utilities integrate with the configuration system:

1. **Load Test Config**: Uses the standard config loading with test environment
2. **Server Config Creation**: Translates app config to server config format
3. **TLS Handling**: Properly configures TLS settings for test environment
4. **Port Management**: Uses test-specific ports to avoid conflicts

### Logging and Debugging

The system provides comprehensive logging for debugging:

```rust
eprintln!("🚀 Starting test server with configuration:");
eprintln!("   Server host: {}", config.server.host);
eprintln!("   Server port: {}", config.server.port);
eprintln!("   TLS enabled: {}", config.server.tls_enabled);
eprintln!("   Database URL: {}", config.database.url());
eprintln!("   Database actual port: {}", config.database.actual_port());
```

### Integration with App Framework

The spawn function integrates with the main application framework:

```rust
// Build and run the application - this should run indefinitely
app::build_and_run(config, server_config).await
```

This ensures that tests run against the same application code that runs in production.

## Integration Testing Patterns

Based on the `auth_oauth_start.rs` file, here are the established patterns for writing integration tests.

### Basic Test Structure

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_github_redirect_success() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup external service fixtures (scoped to this test)
    let _github_service = GitHubFixtures::service().await;
    
    // Make request to endpoint
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // Assert response status
    assert_eq!(response.status(), 303, "Should return 303 redirect status");
    
    // Assert response headers
    let location = response
        .headers()
        .get("location")
        .expect("Should have Location header")
        .to_str()
        .expect("Location header should be valid string");
    
    // Assert redirect destination
    assert!(location.contains("github.com") || location.contains("localhost:3000"), 
           "Should redirect to GitHub OAuth provider (or mock)");
    
    // Parse and validate query parameters
    let (base_path, params) = parse_redirect_url(location)
        .expect("Should be able to parse redirect URL");
    
    // Detailed parameter validation...
}
```

### HTTP Client Configuration

All tests use a standardized HTTP client that doesn't follow redirects:

```rust
/// Create a common HTTP client for tests that doesn't follow redirects automatically
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}
```

This pattern allows tests to:
- Verify redirect responses explicitly
- Check redirect destinations and parameters
- Test the actual HTTP flow without automatic following

### URL Parsing and Validation

Complex URL validation uses helper functions:

```rust
/// Helper function to verify redirect URL structure and extract query parameters
fn parse_redirect_url(location: &str) -> Result<(String, std::collections::HashMap<String, String>), Box<dyn std::error::Error>> {
    let url = Url::parse(location)?;
    let mut params = std::collections::HashMap::new();
    
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }
    
    Ok((url.origin().ascii_serialization() + url.path(), params))
}
```

### State Parameter Validation

OAuth state parameters require special validation:

```rust
/// Helper function to decode and verify OAuth state parameter
fn decode_oauth_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(state)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;
    let state_json: Value = serde_json::from_str(&decoded_str)?;
    Ok(state_json)
}

// Usage in tests:
let state = params.get("state").unwrap();
let decoded_state = decode_oauth_state(state)
    .expect("Should be able to decode state parameter");

assert_eq!(decoded_state["operation"]["type"], "login", 
          "State should contain login operation type");
assert!(decoded_state["nonce"].is_string(), 
       "State should contain nonce for security");
```

### Error Testing Patterns

Test error scenarios comprehensively:

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_unsupported_provider_returns_400() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Test multiple error cases
    let unsupported_providers = vec!["facebook", "google", "twitter", "unknown", ""];
    
    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider))
            .send()
            .await
            .expect("Failed to send request");
        
        // Verify error status
        assert_eq!(response.status(), 400, 
                  "Should return 400 Bad Request for unsupported provider: {}", provider);
        
        // Verify error response structure
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert_eq!(error_response["operation"], "start", 
                  "Error response should indicate 'start' operation");
        assert_eq!(error_response["error"], "invalid_provider", 
                  "Error response should indicate 'invalid_provider'");
        assert!(error_response["message"].is_string(), 
               "Error response should have error message");
    }
}
```

### Case Sensitivity Testing

Test various input variations:

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_case_insensitive_providers() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup fixtures
    let _github_service = GitHubFixtures::service().await;
    let _gitlab_service = GitLabFixtures::service().await;
    
    // Test case variations that should work
    let valid_cases = vec![
        ("github", "GitHub"),
        ("GITHUB", "GitHub"), 
        ("GitHub", "GitHub"),
        ("gitlab", "GitLab"),
        ("GITLAB", "GitLab"),
        ("GitLab", "GitLab"),
    ];
    
    for (provider_input, expected_provider) in valid_cases {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider_input))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 303, 
                  "Should handle case-insensitive provider: {}", provider_input);
        
        // Verify correct provider handling...
    }
}
```

### Security Testing Patterns

Test security aspects like state uniqueness:

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_state_security_and_uniqueness() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    let _github_service = GitHubFixtures::service().await;
    
    // Make multiple requests to verify state uniqueness
    let mut states = std::collections::HashSet::new();
    
    for i in 0..5 {
        let response = client
            .get(&format!("{}/api/auth/github/start", base_url))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 303);
        
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        let (_, params) = parse_redirect_url(location)
            .expect("Should be able to parse redirect URL");
        
        let state = params.get("state").unwrap();
        
        // Each state should be unique
        assert!(!states.contains(state), 
               "State parameter should be unique across requests (iteration {})", i);
        states.insert(state.clone());
        
        // State should be properly formatted
        let decoded_state = decode_oauth_state(state)
            .expect("State should be valid base64 encoded JSON");
        
        // Verify security fields
        assert_eq!(decoded_state["operation"]["type"], "login");
        assert!(decoded_state["nonce"].is_string());
        
        // Nonce should be a valid UUID format
        let nonce = decoded_state["nonce"].as_str().unwrap();
        assert!(uuid::Uuid::parse_str(nonce).is_ok(), 
               "Nonce should be a valid UUID");
    }
    
    assert_eq!(states.len(), 5, "Should generate 5 unique state parameters");
}
```

### Authorization Header Testing

Test authenticated endpoints:

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_with_auth_header_link_operation() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    let _github_service = GitHubFixtures::service().await;
    
    // Mock JWT token for testing
    let mock_jwt_token = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
    
    // Make request with Authorization header
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .header("Authorization", mock_jwt_token)
        .send()
        .await
        .expect("Failed to send request");
    
    // Handle different response scenarios
    if response.status() == 303 {
        // Valid token - check for link operation
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        let (_, params) = parse_redirect_url(location)
            .expect("Should be able to parse redirect URL");
        
        let state = params.get("state").unwrap();
        let decoded_state = decode_oauth_state(state)
            .expect("Should be able to decode state parameter");
        
        assert_eq!(decoded_state["operation"]["type"], "link", 
                  "State should contain link operation type when Authorization header is present");
        assert!(decoded_state["operation"]["user_id"].is_string(), 
               "Link operation should contain user_id");
    } else if response.status() == 401 {
        // Invalid token - verify error response
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert_eq!(error_response["operation"], "start");
        assert_eq!(error_response["error"], "invalid_token");
    } else {
        panic!("Unexpected response status: {}", response.status());
    }
}
```

## Database Testing

The database testing system provides a comprehensive PostgreSQL container-based testing environment with automatic cleanup.

### Test Database Architecture

```rust
/// Global test database container instance
static TEST_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestDatabaseContainer>>>>> = OnceLock::new();

/// Test database fixture providing database connection and cleanup utilities
pub struct TestDatabase {
    pub pool: DbConnectionPool,
    pub connection: Arc<DatabaseConnection>,
    pub database_url: String,
}
```

### Basic Database Test Pattern

```rust
mod common;

use common::TestDatabase;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_database_operations() {
    // Create test database instance
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.get_connection();
    
    // Run your database operations
    // Tables are automatically truncated between tests
    
    // Verify results
    // No manual cleanup needed
}
```

### Database Container Management

The system uses a single PostgreSQL container for all tests:

1. **Container Creation**: Automatic PostgreSQL container setup with random ports
2. **Migration Execution**: Automatic database schema migration
3. **Connection Pooling**: Shared connection pool for all tests
4. **Table Truncation**: Automatic cleanup between tests without container restart

### Table Truncation System

Between each test, all tables are truncated:

```rust
/// Truncate all tables to clean up between tests
pub async fn truncate_all_tables(&self) -> Result<(), DbErr> {
    debug!("Truncating all tables for test cleanup");
    
    // Get all table names from the database
    let tables = self.get_all_table_names().await?;
    
    if tables.is_empty() {
        debug!("No tables found to truncate");
        return Ok(());
    }
    
    // Disable foreign key checks temporarily
    self.connection
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SET session_replication_role = replica;".to_string(),
        ))
        .await?;
    
    // Truncate all tables
    for table in &tables {
        let sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE;", table);
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            ))
            .await?;
    }
    
    // Re-enable foreign key checks
    self.connection
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SET session_replication_role = DEFAULT;".to_string(),
        ))
        .await?;
    
    debug!("Successfully truncated {} tables", tables.len());
    Ok(())
}
```

### TestFixture Integration

For complete integration testing, use `TestFixture`:

```rust
mod common;

use common::TestFixture;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_complete_integration() {
    // Setup complete test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let config = test_fixture.config();
    
    // Test with real database and configuration
    // Database is automatically cleaned up
}
```

### Database Entity Testing

```rust
use sea_orm::{EntityTrait, ActiveModelTrait, ActiveValue};
use infra::repository::entity::users::{Entity as UsersEntity, ActiveModel as UserActiveModel};

#[tokio::test]
#[serial]
async fn test_user_creation() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user entity
    let user_active_model = UserActiveModel {
        username: ActiveValue::Set("test_user".to_string()),
        avatar_url: ActiveValue::Set(Some("https://example.com/avatar.png".to_string())),
        ..Default::default()
    };
    
    let user = user_active_model.insert(&*db).await.expect("Failed to create user");
    
    // Verify user was created
    let found_user = UsersEntity::find_by_id(user.id)
        .one(&*db)
        .await
        .expect("Failed to query user")
        .expect("User should exist");
    
    assert_eq!(found_user.username, "test_user");
    assert_eq!(found_user.avatar_url, Some("https://example.com/avatar.png".to_string()));
    
    // Count users to verify isolation
    let user_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users".to_string(),
        ))
        .await
        .expect("Failed to count users")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(user_count, 1);
}
```

## OAuth and Authentication Tests

OAuth and authentication testing requires coordination between external service mocks and database state.

### Complete OAuth Flow Testing

```rust
mod common;
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::{GitHubFixtures, DbFixtures};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_complete_github_oauth_flow() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Step 1: Start OAuth flow
    let start_response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to start OAuth flow");
    
    assert_eq!(start_response.status(), 303);
    
    // Extract authorization URL and state
    let location = start_response.headers().get("location").unwrap().to_str().unwrap();
    let (_, params) = parse_redirect_url(location).unwrap();
    let state = params.get("state").unwrap();
    
    // Step 2: Simulate OAuth callback
    let callback_response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("code", "test_auth_code"), ("state", state)])
        .send()
        .await
        .expect("Failed to complete OAuth callback");
    
    assert_eq!(callback_response.status(), 200);
    
    // Step 3: Verify database state
    let user_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users".to_string(),
        ))
        .await
        .expect("Failed to count users")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(user_count, 1, "Should create one user");
    
    // Verify provider token was created
    let token_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM provider_tokens WHERE provider = 'github'".to_string(),
        ))
        .await
        .expect("Failed to count tokens")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(token_count, 1, "Should create one GitHub token");
}
```

### Pre-existing User Linking

```rust
#[tokio::test]
#[serial]
async fn test_oauth_provider_linking() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user with database fixtures
    let existing_user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    let primary_email = DbFixtures::user_email()
        .arthur_primary(existing_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create email");
    
    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Create mock JWT for authenticated request
    let mock_jwt = generate_test_jwt(existing_user.id());
    
    // Start OAuth flow with authentication (linking)
    let start_response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .header("Authorization", format!("Bearer {}", mock_jwt))
        .send()
        .await
        .expect("Failed to start OAuth linking");
    
    assert_eq!(start_response.status(), 303);
    
    // Complete OAuth flow...
    
    // Verify no new user was created
    let user_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users".to_string(),
        ))
        .await
        .expect("Failed to count users")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(user_count, 1, "Should not create new user for linking");
    
    // Verify GitHub token was linked to existing user
    assert!(existing_user.check(db.clone()).await.expect("Failed to check user"));
    assert!(primary_email.check(db.clone()).await.expect("Failed to check email"));
}
```

## Test Utilities and Helpers

### Helper Function Patterns

Common helper functions for test utilities:

```rust
/// Create HTTP client that doesn't follow redirects
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// Parse OAuth redirect URL and extract parameters
fn parse_redirect_url(location: &str) -> Result<(String, HashMap<String, String>), Box<dyn std::error::Error>> {
    let url = Url::parse(location)?;
    let mut params = HashMap::new();
    
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }
    
    Ok((url.origin().ascii_serialization() + url.path(), params))
}

/// Decode base64-encoded OAuth state parameter
fn decode_oauth_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(state)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;
    let state_json: Value = serde_json::from_str(&decoded_str)?;
    Ok(state_json)
}

/// Generate test JWT token for authentication testing
fn generate_test_jwt(user_id: uuid::Uuid) -> String {
    // Implementation depends on your JWT system
    format!("test_jwt_for_user_{}", user_id)
}

/// Assert that response contains expected error structure
fn assert_error_response(response: &Value, operation: &str, error_code: &str) {
    assert_eq!(response["operation"], operation, 
              "Error response should indicate correct operation");
    assert_eq!(response["error"], error_code, 
              "Error response should indicate correct error code");
    assert!(response["message"].is_string(), 
           "Error response should have error message");
}
```

### Test Data Builders

Create builders for complex test data:

```rust
/// Builder for OAuth test scenarios
pub struct OAuthTestBuilder {
    provider: String,
    user_authenticated: bool,
    expected_operation: String,
}

impl OAuthTestBuilder {
    pub fn new(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            user_authenticated: false,
            expected_operation: "login".to_string(),
        }
    }
    
    pub fn with_authenticated_user(mut self) -> Self {
        self.user_authenticated = true;
        self.expected_operation = "link".to_string();
        self
    }
    
    pub async fn run_start_test(self, base_url: &str, client: &Client) -> TestResult {
        let mut request = client.get(&format!("{}/api/auth/{}/start", base_url, self.provider));
        
        if self.user_authenticated {
            request = request.header("Authorization", "Bearer test_jwt");
        }
        
        let response = request.send().await.expect("Failed to send request");
        
        TestResult {
            response,
            expected_operation: self.expected_operation,
        }
    }
}

/// Test result wrapper for fluent assertions
pub struct TestResult {
    response: reqwest::Response,
    expected_operation: String,
}

impl TestResult {
    pub async fn assert_redirect(self) -> Self {
        assert_eq!(self.response.status(), 303, "Should return redirect");
        self
    }
    
    pub async fn assert_state_operation(self) -> Self {
        let location = self.response.headers().get("location").unwrap().to_str().unwrap();
        let (_, params) = parse_redirect_url(location).unwrap();
        let state = params.get("state").unwrap();
        let decoded_state = decode_oauth_state(state).unwrap();
        
        assert_eq!(decoded_state["operation"]["type"], self.expected_operation);
        self
    }
}
```

### Assertion Helpers

Common assertion patterns:

```rust
/// Assert OAuth redirect response structure
fn assert_oauth_redirect(response: &reqwest::Response, provider: &str) {
    assert_eq!(response.status(), 303, "Should return 303 redirect");
    
    let location = response
        .headers()
        .get("location")
        .expect("Should have Location header")
        .to_str()
        .expect("Location header should be valid string");
    
    assert!(
        location.contains(&format!("{}.com", provider)) || location.contains("localhost:3000"),
        "Should redirect to {} OAuth provider (or mock)", provider
    );
}

/// Assert OAuth query parameters
fn assert_oauth_parameters(params: &HashMap<String, String>, provider: &str) {
    let required_params = vec!["client_id", "redirect_uri", "scope", "response_type", "state"];
    
    for param in required_params {
        assert!(params.contains_key(param), 
               "Should have required OAuth2 parameter '{}'", param);
        assert!(!params.get(param).unwrap().is_empty(), 
               "OAuth2 parameter '{}' should not be empty", param);
    }
    
    assert_eq!(params.get("response_type").unwrap(), "code", 
              "response_type should be 'code' for authorization code flow");
    
    let redirect_uri = params.get("redirect_uri").unwrap();
    assert!(redirect_uri.contains(&format!("/api/auth/{}/callback", provider)), 
           "redirect_uri should point to correct callback endpoint");
}

/// Assert error response structure
fn assert_api_error(response_json: &Value, operation: &str, error_code: &str) {
    assert_eq!(response_json["operation"], operation);
    assert_eq!(response_json["error"], error_code);
    assert!(response_json["message"].is_string());
    
    if let Some(details) = response_json.get("details") {
        assert!(details.is_object() || details.is_array());
    }
}
```

## Best Practices

### Test Organization

1. **Group Related Tests**: Use descriptive test function names and group related tests
2. **Serial Execution**: Always use `#[serial]` for integration tests
3. **Clear Test Names**: Use descriptive names that indicate what is being tested
4. **Comprehensive Coverage**: Test success cases, error cases, edge cases, and security aspects

### Resource Management

1. **Automatic Cleanup**: Rely on automatic table truncation and mock cleanup
2. **Scoped Fixtures**: Create fixtures within test scope for automatic cleanup
3. **Shared Server**: Use the global test server for efficiency
4. **Database Isolation**: Each test starts with a clean database state

### Error Handling

1. **Comprehensive Error Testing**: Test all possible error scenarios
2. **Realistic Error Responses**: Use actual error responses that match production
3. **Status Code Verification**: Always verify HTTP status codes
4. **Error Message Validation**: Check error message structure and content

### Security Testing

1. **State Uniqueness**: Verify OAuth state parameters are unique
2. **Parameter Validation**: Test input validation thoroughly
3. **Authorization Testing**: Test both authenticated and unauthenticated scenarios
4. **Token Security**: Verify token handling and validation

### Performance Considerations

1. **Shared Resources**: Use shared test server and database container
2. **Efficient Cleanup**: Use table truncation instead of container restart
3. **Parallel Safety**: Design tests to be thread-safe when possible
4. **Resource Limits**: Be mindful of test execution time and resource usage

## Common Patterns

### Testing API Endpoints

```rust
#[tokio::test]
#[serial]
async fn test_api_endpoint() {
    // 1. Setup
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    let _fixtures = ExternalServiceFixtures::service().await;
    
    // 2. Execute
    let response = client
        .method(&format!("{}/api/endpoint", base_url))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");
    
    // 3. Assert
    assert_eq!(response.status(), expected_status);
    let response_json: Value = response.json().await.expect("Failed to parse JSON");
    // Additional assertions...
}
```

### Testing with Database State

```rust
#[tokio::test]
#[serial]
async fn test_with_database() {
    // Setup
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    
    // Pre-create database entities
    let user = DbFixtures::user().arthur().commit(db.clone()).await?;
    
    // Execute API operation
    let response = client.post(&format!("{}/api/operation", base_url))
        .json(&request_data)
        .send()
        .await?;
    
    // Verify API response
    assert_eq!(response.status(), 200);
    
    // Verify database state
    assert!(user.check(db.clone()).await?);
    let updated_count = count_entities(db.clone(), "table_name").await?;
    assert_eq!(updated_count, expected_count);
}
```

### Testing External Service Integration

```rust
#[tokio::test]
#[serial]
async fn test_external_service() {
    // Setup mocks
    let external_service = ExternalServiceFixtures::service().await;
    external_service.setup_successful_response().await;
    
    // Setup database
    let test_fixture = TestFixture::new().await?;
    let db = test_fixture.db();
    
    // Execute integration
    let base_url = get_test_server().await?;
    let response = client.post(&format!("{}/api/integration", base_url))
        .json(&integration_data)
        .send()
        .await?;
    
    // Verify integration results
    assert_eq!(response.status(), 200);
    
    // Verify external service was called
    // (This depends on your mocking framework)
    
    // Verify database state changes
    let result_count = count_entities(db, "results").await?;
    assert_eq!(result_count, 1);
}
```

## Troubleshooting

### Common Issues

1. **Server Start Failures**
   - Check if port is already in use
   - Verify database container is running
   - Check configuration loading

2. **Database Connection Issues**
   - Ensure PostgreSQL container is running
   - Check database URL formatting
   - Verify migrations are applied

3. **Test Isolation Problems**
   - Always use `#[serial]` for integration tests
   - Verify table truncation is working
   - Check for static state between tests

4. **Mock Server Issues**
   - Verify fixtures are properly scoped
   - Check mock endpoint configurations
   - Ensure automatic cleanup is working

### Debugging Tips

1. **Enable Logging**
   ```rust
   use tracing_test::traced_test;
   
   #[tokio::test]
   #[serial]
   #[traced_test]
   async fn test_with_logging() {
       // Test implementation
       // Logs will be captured and displayed on test failure
   }
   ```

2. **Database State Inspection**
   ```rust
   // Add debugging queries to inspect database state
   let debug_count: i64 = db
       .query_one(Statement::from_string(
           DatabaseBackend::Postgres,
           "SELECT COUNT(*) as count FROM table_name".to_string(),
       ))
       .await?
       .unwrap()
       .try_get("", "count")?;
   
   eprintln!("Debug: table has {} rows", debug_count);
   ```

3. **Response Body Inspection**
   ```rust
   let response_text = response.text().await?;
   eprintln!("Response body: {}", response_text);
   
   // Then parse if needed
   let response_json: Value = serde_json::from_str(&response_text)?;
   ```

4. **Configuration Debugging**
   ```rust
   let config = test_fixture.config();
   eprintln!("Test config: {:#?}", config);
   ```

### Performance Issues

1. **Slow Test Execution**
   - Check if tests are running serially when they could run in parallel
   - Verify database container isn't being recreated unnecessarily
   - Look for inefficient database operations

2. **Resource Leaks**
   - Ensure proper cleanup of HTTP clients
   - Verify database connections are properly closed
   - Check for hanging server processes

3. **Flaky Tests**
   - Add proper waits for asynchronous operations
   - Increase timeouts for slow operations
   - Verify test isolation is complete

### CI/CD Considerations

1. **Container Management**: Ensure Docker is available in CI environment
2. **Port Conflicts**: Use random ports to avoid conflicts
3. **Resource Limits**: Consider resource constraints in CI environment
4. **Parallel Execution**: Be careful with parallel test execution
5. **Cleanup**: Ensure proper cleanup even on test failures

This comprehensive testing guide provides the foundation for writing robust, maintainable integration tests in the IAM system. The patterns and utilities ensure consistent test quality and reliable test execution across different environments. 