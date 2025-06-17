# Testing Guide

Comprehensive guide for writing integration tests in the IAM (Identity Access Management) system using real database containers, HTTP servers, and external service mocking.

## Table of Contents

- [Overview](#overview)
- [Test Architecture](#test-architecture)
- [Test Infrastructure](#test-infrastructure)
- [Integration Testing Patterns](#integration-testing-patterns)
- [Database Testing](#database-testing)
- [HTTP Testing](#http-testing)
- [External Service Testing](#external-service-testing)
- [Message Queue Testing](#message-queue-testing)
- [JWT Testing](#jwt-testing)
- [Best Practices](#best-practices)
- [Example References](#example-references)

## Overview

The testing system focuses on comprehensive integration testing with real infrastructure components. Key characteristics:

- **Real Infrastructure**: PostgreSQL containers, HTTP servers, Kafka/SQS containers
- **Test Isolation**: Serial execution with database table truncation between tests
- **Fixture System**: Comprehensive mocking for external services and test data creation
- **Configuration-Based**: Leverages existing configuration system for test setup
- **Automatic Cleanup**: Infrastructure and test data cleanup without manual intervention

### Core Components

1. **Test Server Management** - Global HTTP server lifecycle
2. **Database Testing** - PostgreSQL containers with migration and cleanup
3. **Fixture System** - External service mocking and database entity creation
4. **Message Queue Testing** - Kafka and SQS container-based testing
5. **JWT Testing** - Token creation and validation utilities

## Test Architecture

### Directory Structure

```
tests/
├── common/                        # Test infrastructure
│   ├── mod.rs                    # Common exports
│   ├── test_server.rs            # HTTP server management  
│   ├── database.rs               # Database container management
│   ├── http_test.rs              # Server spawning utilities
│   ├── jwt_test_utils.rs         # JWT testing utilities
│   ├── kafka_testcontainer.rs    # Kafka container (optional)
│   ├── sqs_testcontainer.rs      # SQS container (optional)
│   └── mock_event_publisher.rs   # No-op event publisher
├── fixtures/                      # External service mocking and test data
│   ├── mod.rs                    # Fixture exports
│   ├── github/                   # GitHub OAuth mocking
│   ├── gitlab/                   # GitLab OAuth mocking
│   ├── db/                       # Database entity fixtures
│   └── common/                   # Shared fixture utilities
└── *.rs                          # Integration test files
```

### Test File Structure

All integration tests follow this pattern:

```rust
#[path = "common/mod.rs"]
mod common;
#[path = "fixtures/mod.rs"] 
mod fixtures;

use common::setup_test_server;
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_name() {
    // Test implementation
}
```

## Test Infrastructure

### Test Server Management

**Implementation**: `tests/common/test_server.rs`

**Global Singleton Pattern**:
- Single HTTP server instance across all tests
- Automatic server lifecycle management
- Health checking and restart on failure
- Configuration-based URL resolution

**Usage**:
```rust
let (_fixture, base_url, client) = setup_test_server().await?;
```

**Key Features**:
- Server reuse for performance
- Detects and handles dead servers
- Integrates with test database lifecycle
- Provides configured HTTP client

### Database Testing

**Implementation**: `tests/common/database.rs`

**Container-Based Approach**:
- Single PostgreSQL container for all tests
- Automatic migration execution
- Table truncation between tests (not container restart)
- Configuration system integration

**TestFixture Structure**:
```rust
pub struct TestFixture {
    pub database: TestDatabase,
    cleanup_container_on_drop: bool,
}
```

**Key Features**:
- **Performance**: Container reuse with table truncation
- **Isolation**: Clean database state between tests
- **Integration**: Works with existing configuration system
- **Cleanup**: Automatic resource management

### HTTP Client Configuration

All tests use standardized HTTP client:
- No redirect following (to test redirects properly)
- Consistent configuration across tests
- Integrated with test server management

## Integration Testing Patterns

### Basic Test Structure

**Standard Pattern** (see `tests/auth_oauth_start.rs`):

1. **Setup**: Get test server, database, and HTTP client
2. **Mock Setup**: Create external service fixtures if needed
3. **Data Setup**: Create test entities with database fixtures if needed
4. **Execute**: Perform test operations
5. **Verify**: Assert results (cleanup automatic)

### OAuth Flow Testing

**Example Files**: `tests/auth_oauth_start.rs`, `tests/auth_oauth_callback.rs`

**Pattern**:
```rust
#[tokio::test]
#[serial]
async fn test_oauth_flow() {
    let (_fixture, base_url, client) = setup_test_server().await?;
    let _github = GitHubFixtures::service().await;
    
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send().await?;
    
    // Verify redirect, state, parameters, etc.
}
```

### Username/Password Flow Testing

**Example Files**: `tests/auth_username_flow.rs`, `tests/auth_email_password.rs`

**Pattern**: Tests authentication with database entities and validation.

### Registration Testing

**Example Files**: `tests/auth_complete_registration.rs`, `tests/auth_resend_verification.rs`

**Pattern**: Tests user registration flows with email verification.

## Database Testing

### Entity Creation with Fixtures

**Database Fixtures** (see `tests/fixtures/db/`):

```rust
// Create test entities
let user = DbFixtures::user().arthur().commit(db.clone()).await?;
let email = DbFixtures::user_email()
    .arthur_primary(user.id())
    .commit(db.clone()).await?;
```

### Helper Methods

**High-level helpers** (see `tests/fixtures/db/mod.rs`):
- `create_user_with_email_password()` - Complete user setup
- `create_user_without_username()` - Registration flow setup
- `create_user_with_oauth_provider()` - OAuth user setup

### Validation and Verification

**Entity State Verification**:
```rust
// Verify entities exist and match expected state
assert!(user.check(db.clone()).await?);
assert!(email.check(db.clone()).await?);
```

### Automatic Cleanup

**Table Truncation System**:
- All tables truncated between tests
- Foreign key constraints handled properly
- Identity columns reset
- No manual cleanup required

## HTTP Testing

### URL Parsing and Validation

**Helper Functions** (see `tests/auth_oauth_start.rs`):

```rust
fn parse_redirect_url(location: &str) -> Result<(String, HashMap<String, String>), Box<dyn std::error::Error>>
fn decode_oauth_state(state: &str) -> Result<Value, Box<dyn std::error::Error>>
```

### Response Validation

**Comprehensive Verification**:
- HTTP status codes
- Response headers (Location, Content-Type, etc.)
- JSON response structure and content
- Parameter validation (query parameters, state, etc.)

### Error Testing

**Error Scenario Coverage**:
- Invalid providers (422 status)
- Malformed requests
- Authentication failures
- Authorization errors

## External Service Testing

### Service Mocking

**Available Services**:
- **GitHub**: `GitHubFixtures::service()`
- **GitLab**: `GitLabFixtures::service()`

**Automatic Cleanup**: Mock servers automatically reset between tests.

### Mock Endpoint Configuration

**Endpoint Mocking** (see `tests/fixtures/github/service.rs`):
- OAuth token exchange endpoints
- User profile endpoints
- User email endpoints
- Custom endpoints for specific scenarios

### Convenience Methods

**Pre-built Scenarios**:
- `setup_successful_token_exchange()`
- `setup_successful_user_profile_arthur()`
- `setup_failed_token_exchange_invalid_code()`
- Error scenarios and rate limiting

## Message Queue Testing

### Kafka Testing

**Implementation**: `tests/common/kafka_testcontainer.rs`

**Container-Based Approach**:
- Apache Kafka 3.7.0 in KRaft mode
- Real event publishing and consumption
- Integration with configuration system

**Example**: `tests/signup_kafka.rs`

**Note**: Kafka tests are marked with `#[ignore]` and require Docker.

### SQS Testing

**Implementation**: `tests/common/sqs_testcontainer.rs`

**LocalStack-Based Approach**:
- Real SQS-compatible testing
- Event publishing verification
- Message consumption and validation

**Example**: `tests/signup_sqs.rs`

### Event Verification

**Multi-Level Validation**:
1. **Structure**: Required fields and data types
2. **Content**: Field values match operation inputs
3. **Business Logic**: Event data reflects operation results

## JWT Testing

### Token Creation Utilities

**Implementation**: `tests/common/jwt_test_utils.rs`

**Available Functions**:
- `create_valid_jwt_token_with_encoder()`
- `create_expired_jwt_token_with_encoder()`
- `create_invalid_jwt_token_with_encoder()`
- Registration token variants

### Token Validation Testing

**Usage Pattern**:
```rust
let valid_token = create_valid_jwt_token_with_encoder(user_id, &config)?;
let response = client
    .get(&format!("{}/api/protected", base_url))
    .header("Authorization", format!("Bearer {}", valid_token))
    .send().await?;
```

## Best Practices

### Test Organization

1. **Serial Execution**: Always use `#[serial]` for integration tests
2. **Descriptive Names**: Test function names should clearly indicate what's being tested
3. **File Organization**: Group related tests in logical files
4. **Reference Examples**: Follow patterns from existing test files

### Resource Management

1. **Automatic Cleanup**: Rely on automatic table truncation and fixture cleanup
2. **Fixture Scoping**: Create fixtures within test scope for automatic cleanup
3. **Server Reuse**: Use global test server for performance
4. **Container Efficiency**: Single containers per test file for message queues

### Error Testing

1. **Comprehensive Coverage**: Test success, error, and edge cases
2. **Realistic Errors**: Use actual error responses and status codes
3. **Status Code Verification**: Verify exact HTTP status codes (note: unsupported providers return 422)
4. **Security Testing**: Validate authentication and authorization properly

### Data Management

1. **Factory Methods**: Use factory methods (`arthur()`, `bob()`) for common scenarios
2. **Custom Data**: Use fluent API for test-specific requirements
3. **Relationships**: Create parent entities before children
4. **Validation**: Use `check()` methods to verify entity state

### Performance Considerations

1. **Container Reuse**: Minimize container startup overhead
2. **Database Efficiency**: Table truncation vs. container restart
3. **Mock Isolation**: Automatic cleanup without performance penalty
4. **Parallel Constraints**: Message queue tests run with Docker dependency

## Example References

### OAuth and Authentication

- **OAuth Start Flow**: `tests/auth_oauth_start.rs`
- **OAuth Callback**: `tests/auth_oauth_callback.rs`
- **Username Flow**: `tests/auth_username_flow.rs`
- **Email/Password**: `tests/auth_email_password.rs`

### Registration and Verification

- **Complete Registration**: `tests/auth_complete_registration.rs`
- **Email Verification**: `tests/auth_resend_verification.rs`
- **Username Validation**: `tests/auth_username_validation.rs`

### User and Token Management

- **User Management**: `tests/user.rs`
- **Token Management**: `tests/token.rs`
- **Username Checking**: `tests/username_check_tests.rs`

### Message Queue Integration

- **Kafka Integration**: `tests/signup_kafka.rs` (with `#[ignore]`)
- **SQS Integration**: `tests/signup_sqs.rs`

### Advanced Scenarios

- **Password Reset**: `tests/auth_password_reset.rs`
- **Internal Tokens**: `tests/internal_provider_token.rs`
- **Advanced Username Flows**: `tests/auth_username_flow_advanced.rs`

### Infrastructure Files

- **Test Server**: `tests/common/test_server.rs`
- **Database**: `tests/common/database.rs`
- **JWT Utils**: `tests/common/jwt_test_utils.rs`
- **Fixtures**: `tests/fixtures/` directory

This testing approach ensures comprehensive coverage of the IAM system while maintaining test isolation, performance, and reliability. 