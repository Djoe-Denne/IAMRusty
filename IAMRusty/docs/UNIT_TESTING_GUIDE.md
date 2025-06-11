# Unit Testing Guide

## 📚 Overview

This guide covers unit testing strategy and implementation for the IAM service, focusing on the domain layer business logic. Unit tests provide fast, isolated testing of individual components without external dependencies.

## 🎯 Testing Philosophy

### Unit Testing Principles

1. **Fast Execution**: Unit tests run in milliseconds without external dependencies
2. **Isolation**: Each test is independent and doesn't affect others
3. **Deterministic**: Tests produce consistent results across runs
4. **Focused**: Each test validates a single behavior or scenario
5. **Maintainable**: Tests are easy to read, understand, and modify

### Testing Strategy

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Unit Tests    │    │ Integration     │    │   E2E Tests     │
│                 │    │    Tests        │    │                 │
│ • Domain Logic  │    │ • API Routes    │    │ • Full Workflow │
│ • Services      │    │ • Database      │    │ • User Journey  │
│ • Fast & Isolated  │    │ • HTTP Layer    │    │ • Real Systems  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
      ⬆️ This Guide           Other Guides          Other Guides
```

## 🧰 Testing Tools & Libraries

### Core Testing Dependencies

```toml
[dev-dependencies]
# Async runtime with test utilities
tokio = { version = "1.44", features = ["rt-multi-thread", "macros", "test-util"] }

# Mock generation and verification
mockall = "0.13.0"

# Parameterized tests and fixtures
rstest = "0.23.0" 

# Async testing utilities
tokio-test = "0.4.4"

# Better assertions
claims = "0.7.1"
```

### Library Purposes

| Library | Purpose | Usage |
|---------|---------|-------|
| `mockall` | Mock generation for traits | Mocking repositories and external services |
| `rstest` | Test fixtures and parameterization | Reusable test data and parameterized tests |
| `tokio-test` | Async testing utilities | Testing async functions and streams |
| `claims` | Enhanced assertions | `assert_ok!`, `assert_err!`, cleaner assertions |

## 🏗️ Architecture Under Test

### Domain Services

Our unit tests focus on the domain layer services:

```rust
// Domain Services (Business Logic)
┌─────────────────────────────────┐
│         AuthService             │
│  ┌─────────────────────────┐   │
│  │ • generate_authorize_url │   │
│  │ • process_callback      │   │  
│  │ • find_user_by_id       │   │
│  │ • get_provider_token    │   │
│  └─────────────────────────┘   │
└─────────────────────────────────┘

┌─────────────────────────────────┐
│        TokenService             │
│  ┌─────────────────────────┐   │
│  │ • generate_token        │   │
│  │ • validate_token        │   │
│  │ • jwks                  │   │
│  └─────────────────────────┘   │
└─────────────────────────────────┘
```

### Dependency Injection Pattern

Services use dependency injection for testability:

```rust
pub struct AuthService<U, T> 
where
    U: UserRepository,
    T: TokenRepository,
{
    user_repository: U,
    token_repository: T,
    token_service: TokenService,
    provider_clients: HashMap<Provider, Box<dyn ProviderOAuth2Client + Send + Sync>>,
}
```

This design enables:
- **Mock Injection**: Replace real repositories with mocks
- **Isolated Testing**: Test business logic without database
- **Fast Execution**: No I/O operations during testing

## 🧪 Test Implementation Examples

### Mock Setup Pattern

```rust
// Manual mock implementation for better control
#[derive(Default)]
struct MockUserRepo {
    find_by_id_responses: HashMap<Uuid, Result<Option<User>, TestError>>,
    find_by_email_responses: HashMap<String, Result<Option<User>, TestError>>,
    create_responses: Vec<Result<User, TestError>>,
}

impl MockUserRepo {
    fn expect_find_by_id(&mut self, id: Uuid, response: Result<Option<User>, TestError>) {
        self.find_by_id_responses.insert(id, response);
    }
}

#[async_trait::async_trait]
impl UserReadRepository for MockUserRepo {
    type Error = TestError;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
        self.find_by_id_responses.get(&id).cloned().unwrap_or(Ok(None))
    }
}
```

### Fixture-Based Testing

```rust
use rstest::*;

// Reusable test data
#[fixture]
fn sample_user() -> User {
    User {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        username: "testuser".to_string(),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[fixture]
fn sample_provider_tokens() -> ProviderTokens {
    ProviderTokens {
        access_token: "github_access_token".to_string(),
        refresh_token: Some("github_refresh_token".to_string()),
        expires_in: Some(3600),
    }
}

// Test using fixtures
#[rstest]
#[tokio::test]
async fn success_with_existing_user(
    sample_user: User,
    sample_provider_tokens: ProviderTokens,
) {
    // Test implementation using fixtures
}
```

### Parameterized Testing

```rust
#[rstest]
#[case("github", Provider::GitHub)]
#[case("gitlab", Provider::GitLab)]
#[test]
fn success_with_valid_provider(#[case] provider_str: &str, #[case] provider: Provider) {
    // Test runs for each case
    let mut auth_service = auth_service();
    let mut mock_client = MockOAuth2Client::new();
    
    mock_client
        .expect_generate_authorize_url()
        .times(1)
        .returning(|| "https://github.com/login/oauth/authorize?client_id=test".to_string());

    auth_service.register_provider_client(provider, Box::new(mock_client));
    let result = auth_service.generate_authorize_url(provider_str);

    assert_ok!(&result);
}
```

### Error Testing Patterns

```rust
#[tokio::test]
async fn error_when_profile_missing_email(sample_provider_tokens: ProviderTokens) {
    let mut auth_service = auth_service();
    let provider = Provider::GitHub;
    
    // Create invalid scenario
    let mut profile_without_email = sample_provider_profile();
    profile_without_email.email = None;

    // Setup mocks for error case
    let mut mock_client = MockOAuth2Client::new();
    mock_client
        .expect_exchange_code()
        .times(1)
        .returning(move |_| Ok(sample_provider_tokens.clone()));
    
    mock_client
        .expect_get_user_profile()
        .times(1)
        .returning(move |_| Ok(profile_without_email.clone()));

    auth_service.register_provider_client(provider, Box::new(mock_client));

    // Test error scenario
    let result = auth_service.process_callback("github", "auth_code").await;

    assert_err!(&result);
    match result.unwrap_err() {
        DomainError::UserProfileError(msg) => {
            assert!(msg.contains("Email is required"));
        }
        _ => panic!("Expected UserProfileError"),
    }
}
```

## 📂 Test Organization

### Module Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Test imports and setup

    // Organized test modules
    mod auth_service_creation {
        use super::*;
        
        #[test]
        fn new_creates_auth_service_with_empty_provider_clients() { /* */ }
        
        #[test] 
        fn register_provider_client_adds_client_to_map() { /* */ }
    }

    mod generate_authorize_url {
        use super::*;
        
        #[rstest]
        #[case("github", Provider::GitHub)]
        #[case("gitlab", Provider::GitLab)]
        #[test]
        fn success_with_valid_provider(/* */) { /* */ }
        
        #[test]
        fn error_with_unsupported_provider() { /* */ }
    }

    mod process_callback {
        use super::*;
        
        #[rstest]
        #[tokio::test]
        async fn success_with_existing_user(/* */) { /* */ }
        
        #[tokio::test]
        async fn error_with_unsupported_provider() { /* */ }
    }
}
```

### Test Categories

| Category | Purpose | Examples |
|----------|---------|----------|
| **Creation Tests** | Service instantiation | Constructor validation, initial state |
| **Success Path Tests** | Happy path scenarios | Valid inputs produce expected outputs |
| **Error Handling Tests** | Failure scenarios | Invalid inputs, external service failures |
| **Edge Case Tests** | Boundary conditions | Empty strings, null values, limits |
| **Integration Workflow Tests** | Multi-step processes | Token generation → validation workflow |

## 🚀 Running Unit Tests

### Command Reference

```bash
# Run all unit tests (fast, no external dependencies)
just unit-test

# Run unit tests with cargo directly
cargo test --lib

# Run specific test module
cargo test --lib auth_service::tests::generate_authorize_url

# Run with output capture
cargo test --lib -- --nocapture

# Run single test
cargo test --lib test_generate_authorize_url_success
```

### Test Output

```
🧪 Running unit tests...
   Testing domain business logic (auth_service, token_service)
     Running unittests src\lib.rs

running 41 tests
test service::auth_service::tests::auth_service_creation::new_creates_auth_service_with_empty_provider_clients ... ok
test service::auth_service::tests::generate_authorize_url::success_with_valid_provider::case_1 ... ok
test service::auth_service::tests::process_callback::success_with_existing_user ... ok
test service::token_service::tests::generate_token::success_with_valid_inputs ... ok

test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
✅ Unit tests completed
```

## 🔧 Best Practices

### Mock Design Patterns

#### 1. Manual Mocks for Complex Logic

```rust
// Better control over mock behavior
struct MockUserRepo {
    responses: HashMap<String, Result<User, Error>>,
}

impl MockUserRepo {
    fn expect_call(&mut self, input: String, output: Result<User, Error>) {
        self.responses.insert(input, output);
    }
}
```

#### 2. Mockall for Simple Traits

```rust
// Auto-generated mocks for straightforward cases
mock! {
    OAuth2Client {}

    #[async_trait::async_trait]
    impl ProviderOAuth2Client for OAuth2Client {
        fn generate_authorize_url(&self) -> String;
        async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError>;
    }
}
```

### Assertion Strategies

#### 1. Use Claims for Better Assertions

```rust
use claims::*;

// Instead of
assert!(result.is_ok());
let user = result.unwrap();

// Use
assert_ok!(&result);
let user = result.unwrap();

// Or
let user = assert_ok!(result);
```

#### 2. Detailed Error Matching

```rust
// Specific error validation
match result.unwrap_err() {
    DomainError::UserProfileError(msg) => {
        assert!(msg.contains("Email is required"));
    }
    _ => panic!("Expected UserProfileError"),
}
```

### Test Data Management

#### 1. Fixture Reuse

```rust
#[fixture]
fn auth_service() -> AuthService<MockUserRepo, MockTokenRepo> {
    let user_repo = MockUserRepo::new();
    let token_repo = MockTokenRepo::new();
    let token_service = TokenService::new(
        Box::new(MockTokenEnc::new()), 
        ChronoDuration::hours(1)
    );
    
    AuthService::new(user_repo, token_repo, token_service)
}
```

#### 2. Builder Pattern for Complex Data

```rust
struct UserBuilder {
    user: User,
}

impl UserBuilder {
    fn new() -> Self {
        Self {
            user: User::default(),
        }
    }
    
    fn with_email(mut self, email: &str) -> Self {
        self.user.email = Some(email.to_string());
        self
    }
    
    fn build(self) -> User {
        self.user
    }
}

// Usage in tests
let user = UserBuilder::new()
    .with_email("test@example.com")
    .build();
```

### Async Testing Patterns

#### 1. Tokio Test Attribute

```rust
#[tokio::test]
async fn async_test_example() {
    let result = some_async_function().await;
    assert_ok!(result);
}
```

#### 2. Async Mock Returns

```rust
mock_client
    .expect_exchange_code()
    .times(1)
    .returning(|_| async { Ok(tokens) }.boxed()); // For complex async returns
```

## 📊 Test Coverage & Metrics

### Current Test Coverage

| Service | Methods Tested | Test Count | Coverage |
|---------|---------------|------------|----------|
| `AuthService` | 6/6 | 22 tests | 100% |
| `TokenService` | 3/3 | 19 tests | 100% |
| **Total** | **9/9** | **41 tests** | **100%** |

### Test Categories Breakdown

```
AuthService Tests (22):
├── Creation & Setup (2)
├── Authorization URL Generation (3) 
├── OAuth Callback Processing (5)
├── User Lookup (4)
├── Provider Token Retrieval (4)
├── Helper Methods (2)
└── Error Scenarios (2)

TokenService Tests (19):
├── Service Creation (2)
├── Token Generation (4)
├── Token Validation (5)
├── JWKS Operations (2)
├── Integration Workflows (1)
└── Edge Cases (5)
```

## 🐛 Common Issues & Solutions

### Issue 1: Mock Borrow Checker Errors

**Problem**: Moving values into closures and borrowing them later

```rust
// ❌ Fails - moved value
.withf(move |claims| claims.sub == user_id)
let result = service.generate_token(&user_id, &username); // Error: value moved
```

**Solution**: Clone before moving

```rust
// ✅ Works - clone before move
let user_id_clone = user_id.clone();
.withf(move |claims| claims.sub == user_id_clone)
let result = service.generate_token(&user_id, &username);
```

### Issue 2: Debug Trait Requirements

**Problem**: Trait objects don't implement Debug for error assertions

```rust
// ❌ Fails - Debug not implemented
assert_err!(&result);
```

**Solution**: Use pattern matching

```rust
// ✅ Works - manual error checking
assert!(result.is_err());
if let Err(error) = result {
    // Handle specific error
}
```

### Issue 3: Async Trait Mock Ordering

**Problem**: Incorrect macro ordering for async traits

```rust
// ❌ Wrong order
#[async_trait]
#[cfg_attr(test, automock)]
trait MyTrait { /* */ }
```

**Solution**: Correct macro ordering

```rust
// ✅ Correct order
#[cfg_attr(test, automock)]
#[async_trait]
trait MyTrait { /* */ }
```

## 📈 Extending Unit Tests

### Adding New Service Tests

1. **Create test module structure**:
   ```rust
   mod new_service {
       mod service_creation { /* */ }
       mod method_tests { /* */ }
       mod error_scenarios { /* */ }
   }
   ```

2. **Define fixtures for test data**:
   ```rust
   #[fixture]
   fn sample_entity() -> Entity { /* */ }
   ```

3. **Implement mocks for dependencies**:
   ```rust
   struct MockDependency { /* */ }
   impl DependencyTrait for MockDependency { /* */ }
   ```

4. **Write tests for each public method**:
   - Success scenarios
   - Error conditions  
   - Edge cases
   - Boundary values

### Testing Guidelines

1. **Test Naming**: Use descriptive names indicating the scenario
   ```rust
   fn should_return_error_when_user_not_found()
   fn should_generate_token_with_valid_claims()
   ```

2. **Arrange-Act-Assert Pattern**:
   ```rust
   #[test]
   fn test_example() {
       // Arrange
       let service = create_service();
       let input = create_input();
       
       // Act
       let result = service.method(input);
       
       // Assert
       assert_ok!(result);
   }
   ```

3. **Test Independence**: Each test should be self-contained
4. **Clear Assertions**: Use specific assertions that clearly indicate what failed

## 🔗 Related Documentation

- [Testing Guide](TESTING_GUIDE.md) - Complete testing strategy
- [Fixtures Guide](FIXTURES_GUIDE.md) - Test fixture patterns
- [Error Handling Guide](ERROR_HANDLING_GUIDE.md) - Error testing strategies
- [Architecture Guide](ARCHITECTURE.md) - System design patterns

## 🎯 Summary

Unit tests provide the foundation for reliable software by testing business logic in isolation. Key takeaways:

- **Fast Feedback**: Unit tests run in milliseconds
- **High Coverage**: 100% coverage of domain services  
- **Modern Tools**: Leveraging `mockall`, `rstest`, and `claims`
- **Maintainable**: Well-organized, documented test patterns
- **Reliable**: Consistent, deterministic test results

The unit test suite ensures that domain business logic works correctly regardless of external dependencies, providing confidence for refactoring and feature development. 