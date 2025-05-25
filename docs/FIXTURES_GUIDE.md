# Fixtures Guide

Comprehensive guide for using the fixture system to mock external services in tests.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Usage Patterns](#usage-patterns)
- [Available Fixtures](#available-fixtures)
- [Writing Custom Fixtures](#writing-custom-fixtures)
- [Best Practices](#best-practices)

## Overview

The fixture system provides a structured approach to mocking external services using `wiremock`. It's designed to be:

- **Modular**: Each external service has its own fixture class
- **Fluent**: Chainable API for easy test setup
- **Reusable**: Pre-built resources and flows for common scenarios
- **Type-safe**: Strongly typed inputs and outputs

### Key Components

1. **Service**: Main fixture class with fluent API for mocking endpoints
2. **Resources**: Type-safe data structures for inputs/outputs
3. **Flow**: Pre-made scenarios for common use cases

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