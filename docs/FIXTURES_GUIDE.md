# Fixtures Guide

Comprehensive guide for using the fixture system to mock external services in tests.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Usage Patterns](#usage-patterns)
- [Available Fixtures](#available-fixtures)
- [Database Fixtures](#database-fixtures)
- [Writing Custom Fixtures](#writing-custom-fixtures)
- [Best Practices](#best-practices)

## Overview

The fixture system provides a structured approach to mocking external services using `wiremock`. It's designed to be:

- **Modular**: Each external service has its own fixture class
- **Fluent**: Chainable API for easy test setup
- **Reusable**: Pre-built resources and flows for common scenarios
- **Type-safe**: Strongly typed inputs and outputs
- **Integrated**: Works seamlessly with the test database system for complete integration testing
- **Self-Cleaning**: Automatic mock cleanup between tests for perfect isolation
- **Database-Aware**: Includes comprehensive database fixture system for entity creation and validation

### Key Components

1. **Service**: Main fixture class with fluent API for mocking endpoints
2. **Resources**: Type-safe data structures for inputs/outputs
3. **Flow**: Pre-made scenarios for common use cases
4. **MockServerFixture**: Automatic cleanup mechanism for test isolation
5. **DbFixtures**: Database entity fixtures with fluent API and validation

## Architecture

### Directory Structure

```
tests/
├── fixtures/
│   ├── mod.rs                    # Main fixtures module
│   ├── github/
│   │   ├── mod.rs               # GitHub fixture exports
│   │   ├── service.rs           # GitHubService (fluent API)
│   │   └── resources.rs         # GitHub data structures
│   ├── gitlab/
│   │   ├── mod.rs               # GitLab fixture exports
│   │   ├── service.rs           # GitLabService (fluent API)
│   │   └── resources.rs         # GitLab data structures
│   ├── db/
│   │   ├── mod.rs               # Database fixture exports
│   │   ├── common.rs            # Shared DB fixture traits
│   │   ├── users.rs             # User entity fixtures
│   │   ├── user_emails.rs       # UserEmail entity fixtures
│   │   ├── provider_tokens.rs   # ProviderToken entity fixtures
│   │   └── refresh_tokens.rs    # RefreshToken entity fixtures
│   └── common/
│       ├── mod.rs               # Common utilities
│       └── wiremock_server.rs   # Shared wiremock server
```

### Class Hierarchy

```
ExternalServiceFixtures
├── Service (Fluent API for endpoint mocking)
│   ├── endpoint_name(status_code, inputs, outputs)
│   ├── setup_successful_*() convenience methods
│   └── setup_failed_*() convenience methods
└── Resources (Type-safe data structures)
    ├── User (with builder pattern)
    ├── TokenRequest/TokenResponse
    ├── UserRequest
    └── Error
```

## Usage Patterns

### Basic Service Mocking

```rust
#[tokio::test]
async fn test_github_oauth_flow() {
    let github = GitHubFixtures::service().await;
    
    // Mock token exchange endpoint
    github
        .oauth_token(200, 
            GitHubTokenRequest::valid(),
            GitHubTokenResponse::success()
        )
        .await;
    
    // Mock user profile endpoint
    github
        .user_profile(200,
            GitHubUserRequest::authenticated(),
            GitHubUser::arthur()
        )
        .await;
    
    // Your test logic here...
}
```

### Individual Method Calls

```rust
#[tokio::test]
async fn test_github_error_scenarios() {
    let github = GitHubFixtures::service().await;
    
    // Mock error scenarios using individual calls
    github
        .oauth_token(400, 
            GitHubTokenRequest::invalid_code(),
            GitHubError::invalid_grant()
        )
        .await;
    
    github
        .user_profile(401,
            GitHubUserRequest::invalid_token(),
            GitHubError::unauthorized()
        )
        .await;
    
    // Test error handling...
}
```

### Using Convenience Methods

```rust
#[tokio::test]
async fn test_successful_github_login() {
    let github = GitHubFixtures::service().await;
    
    // Setup complete successful flow using convenience methods
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Test the complete OAuth flow...
}
```

### Resource Customization

```rust
#[tokio::test]
async fn test_custom_user_data() {
    let github = GitHubFixtures::service().await;
    
    let custom_user = GitHubUser::create()
        .id(12345)
        .login("custom_user")
        .email(Some("custom@example.com"))
        .avatar_url(Some("https://example.com/avatar.png"))
        .build();
    
    github
        .user_profile(200, 
            GitHubUserRequest::authenticated(),
            custom_user
        )
        .await;
}
```

### Integration with Test Database

The fixture system works seamlessly with the test database system for complete integration testing:

```rust
mod common;
mod fixtures;

use common::TestFixture;
use fixtures::GitHubFixtures;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_complete_oauth_flow() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let config = test_fixture.config();
    
    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Test complete OAuth flow with database persistence
    // - Mock external API calls with fixtures
    // - Verify database state changes
    // - Use real configuration from test fixture
    
    // Verify user was created in database
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

### Automatic Mock Cleanup

The fixture system includes automatic cleanup to ensure perfect test isolation:

```rust
#[tokio::test]
async fn test_automatic_cleanup() {
    // Each test gets a fresh set of mocks
    let github = GitHubFixtures::service().await;
    
    // Setup mocks for this test
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Test logic here...
    
    // Mocks are automatically cleaned up when github service is dropped
    // Next test will start with a completely clean slate
}

#[tokio::test]
async fn test_no_interference() {
    // This test won't see any mocks from the previous test
    let github = GitHubFixtures::service().await;
    
    // Can setup completely different mocks without conflicts
    github.setup_failed_token_exchange_invalid_code().await;
    
    // Perfect test isolation guaranteed
}
```

#### Manual Reset (Optional)

You can also manually reset mocks mid-test if needed:

```rust
#[tokio::test]
async fn test_manual_reset() {
    let github = GitHubFixtures::service().await;
    
    // Setup initial mocks
    github.setup_successful_token_exchange().await;
    
    // Test first scenario...
    
    // Reset mocks mid-test
    github.reset().await;
    
    // Setup different mocks for second scenario
    github.setup_failed_token_exchange_invalid_code().await;
    
    // Test second scenario...
}
```

## Available Fixtures

### GitHubFixtures

#### Service Methods

| Method | Description | Parameters |
|--------|-------------|------------|
| `oauth_token()` | Mock OAuth token exchange | status_code, request, response |
| `user_profile()` | Mock user profile fetch | status_code, request, response |
| `user_emails()` | Mock user emails fetch | status_code, request, response |

#### Resources

| Resource | Description | Factory Methods |
|----------|-------------|-----------------|
| `User` | GitHub user data | `arthur()`, `bob()`, `create()` |
| `TokenRequest` | Token exchange request | `valid()`, `invalid_code()` |
| `TokenResponse` | Token exchange response | `success()`, `expired()` |
| `Error` | GitHub API errors | `invalid_grant()`, `unauthorized()` |

#### Convenience Methods

| Method | Description |
|--------|-------------|
| `setup_successful_token_exchange()` | Mock successful OAuth token exchange |
| `setup_successful_user_profile_arthur()` | Mock successful user profile for Arthur |
| `setup_failed_token_exchange_invalid_code()` | Mock failed token exchange with invalid code |
| `setup_failed_user_profile_unauthorized()` | Mock unauthorized user profile access |
| `setup_rate_limit_exceeded()` | Mock rate limit exceeded error |

### GitLabFixtures

#### Service Methods

| Method | Description | Parameters |
|--------|-------------|------------|
| `oauth_token()` | Mock OAuth token exchange | status_code, request, response |
| `user_profile()` | Mock user profile fetch | status_code, request, response |

#### Resources

| Resource | Description | Factory Methods |
|----------|-------------|-----------------|
| `User` | GitLab user data | `alice()`, `charlie()`, `create()` |
| `TokenRequest` | Token exchange request | `valid()`, `invalid_code()` |
| `TokenResponse` | Token exchange response | `success()`, `expired()` |
| `Error` | GitLab API errors | `invalid_grant()`, `unauthorized()` |

#### Convenience Methods

| Method | Description |
|--------|-------------|
| `setup_successful_token_exchange()` | Mock successful OAuth token exchange |
| `setup_successful_user_profile_alice()` | Mock successful user profile for Alice |
| `setup_failed_token_exchange_invalid_code()` | Mock failed token exchange with invalid code |
| `setup_failed_user_profile_unauthorized()` | Mock unauthorized user profile access |

## Database Fixtures

The database fixture system provides a fluent API for creating and managing database entities in tests. It integrates seamlessly with the test database system and provides automatic cleanup through table truncation.

### Overview

Database fixtures offer:

- **Fluent API**: Chainable methods for entity creation
- **Type Safety**: Strongly typed entity builders
- **Factory Methods**: Pre-built entities for common scenarios
- **Validation**: Check methods to verify entity state against database
- **Integration**: Works with test database system for automatic cleanup
- **Relationships**: Support for entity relationships and foreign keys

### Architecture

```
DbFixtures
├── user()           # User entity fixture builder
├── user_email()     # UserEmail entity fixture builder  
├── provider_token() # ProviderToken entity fixture builder
└── refresh_token()  # RefreshToken entity fixture builder
```

Each entity fixture provides:
- **Builder Pattern**: Fluent API for setting entity properties
- **Factory Methods**: Pre-built entities (arthur(), bob(), alice(), etc.)
- **Commit Method**: Persist entity to database
- **Check Method**: Validate entity state against database

### Basic Usage

```rust
mod common;
mod fixtures;

use common::TestFixture;
use fixtures::DbFixtures;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_user_creation() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user using the fluent API
    let user = DbFixtures::user()
        .username("test_user")
        .avatar_url(Some("https://example.com/avatar.png".to_string()))
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Verify the user exists in the database
    assert!(user.check(db.clone()).await.expect("Failed to check user"));
    
    // Access user properties
    assert_eq!(user.username(), "test_user");
    assert_eq!(user.avatar_url(), Some(&"https://example.com/avatar.png".to_string()));
    
    println!("✅ User created with ID: {}", user.id());
}
```

### Factory Methods

Each entity fixture provides factory methods for common scenarios:

```rust
#[tokio::test]
#[serial]
async fn test_factory_methods() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create users using factory methods
    let arthur = DbFixtures::user().arthur().commit(db.clone()).await?;
    let bob = DbFixtures::user().bob().commit(db.clone()).await?;
    let alice = DbFixtures::user().alice().commit(db.clone()).await?;
    
    // Verify factory method data
    assert_eq!(arthur.username(), "arthur");
    assert_eq!(bob.username(), "bob");
    assert_eq!(alice.username(), "alice");
    
    // All users should exist in database
    assert!(arthur.check(db.clone()).await?);
    assert!(bob.check(db.clone()).await?);
    assert!(alice.check(db.clone()).await?);
}
```

### Entity Relationships

Create related entities with proper foreign key relationships:

```rust
#[tokio::test]
#[serial]
async fn test_entity_relationships() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user first
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await?;
    
    // Create related entities
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await?;
    
    let github_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await?;
    
    let refresh_token = DbFixtures::refresh_token()
        .arthur_valid(user.id())
        .commit(db.clone())
        .await?;
    
    // Verify relationships
    assert_eq!(primary_email.user_id(), user.id());
    assert_eq!(github_token.user_id(), user.id());
    assert_eq!(refresh_token.user_id(), user.id());
    
    // Verify all entities exist
    assert!(user.check(db.clone()).await?);
    assert!(primary_email.check(db.clone()).await?);
    assert!(github_token.check(db.clone()).await?);
    assert!(refresh_token.check(db.clone()).await?);
}
```

### Custom Entity Data

Use the fluent API to create entities with custom data:

```rust
#[tokio::test]
#[serial]
async fn test_custom_entity_data() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create custom user
    let custom_user = DbFixtures::user()
        .username("custom_test_user")
        .avatar_url(None)
        .commit(db.clone())
        .await?;
    
    // Create custom email with specific properties
    let custom_email = DbFixtures::user_email()
        .user_id(custom_user.id())
        .email("custom@test.example.com")
        .is_primary(true)
        .is_verified(false) // Unverified primary email
        .commit(db.clone())
        .await?;
    
    // Create custom provider token
    let custom_token = DbFixtures::provider_token()
        .user_id(custom_user.id())
        .provider("custom_provider")
        .access_token("custom_access_token_123")
        .refresh_token(None) // No refresh token
        .expires_in(Some(1800)) // 30 minutes
        .provider_user_id("custom_provider_user_456")
        .commit(db.clone())
        .await?;
    
    // Verify custom data
    assert_eq!(custom_user.username(), "custom_test_user");
    assert_eq!(custom_user.avatar_url(), None);
    
    assert_eq!(custom_email.email(), "custom@test.example.com");
    assert!(custom_email.is_primary());
    assert!(!custom_email.is_verified());
    
    assert_eq!(custom_token.provider(), "custom_provider");
    assert_eq!(custom_token.access_token(), "custom_access_token_123");
    assert_eq!(custom_token.refresh_token(), None);
    assert_eq!(custom_token.expires_in(), Some(1800));
}
```

### Fixture Validation with Check Methods

The check methods validate that fixture data matches the current database state:

```rust
#[tokio::test]
#[serial]
async fn test_fixture_validation() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create and commit a user fixture
    let original_user = DbFixtures::user()
        .username("original_user")
        .avatar_url(Some("https://example.com/original.png".to_string()))
        .commit(db.clone())
        .await?;
    
    // Initially, the fixture should match the database
    assert!(original_user.check(db.clone()).await?);
    
    // Manually modify the database (simulating external changes)
    use sea_orm::{EntityTrait, ActiveModelTrait, ActiveValue};
    use infra::repository::entity::users::{Entity as UsersEntity, ActiveModel as UserActiveModel};
    
    let mut user_active_model: UserActiveModel = UsersEntity::find_by_id(original_user.id())
        .one(&*db)
        .await?
        .expect("User should exist")
        .into();
    
    // Modify the user data directly in the database
    user_active_model.username = ActiveValue::Set("modified_user".to_string());
    user_active_model.avatar_url = ActiveValue::Set(Some("https://example.com/modified.png".to_string()));
    
    let _updated_user = user_active_model.update(&*db).await?;
    
    // The original fixture should now fail the check because the database has been updated
    // but the original fixture still has the old data
    let check_result = original_user.check(db.clone()).await?;
    assert!(!check_result, "Original fixture should fail check after database was modified");
    
    // Verify the original fixture still has the old data (unchanged)
    assert_eq!(original_user.username(), "original_user");
    assert_eq!(original_user.avatar_url(), Some(&"https://example.com/original.png".to_string()));
    
    println!("✅ Fixture validation correctly detected database changes");
}
```

### Available Entity Fixtures

#### User Fixtures

| Method | Description |
|--------|-------------|
| `username(name)` | Set username |
| `avatar_url(url)` | Set avatar URL (optional) |
| `arthur()` | Create Arthur user (GitHub) |
| `bob()` | Create Bob user (GitHub) |
| `alice()` | Create Alice user (GitLab) |
| `charlie()` | Create Charlie user (GitLab) |
| `no_avatar()` | Create user without avatar |

#### User Email Fixtures

| Method | Description |
|--------|-------------|
| `user_id(id)` | Set user ID (required) |
| `email(email)` | Set email address |
| `is_primary(bool)` | Set primary email flag |
| `is_verified(bool)` | Set verified flag |
| `arthur_primary(user_id)` | Arthur's primary email |
| `arthur_github(user_id)` | Arthur's GitHub email |
| `bob_primary(user_id)` | Bob's primary email |
| `alice_primary(user_id)` | Alice's primary email |
| `alice_gitlab(user_id)` | Alice's GitLab email |

#### Provider Token Fixtures

| Method | Description |
|--------|-------------|
| `user_id(id)` | Set user ID (required) |
| `provider(name)` | Set provider name |
| `access_token(token)` | Set access token |
| `refresh_token(token)` | Set refresh token (optional) |
| `expires_in(seconds)` | Set expiration time |
| `provider_user_id(id)` | Set provider user ID |
| `github(user_id)` | Create GitHub token |
| `gitlab(user_id)` | Create GitLab token |
| `arthur_github(user_id)` | Arthur's GitHub token |
| `bob_github(user_id)` | Bob's GitHub token |
| `alice_gitlab(user_id)` | Alice's GitLab token |

#### Refresh Token Fixtures

| Method | Description |
|--------|-------------|
| `user_id(id)` | Set user ID (required) |
| `token(token)` | Set token string |
| `is_valid(bool)` | Set validity flag |
| `expires_at(time)` | Set expiration time |
| `valid(user_id)` | Create valid token |
| `expired(user_id)` | Create expired token |
| `invalid(user_id)` | Create invalid token |
| `arthur_valid(user_id)` | Arthur's valid token |
| `bob_valid(user_id)` | Bob's valid token |
| `alice_valid(user_id)` | Alice's valid token |

### Integration with HTTP Fixtures

Combine database and HTTP fixtures for complete integration testing:

```rust
mod common;
mod fixtures;

use common::TestFixture;
use fixtures::{DbFixtures, GitHubFixtures};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_complete_oauth_integration() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Pre-create user in database
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await?;
    
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await?;
    
    // Test OAuth flow with both mocked HTTP and real database
    // - HTTP calls will hit the mocked GitHub server
    // - Database operations will use the real test database
    // - Entities created by fixtures provide expected test data
    
    // Verify initial state
    assert!(user.check(db.clone()).await?);
    assert!(primary_email.check(db.clone()).await?);
    
    // Run OAuth flow test...
    // The test will use mocked HTTP responses and real database operations
    
    println!("✅ Complete integration test with HTTP mocks and DB fixtures");
    println!("   GitHub mock URL: {}", github.base_url());
    println!("   User ID: {}", user.id());
    println!("   Primary email: {}", primary_email.email());
}
```

### Best Practices for Database Fixtures

1. **Use Serial Tests**: Always use `#[serial]` for database tests to ensure proper isolation
2. **Factory Methods**: Prefer factory methods for common scenarios
3. **Relationships**: Create parent entities before child entities
4. **Validation**: Use check methods to verify entity state
5. **Cleanup**: Rely on automatic table truncation for cleanup
6. **Custom Data**: Use fluent API for test-specific entity data
7. **Integration**: Combine with HTTP fixtures for complete testing

## Writing Custom Fixtures

### Adding New Service Methods

```rust
impl GitHubService {
    /// Mock a new custom endpoint
    pub async fn new_endpoint(
        &self,
        method_name: &str,
        path_pattern: &str,
        status_code: u16,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method(method_name))
            .and(path(path_pattern))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json")
            )
            .mount(&*self.server)
            .await;
        
        self
    }
    
    /// Convenience method for common scenario
    pub async fn setup_custom_scenario(&self) -> &Self {
        self.oauth_token(200, GitHubTokenRequest::valid(), GitHubTokenResponse::success()).await;
        self.user_profile(200, GitHubUserRequest::authenticated(), GitHubUser::arthur()).await;
        self
    }
}
```

### Adding New Resources

```rust
/// New resource data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubNewResource {
    pub field1: String,
    pub field2: i32,
}

impl GitHubNewResource {
    /// Create a builder for custom data
    pub fn create() -> GitHubNewResourceBuilder {
        GitHubNewResourceBuilder::default()
    }
    
    /// Pre-built instance
    pub fn default_instance() -> Self {
        Self {
            field1: "default_value".to_string(),
            field2: 42,
        }
    }
}

/// Builder for new resource
#[derive(Debug, Default)]
pub struct GitHubNewResourceBuilder {
    field1: Option<String>,
    field2: Option<i32>,
}

impl GitHubNewResourceBuilder {
    pub fn field1(mut self, field1: impl Into<String>) -> Self {
        self.field1 = Some(field1.into());
        self
    }
    
    pub fn field2(mut self, field2: i32) -> Self {
        self.field2 = Some(field2);
        self
    }
    
    pub fn build(self) -> GitHubNewResource {
        GitHubNewResource {
            field1: self.field1.unwrap_or_else(|| "default".to_string()),
            field2: self.field2.unwrap_or(0),
        }
    }
}
```

### Adding New Convenience Methods

```rust
impl GitHubService {
    /// Setup a complete custom scenario
    pub async fn setup_custom_flow(&self) -> &Self {
        // Step 1: Mock token exchange
        self.oauth_token(
            200,
            GitHubTokenRequest::valid(),
            GitHubTokenResponse::success()
        ).await;
        
        // Step 2: Mock user profile
        self.user_profile(
            200,
            GitHubUserRequest::authenticated(),
            GitHubUser::arthur()
        ).await;
        
        // Step 3: Mock additional endpoints as needed
        self.custom_endpoint(
            "GET",
            "/user/emails",
            200,
            json!([{"email": "arthur@example.com", "primary": true}])
        ).await;
        
        self
    }
    
    /// Setup error scenario
    pub async fn setup_error_flow(&self) -> &Self {
        self.setup_failed_token_exchange_invalid_code().await;
        self.setup_failed_user_profile_unauthorized().await;
        self
    }
}
```

## Best Practices

### Naming Conventions

1. **Service Methods**: Use endpoint names (`oauth_token`, `user_profile`)
2. **Resources**: Use domain names (`User`, `Token`, `Error`)
3. **Convenience Methods**: Use descriptive names (`setup_successful_*`, `setup_failed_*`)

### Resource Design

1. **Factory Methods**: Provide common instances (`arthur()`, `bob()`)
2. **Builder Pattern**: Use for complex customization (`create().field().build()`)
3. **Validation**: Ensure resources match real API responses

### Error Handling

1. **Comprehensive Coverage**: Mock all possible error scenarios
2. **Realistic Errors**: Use actual error responses from APIs
3. **Status Codes**: Match real HTTP status codes

### Performance

1. **Shared Servers**: Reuse MockServer instances when possible
2. **Cleanup**: Ensure proper cleanup between tests
3. **Parallel Safety**: Design fixtures to be thread-safe

### Maintainability

1. **Documentation**: Document all public methods and resources
2. **Versioning**: Handle API version changes gracefully
3. **Testing**: Test the fixtures themselves

## Examples

### Complete Test Example

```rust
use fixtures::{GitHubFixtures, GitLabFixtures};
use fixtures::github::*;
use fixtures::gitlab::*;

#[tokio::test]
async fn test_multi_provider_oauth() {
    // Setup GitHub mocks
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Setup GitLab mocks  
    let gitlab = GitLabFixtures::service().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_successful_user_profile_alice().await;
    
    // Test both providers...
    println!("🔗 GitHub mock URL: {}", github.base_url());
    println!("🔗 GitLab mock URL: {}", gitlab.base_url());
}
```

### Custom Resource Example

```rust
#[tokio::test]
async fn test_user_with_no_email() {
    let github = GitHubFixtures::service().await;
    
    let user_without_email = GitHubUser::create()
        .id(99999)
        .login("no_email_user")
        .email(None)  // No email provided
        .avatar_url(None::<String>)
        .build();
    
    github
        .user_profile(200,
            GitHubUserRequest::authenticated(),
            user_without_email
        )
        .await;
    
    // Test handling of users without email...
    println!("✅ Custom user data mocked successfully");
}
```

## Quick Start

```rust
mod fixtures;

use fixtures::{GitHubFixtures, GitLabFixtures};
use fixtures::github::*;

#[tokio::test]
async fn test_oauth_flow() {
    // Create service fixture
    let github = GitHubFixtures::service().await;
    
    // Mock successful OAuth flow
    github
        .oauth_token(200, GitHubTokenRequest::valid(), GitHubTokenResponse::success())
        .await;
    
    github
        .user_profile(200, GitHubUserRequest::authenticated(), GitHubUser::arthur())
        .await;
    
    // Your test logic here...
    println!("✅ GitHub OAuth flow mocked at: {}", github.base_url());
}
```

This fixture system provides a robust, maintainable, and developer-friendly approach to mocking external services in your tests. The system is now production-ready and includes comprehensive documentation, examples, and best practices. 