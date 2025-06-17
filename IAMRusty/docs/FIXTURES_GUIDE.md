# Fixtures Guide

Comprehensive guide for using the fixture system to mock external services and create test data in the IAM system.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [HTTP Service Fixtures](#http-service-fixtures)
- [Database Fixtures](#database-fixtures)
- [Usage Patterns](#usage-patterns)
- [Available Fixtures](#available-fixtures)
- [Best Practices](#best-practices)

## Overview

The fixture system provides structured mocking for external services and database entities using `wiremock` and builder patterns. Key features:

- **HTTP Service Mocking**: GitHub and GitLab OAuth endpoints
- **Database Entity Creation**: Users, emails, tokens, and related entities
- **Automatic Cleanup**: Mocks and database state reset between tests
- **Type Safety**: Strongly typed builders and factory methods
- **Fluent API**: Chainable methods for easy test setup

### Key Components

- **Service Fixtures**: Mock external HTTP APIs (`GitHubFixtures`, `GitLabFixtures`)
- **Database Fixtures**: Create and manage test entities (`DbFixtures`)
- **Common Utilities**: Shared mock server management
- **Builder Patterns**: Fluent APIs for entity creation

## Architecture

### Directory Structure

```
tests/fixtures/
├── mod.rs                    # Main fixture exports
├── github/
│   ├── mod.rs               # GitHub fixture exports
│   ├── service.rs           # GitHubService mock endpoints
│   └── resources.rs         # GitHub data structures
├── gitlab/
│   ├── mod.rs               # GitLab fixture exports
│   ├── service.rs           # GitLabService mock endpoints
│   └── resources.rs         # GitLab data structures
├── db/
│   ├── mod.rs               # Database fixture exports and helpers
│   ├── common.rs            # Shared traits and utilities
│   ├── users.rs             # User entity fixtures
│   ├── user_emails.rs       # UserEmail entity fixtures
│   ├── provider_tokens.rs   # ProviderToken entity fixtures
│   ├── refresh_tokens.rs    # RefreshToken entity fixtures
│   ├── email_verification.rs # EmailVerification entity fixtures
│   └── password_reset_tokens.rs # PasswordResetToken entity fixtures
└── common/
    ├── mod.rs               # Common utilities
    └── wiremock_server.rs   # Shared wiremock server management
```

### Fixture Exports

Available through `tests/fixtures/mod.rs`:
- `GitHubFixtures` - GitHub OAuth service mocking
- `GitLabFixtures` - GitLab OAuth service mocking  
- `DbFixtures` - Database entity creation

## HTTP Service Fixtures

### GitHub Fixtures

**Implementation**: `tests/fixtures/github/service.rs`

The `GitHubService` provides mocking for GitHub OAuth endpoints with automatic cleanup through the `MockServerFixture`. Each service instance creates its own mock server that's automatically cleaned up when the service is dropped.

**Key Methods** (see `tests/fixtures/github/service.rs` for full implementation):
- `oauth_token()` - Mock OAuth token exchange endpoint
- `user_profile()` - Mock user profile fetch endpoint  
- `user_emails()` - Mock user emails endpoint
- `oauth_authorize()` - Mock OAuth authorization endpoint
- Convenience methods: `setup_successful_token_exchange()`, `setup_successful_user_profile_arthur()`, etc.

**Resources** (see `tests/fixtures/github/resources.rs`):
- `GitHubUser` with factory methods (`arthur()`, `bob()`)
- `GitHubTokenRequest/Response` for OAuth flows
- `GitHubError` for error scenarios

### GitLab Fixtures

**Implementation**: `tests/fixtures/gitlab/service.rs`

Similar structure to GitHub with GitLab-specific endpoints and data structures.

**Example Usage in Tests**: See `tests/auth_oauth_start.rs` and `tests/auth_oauth_callback.rs` for complete examples.

## Database Fixtures

### Overview

Database fixtures provide a fluent API for creating test entities with automatic cleanup via table truncation between tests.

**Implementation**: `tests/fixtures/db/mod.rs` with entity-specific builders in separate files.

### Key Features

- **Fluent Builder API**: Chainable methods for entity creation
- **Factory Methods**: Pre-built entities for common scenarios (`arthur()`, `bob()`, `alice()`)
- **Validation**: `check()` methods to verify entity state against database
- **Helper Methods**: High-level methods for common test scenarios
- **Automatic Cleanup**: Table truncation between tests ensures isolation

### Helper Methods

The `DbFixtures` struct provides high-level helpers for common scenarios (see `tests/fixtures/db/mod.rs`):

- `create_user_with_email_password()` - Complete user with email/password auth
- `create_user_without_username()` - User without username (for registration flow)
- `create_user_with_oauth_provider()` - User with OAuth provider setup

### Available Entity Fixtures

| Entity | File | Key Builder Methods |
|--------|------|---------------------|
| User | `users.rs` | `username()`, `password_hash()`, `avatar_url()` |
| UserEmail | `user_emails.rs` | `email()`, `is_primary()`, `is_verified()` |
| ProviderToken | `provider_tokens.rs` | `provider()`, `access_token()`, `refresh_token()` |
| RefreshToken | `refresh_tokens.rs` | `token()`, `is_valid()`, `expires_at()` |
| EmailVerification | `email_verification.rs` | `token()`, `expires_at()`, `verified()` |
| PasswordResetToken | `password_reset_tokens.rs` | `token()`, `expires_at()`, `used()` |

Each entity also provides factory methods for common test scenarios.

## Usage Patterns

### Basic Test Structure

**Pattern**: All integration tests follow this structure (see `tests/auth_oauth_start.rs`):

1. Setup test server and database with `setup_test_server()`
2. Create external service fixtures if needed
3. Create test data with database fixtures if needed
4. Execute test operations
5. Verify results (automatic cleanup happens)

### HTTP Service Mocking

**Example**: See `tests/auth_oauth_start.rs` for OAuth flow testing:

```rust
let _github = GitHubFixtures::service().await;
```

The service automatically mocks GitHub endpoints for the duration of the test.

### Database Entity Creation

**Basic Usage**:
```rust
let user = DbFixtures::user().arthur().commit(db.clone()).await?;
```

**Custom Data**:
```rust
let user = DbFixtures::user()
    .username("custom_user")
    .avatar_url(Some("https://example.com/avatar.png".to_string()))
    .commit(db.clone()).await?;
```

**Relationships**:
```rust
let user = DbFixtures::user().arthur().commit(db.clone()).await?;
let email = DbFixtures::user_email()
    .arthur_primary(user.id())
    .commit(db.clone()).await?;
```

### Integration Testing

**Complete Pattern**: See `tests/signup_kafka.rs` for full integration example combining:
- Database fixtures for test data
- HTTP service fixtures for external APIs
- Message queue testing for event verification

## Available Fixtures

### Service Fixtures

| Fixture | File | Purpose |
|---------|------|---------|
| `GitHubFixtures::service()` | `github/service.rs` | GitHub OAuth endpoint mocking |
| `GitLabFixtures::service()` | `gitlab/service.rs` | GitLab OAuth endpoint mocking |

### Database Fixtures

Access all through `DbFixtures`:

| Method | Returns | Purpose |
|--------|---------|---------|
| `user()` | `UserFixtureBuilder` | Create users with optional username, password, avatar |
| `user_email()` | `UserEmailFixtureBuilder` | Create user emails with primary/verified flags |
| `provider_token()` | `ProviderTokenFixtureBuilder` | Create OAuth provider tokens |
| `refresh_token()` | `RefreshTokenFixtureBuilder` | Create refresh tokens |
| `email_verification()` | `EmailVerificationFixtureBuilder` | Create email verification tokens |
| `password_reset_token()` | `PasswordResetTokenFixtureBuilder` | Create password reset tokens |

Each builder provides both custom property methods and factory methods for common scenarios.

### Factory Methods Available

Common factory methods across entities:
- **Users**: `arthur()`, `bob()`, `alice()`, `charlie()`, `no_avatar()`
- **Emails**: `arthur_primary()`, `bob_primary()`, `alice_primary()`, etc.
- **Tokens**: `github()`, `gitlab()`, `valid()`, `expired()`, `invalid()`

See individual entity files in `tests/fixtures/db/` for complete lists.

## Best Practices

### Test Organization

1. **Use Serial Tests**: Always use `#[serial]` for integration tests
2. **Scope Fixtures**: Create fixtures within test scope for automatic cleanup  
3. **Reference Examples**: See existing test files for established patterns

### Resource Management

1. **Automatic Cleanup**: Fixtures clean up automatically via Drop traits
2. **Database Isolation**: Table truncation ensures clean state between tests
3. **Mock Isolation**: Each test gets fresh mock servers

### Factory vs Custom Data

1. **Use Factories**: Prefer factory methods (`arthur()`, `bob()`) for common scenarios
2. **Custom When Needed**: Use fluent API for test-specific requirements
3. **Relationships**: Create parent entities before children

### Error Handling

1. **Comprehensive Testing**: Test both success and error scenarios
2. **Realistic Errors**: Use actual error responses from APIs
3. **Fixture Validation**: Use `check()` methods to verify entity state

### Example References

For complete, working examples:

- **OAuth Flow Testing**: `tests/auth_oauth_start.rs`, `tests/auth_oauth_callback.rs`
- **Database Entity Testing**: `tests/user.rs`, `tests/token.rs`
- **Integration Testing**: `tests/signup_kafka.rs`, `tests/signup_sqs.rs`
- **Authentication Flows**: `tests/auth_username_flow.rs`, `tests/auth_email_password.rs`
- **Registration**: `tests/auth_complete_registration.rs`

The fixture system provides a robust foundation for testing external integrations and database operations while maintaining test isolation and developer productivity. 