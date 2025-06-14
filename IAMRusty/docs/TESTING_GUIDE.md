# Testing Guide

Comprehensive guide for writing tests in the IAM (Identity Access Management) system. This guide covers integration testing patterns, test server management, database testing, and best practices.

## Table of Contents

- [Overview](#overview)
- [Test Architecture](#test-architecture)
- [Test Server Management](#test-server-management)
- [HTTP Testing Utilities](#http-testing-utilities)
- [Integration Testing Patterns](#integration-testing-patterns)
- [Database Testing](#database-testing)
- [External Service Testing](#external-service-testing)
- [Kafka and SQS Testing](#kafka-and-sqs-testing)
- [JWT Testing](#jwt-testing)
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
- **Container-Based Testing**: Uses TestContainers for PostgreSQL, Kafka, and SQS

### Key Components

1. **Test Server Management** (`tests/common/test_server.rs`): Global test server lifecycle management
2. **HTTP Test Utilities** (`tests/common/http_test.rs`): Server spawning and configuration
3. **Database Testing** (`tests/common/database.rs`): Database container and cleanup utilities
4. **Fixtures System** (`tests/fixtures/`): External service mocking (GitHub, GitLab, DB fixtures)
5. **Message Queue Testing**: Kafka (`kafka_testcontainer.rs`) and SQS (`sqs_testcontainer.rs`) containers
6. **JWT Testing** (`tests/common/jwt_test_utils.rs`): JWT token creation and validation utilities

## Test Architecture

### Directory Structure

```
tests/
├── auth_oauth_start.rs               # OAuth authentication flow tests
├── auth_oauth_callback.rs            # OAuth callback tests
├── auth_username_flow.rs             # Username/password authentication tests
├── auth_username_flow_part2.rs       # Extended username flow tests
├── auth_username_flow_advanced.rs    # Advanced username flow scenarios
├── auth_email_password.rs            # Email/password authentication tests
├── auth_complete_registration.rs     # Registration completion tests
├── auth_resend_verification.rs       # Email verification resend tests
├── auth_username_validation.rs       # Username validation tests
├── username_check_tests.rs           # Username availability tests
├── token.rs                          # Token management tests
├── user.rs                           # User management tests
├── internal_provider_token.rs        # Internal provider token tests
├── signup_sqs.rs                     # SQS signup integration tests
├── signup_kafka.rs                   # Kafka signup integration tests
├── common/
│   ├── mod.rs                        # Common test utilities exports
│   ├── test_server.rs                # Test server lifecycle management
│   ├── http_test.rs                  # HTTP server spawning utilities
│   ├── database.rs                   # Database testing utilities
│   ├── jwt_test_utils.rs             # JWT token testing utilities
│   ├── kafka_testcontainer.rs        # Kafka container management
│   ├── sqs_testcontainer.rs          # SQS container management
│   └── db_utils.rs                   # Database utility functions
└── fixtures/                         # External service mocking system
    ├── mod.rs                        # Fixture exports
    ├── github/                       # GitHub API mocking
    ├── gitlab/                       # GitLab API mocking
    ├── db/                           # Database fixtures
    └── common/                       # Common fixture utilities
```

### Dependencies and Imports

All tests should include these common imports:

```rust
// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::setup_test_server;
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};  
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

The test server management system (`tests/common/test_server.rs`) provides a global test server instance that automatically handles server lifecycle, database setup, and health checking.

### Architecture

```rust
/// Global test server instance that starts only once
static TEST_SERVER: OnceLock<Arc<Mutex<Option<JoinHandle<()>>>>> = OnceLock::new();

/// Get or create the global test server instance
pub async fn get_test_server() -> Result<String, Box<dyn std::error::Error>>

/// Setup method that returns fixture, base URL, and HTTP client
pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
```

### Key Features

1. **Single Instance**: Only one server instance runs across all tests
2. **Lifecycle Management**: Detects dead servers and respawns automatically
3. **Database Integration**: Ensures test database is ready before server start
4. **Configuration-Based URLs**: Returns correct base URL from test configuration
5. **Automatic Cleanup**: Integrated with TestFixture for resource management

### Server Lifecycle

The server management system follows this lifecycle:

1. **Check Existing Server**: Determine if a server handle exists and is alive
2. **Database Setup**: Ensure test database container is running
3. **Server Spawning**: Launch new server if needed
4. **Configuration Loading**: Load test configuration with random ports
5. **URL Resolution**: Return the correct base URL for test clients

### Usage Pattern

```rust
#[tokio::test]
#[serial]
async fn test_oauth_endpoint() {
    // Get test server with fixture and client
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    
    // Setup external service fixtures
    let _github_service = GitHubFixtures::service().await;
    
    // Make requests to test server
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // Assertions...
}
```

### HTTP Client Configuration

All tests use a standardized HTTP client that doesn't follow redirects:

```rust
pub fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}
```

## HTTP Testing Utilities

The HTTP testing utilities (`tests/common/http_test.rs`) provide the core server spawning functionality used by the test server management system.

### Core Function

```rust
pub async fn spawn_test_server() -> anyhow::Result<()> {
    // Use configuration::load_config() for test configuration
    let config = load_config().expect("failed to load test config");
    
    // Initialize logging for the test server
    if config.logging.level != "" {
        config::setup_logging(&config.logging.level);
    }

    // Create server configuration
    let server_config = config::ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled { Some(config.server.tls_cert_path.clone()) } else { None },
        tls_key_path: if config.server.tls_enabled { Some(config.server.tls_key_path.clone()) } else { None },
        tls_port: if config.server.tls_enabled { Some(config.server.tls_port) } else { None },
    };

    // Build and run the application - this should run indefinitely
    app::build_and_run(config, server_config).await
}
```

### Configuration Integration

The HTTP utilities integrate with the configuration system:

1. **Load Test Config**: Uses `configuration::load_config()` with test environment
2. **Server Config Creation**: Translates app config to server config format
3. **TLS Handling**: Properly configures TLS settings for test environment
4. **Port Management**: Uses test-specific ports to avoid conflicts

## Integration Testing Patterns

Based on the actual test files, here are the established patterns for writing integration tests.

### Basic Test Structure

```rust
#[tokio::test]
#[serial]
async fn test_oauth_start_github_redirect_success() {
    // Setup test server with fixture and client
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    
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
async fn test_oauth_start_unsupported_provider_returns_422() {
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    
    // Test multiple error cases
    let unsupported_providers = vec!["facebook", "google", "twitter", "unknown", ""];
    
    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider))
            .send()
            .await
            .expect("Failed to send request");
        
        // Verify error status (actual status is 422, not 400)
        assert_eq!(response.status(), 422, 
                  "Should return 422 Unprocessable Entity for unsupported provider: {}", provider);
        
        // Verify error response structure
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert!(error_response.get("provider_name").is_some(), 
               "Error response should contain provider_name");
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

/// Complete test fixture with database and configuration
pub struct TestFixture {
    pub database: TestDatabase,
    cleanup_container_on_drop: bool,
}
```

### Basic Database Test Pattern

```rust
use common::TestFixture;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_database_operations() {
    // Create test fixture (includes database setup)
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
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

## External Service Testing

### Fixtures System

The fixture system provides mocking for external services:

```rust
// Available fixtures
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};

// Usage in tests
let _github_service = GitHubFixtures::service().await;
let _gitlab_service = GitLabFixtures::service().await;
```

### Database Fixtures

```rust
// Pre-create test data
let user = DbFixtures::user().arthur().commit(db.clone()).await?;
let email = DbFixtures::user_email()
    .arthur_primary(user.id())
    .commit(db.clone())
    .await?;

// Verify fixtures exist
assert!(user.check(db.clone()).await?);
assert!(email.check(db.clone()).await?);
```

## Kafka and SQS Testing

### Kafka Testing

```rust
use common::TestKafkaFixture;

#[tokio::test]
#[serial]
async fn test_kafka_integration() {
    let kafka_fixture = TestKafkaFixture::new().await.expect("Failed to create Kafka fixture");
    
    // Test Kafka event publishing
    let result = kafka_fixture.verify_event_published("user_signup", 5).await;
    assert!(result.is_ok());
    
    // Get all messages from topic
    let messages = kafka_fixture.kafka.get_all_messages(10).await.expect("Failed to get messages");
    assert!(!messages.is_empty());
}
```

### SQS Testing

```rust
use common::TestSqsFixture;

#[tokio::test]
#[serial]
async fn test_sqs_integration() {
    let sqs_fixture = TestSqsFixture::new().await.expect("Failed to create SQS fixture");
    
    // Test SQS message publishing
    let result = sqs_fixture.verify_event_published("user_signup", 5).await;
    assert!(result.is_ok());
    
    // Get all messages from queue
    let messages = sqs_fixture.sqs.get_all_messages(10).await.expect("Failed to get messages");
    assert!(!messages.is_empty());
}
```

## JWT Testing

### JWT Test Utilities

The JWT testing utilities provide comprehensive token creation and validation:

```rust
use common::{create_valid_jwt_token_with_encoder, create_expired_jwt_token_with_encoder, create_invalid_jwt_token_with_encoder};

#[tokio::test]
#[serial]
async fn test_jwt_authentication() {
    let test_fixture = TestFixture::new().await?;
    let config = test_fixture.config();
    let user_id = uuid::Uuid::new_v4();
    
    // Create valid JWT token
    let valid_token = create_valid_jwt_token_with_encoder(user_id, &config)?;
    
    // Create expired JWT token
    let expired_token = create_expired_jwt_token_with_encoder(user_id, &config)?;
    
    // Create invalid JWT token
    let invalid_token = create_invalid_jwt_token_with_encoder(user_id, &config)?;
    
    // Test with different token types...
}
```

### Registration Token Testing

```rust
use common::{create_valid_registration_token_with_encoder, create_expired_registration_token_with_encoder};

#[tokio::test]
#[serial]
async fn test_registration_tokens() {
    let test_fixture = TestFixture::new().await?;
    let config = test_fixture.config();
    let user_id = uuid::Uuid::new_v4();
    let email = "test@example.com".to_string();
    
    // Create valid registration token
    let valid_token = create_valid_registration_token_with_encoder(user_id, email.clone(), &config)?;
    
    // Create expired registration token
    let expired_token = create_expired_registration_token_with_encoder(user_id, email, &config)?;
    
    // Test registration flow with different token types...
}
```

## Test Utilities and Helpers

### Helper Function Patterns

Common helper functions for test utilities:

```rust
/// Create HTTP client that doesn't follow redirects
pub fn create_test_client() -> Client {
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
```

## Best Practices

### Test Organization

1. **Group Related Tests**: Use descriptive test function names and group related tests in separate files
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
3. **Status Code Verification**: Always verify HTTP status codes (note: unsupported providers return 422, not 400)
4. **Error Message Validation**: Check error message structure and content

### Security Testing

1. **State Uniqueness**: Verify OAuth state parameters are unique
2. **Parameter Validation**: Test input validation thoroughly
3. **Authorization Testing**: Test both authenticated and unauthenticated scenarios
4. **Token Security**: Verify token handling and validation

## Common Patterns

### Testing API Endpoints

```rust
#[tokio::test]
#[serial]
async fn test_api_endpoint() {
    // 1. Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    let _external_fixtures = ExternalServiceFixtures::service().await;
    
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
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
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
}
```

### Testing External Service Integration

```rust
#[tokio::test]
#[serial]
async fn test_external_service() {
    // Setup mocks
    let _external_service = ExternalServiceFixtures::service().await;
    
    // Setup database
    let test_fixture = TestFixture::new().await?;
    let db = test_fixture.db();
    
    // Execute integration
    let (_fixture, base_url, client) = setup_test_server().await?;
    let response = client.post(&format!("{}/api/integration", base_url))
        .json(&integration_data)
        .send()
        .await?;
    
    // Verify integration results
    assert_eq!(response.status(), 200);
    
    // Verify database state changes
    // Database operations...
}
```

## Troubleshooting

### Common Issues

1. **Server Start Failures**
   - Check if port is already in use
   - Verify database container is running
   - Check configuration loading with `configuration::load_config()`

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

### Configuration Issues

1. **Random Port Conflicts**
   - Ensure `port = 0` is set in test configuration
   - Check that port caching is working correctly
   - Verify container port coordination

2. **Environment Variables**
   - Check that test environment variables are set
   - Verify `RUN_ENV=test` is configured
   - Ensure queue configuration matches test setup

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

This comprehensive testing guide provides the foundation for writing robust, maintainable integration tests in the IAM system. The patterns and utilities ensure consistent test quality and reliable test execution across different environments. 