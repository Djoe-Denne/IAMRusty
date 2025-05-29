# Email/Password Authentication Guide

## Overview

The IAM service provides a complete email/password authentication system alongside OAuth2 authentication. This guide covers the non-OAuth2 authentication flows including user registration, email verification, and password-based login.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Authentication Flows](#authentication-flows)
- [API Endpoints](#api-endpoints)
- [Request/Response Formats](#requestresponse-formats)
- [Security Features](#security-features)
- [Error Handling](#error-handling)
- [Implementation Details](#implementation-details)
- [Testing Guide](#testing-guide)
- [Configuration](#configuration)
- [Examples](#examples)

## Architecture Overview

The email/password authentication system follows the same hexagonal architecture as the OAuth2 flows, ensuring clean separation of concerns and testability.

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Endpoints                           │
│                /auth/signup, /auth/login                    │
│                    /auth/verify                            │
├─────────────────────────────────────────────────────────────┤
│                 Command Handlers                            │
│         SignupCommand, LoginCommand,                        │
│               VerifyEmailCommand                           │
├─────────────────────────────────────────────────────────────┤
│                  Auth Use Case                              │
│              (AuthUseCaseImpl)                             │
├─────────────────────────────────────────────────────────────┤
│                 Domain Services                             │
│         PasswordService, TokenService,                      │
│            EmailVerificationService                         │
├─────────────────────────────────────────────────────────────┤
│                   Repositories                              │
│      UserRepository, UserEmailRepository,                   │
│           EmailVerificationRepository                       │
└─────────────────────────────────────────────────────────────┘
```

### Domain Entities

```rust
// User entity with password support
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: Option<String>, // Some for email/password users
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// User email entity for email management
pub struct UserEmail {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

// Email verification entity
pub struct EmailVerification {
    pub id: Uuid,
    pub email: String,
    pub verification_token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
```

## Authentication Flows

### 1. User Registration (Signup) Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant UseCase
    participant PasswordService
    participant UserRepo
    participant EmailRepo
    participant VerificationRepo
    participant EventPublisher

    Client->>API: POST /auth/signup
    API->>UseCase: signup(request)
    UseCase->>UserRepo: check user exists by email
    UserRepo-->>UseCase: None (user doesn't exist)
    UseCase->>PasswordService: hash_password()
    PasswordService-->>UseCase: password_hash
    UseCase->>UserRepo: create(user)
    UserRepo-->>UseCase: created_user
    UseCase->>EmailRepo: create(user_email)
    EmailRepo-->>UseCase: created_email
    UseCase->>VerificationRepo: create(verification_token)
    VerificationRepo-->>UseCase: OK
    UseCase->>EventPublisher: publish(UserSignedUp)
    EventPublisher-->>UseCase: OK
    UseCase-->>API: SignupResponse
    API-->>Client: 201 Created
```

### 2. Email Verification Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant UseCase
    participant VerificationRepo
    participant EmailRepo
    participant EventPublisher

    Client->>API: POST /auth/verify
    API->>UseCase: verify_email(request)
    UseCase->>VerificationRepo: find_by_email_and_token()
    VerificationRepo-->>UseCase: verification_record
    UseCase->>UseCase: check token expiry
    UseCase->>EmailRepo: find_by_email()
    EmailRepo-->>UseCase: user_email
    UseCase->>EmailRepo: update(verified=true)
    EmailRepo-->>UseCase: updated_email
    UseCase->>VerificationRepo: delete_by_email()
    VerificationRepo-->>UseCase: OK
    UseCase->>EventPublisher: publish(UserEmailVerified)
    EventPublisher-->>UseCase: OK
    UseCase-->>API: VerifyEmailResponse
    API-->>Client: 200 OK
```

### 3. User Login Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant UseCase
    participant EmailRepo
    participant UserRepo
    participant PasswordService
    participant TokenService
    participant EventPublisher

    Client->>API: POST /auth/login
    API->>UseCase: login(request)
    UseCase->>EmailRepo: find_by_email()
    EmailRepo-->>UseCase: user_email
    UseCase->>UseCase: check email verified
    UseCase->>UserRepo: find_by_id()
    UserRepo-->>UseCase: user
    UseCase->>PasswordService: verify_password()
    PasswordService-->>UseCase: is_valid
    UseCase->>TokenService: generate_jwt_for_user()
    TokenService-->>UseCase: jwt_token
    UseCase->>EventPublisher: publish(UserLoggedIn)
    EventPublisher-->>UseCase: OK
    UseCase-->>API: LoginResponse
    API-->>Client: 200 OK
```

## API Endpoints

### POST /auth/signup

Registers a new user with email and password.

**Request Body:**
```json
{
  "username": "alice",
  "email": "alice@example.com",
  "password": "securePassword123"
}
```

**Response (201 Created):**
```json
{
  "message": "User created successfully. Please check your email for verification instructions."
}
```

**Validation Rules:**
- Username: Not empty, trimmed
- Email: Valid email format, not empty
- Password: Minimum 8 characters

### POST /auth/login

Authenticates a user with email and password.

**Request Body:**
```json
{
  "email": "alice@example.com",
  "password": "securePassword123"
}
```

**Response (200 OK):**
```json
{
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "email": "alice@example.com",
    "avatar": null
  },
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### POST /auth/verify

Verifies a user's email address using a verification token.

**Request Body:**
```json
{
  "email": "alice@example.com",
  "verification_token": "abc123def456ghi789"
}
```

**Response (200 OK):**
```json
{
  "message": "Email verified successfully"
}
```

## Security Features

### Password Security

- **Hashing Algorithm**: Argon2 (industry standard, memory-hard function)
- **Salt Generation**: Cryptographically secure random salt per password
- **Hash Verification**: Constant-time comparison to prevent timing attacks

```rust
// Password hashing implementation
impl PasswordService {
    pub fn hash_password(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self.argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed_hash = PasswordHash::new(hash)?;
        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(DomainError::RepositoryError(format!("Verification failed: {}", e)))
        }
    }
}
```

### Email Verification

- **Token Generation**: 32-character alphanumeric tokens
- **Expiration**: 24-hour default expiration (configurable)
- **Single Use**: Tokens are deleted after successful verification
- **Secure Storage**: Tokens stored hashed in database

### JWT Token Security

- **Algorithm**: HS256 (HMAC with SHA-256)
- **Secret Management**: Configurable secret key (minimum 256 bits)
- **Expiration**: Configurable token lifetime (default 1 hour)
- **Claims**: Standard JWT claims (sub, exp, iat, jti)

## Error Handling

### Authentication Errors

```rust
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Email not verified")]
    EmailNotVerified,
    
    #[error("Invalid verification token")]
    InvalidVerificationToken,
    
    #[error("Verification token expired")]
    VerificationTokenExpired,
    
    #[error("Password hashing error: {0}")]
    PasswordHashingError(String),
}
```

### HTTP Error Responses

#### 400 Bad Request
```json
{
  "error": "validation_error",
  "message": "Password must be at least 8 characters long"
}
```

#### 401 Unauthorized
```json
{
  "error": "invalid_credentials",
  "message": "Invalid email or password"
}
```

#### 409 Conflict
```json
{
  "error": "user_already_exists",
  "message": "User with this email already exists"
}
```

## Implementation Details

### Repository Pattern (Read/Write Split)

The system uses separate read and write repositories for better scalability:

```rust
// Read operations
#[async_trait]
pub trait EmailVerificationReadRepository: Send + Sync {
    async fn find_by_email_and_token(&self, email: &str, token: &str) 
        -> Result<Option<EmailVerification>, DomainError>;
}

// Write operations
#[async_trait]
pub trait EmailVerificationWriteRepository: Send + Sync {
    async fn create(&self, verification: &EmailVerification) -> Result<(), DomainError>;
    async fn delete_by_email(&self, email: &str) -> Result<(), DomainError>;
}

// Combined repository for application layer
pub struct CombinedEmailVerificationRepository {
    read_repo: Arc<dyn EmailVerificationReadRepository>,
    write_repo: Arc<dyn EmailVerificationWriteRepository>,
}
```

### Service Adapters

Adapters bridge application interfaces with infrastructure implementations:

```rust
// Application trait
#[async_trait]
pub trait PasswordService: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, AuthError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
}

// Infrastructure adapter
pub struct PasswordServiceAdapter {
    password_service: Arc<infra::PasswordService>,
}

#[async_trait]
impl application::PasswordService for PasswordServiceAdapter {
    async fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        self.password_service
            .hash_password(password)
            .map_err(|e| AuthError::PasswordHashingError(e.to_string()))
    }
}
```

### Factory Pattern

The factory pattern simplifies dependency injection:

```rust
impl AuthFactory {
    pub fn create_auth_use_case<UR, UER, EVR, PS, TS, EP>(
        user_repository: Arc<UR>,
        user_email_repository: Arc<UER>,
        email_verification_repository: Arc<EVR>,
        password_service: Arc<PS>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
    ) -> Arc<dyn AuthUseCase>
    where
        UR: UserRepository + Send + Sync + 'static,
        UER: UserEmailRepository + Send + Sync + 'static,
        EVR: EmailVerificationRepository + Send + Sync + 'static,
        PS: PasswordService + Send + Sync + 'static,
        TS: TokenService + Send + Sync + 'static,
        EP: EventPublisher + Send + Sync + 'static,
    {
        Arc::new(AuthUseCaseImpl::new(
            user_repository,
            user_email_repository,
            email_verification_repository,
            password_service,
            token_service,
            event_publisher,
        ))
    }
}
```

## Testing Guide

### Unit Testing

Test individual components in isolation:

```rust
#[tokio::test]
async fn test_signup_success() {
    let mut mock_user_repo = MockUserRepository::new();
    let mut mock_email_repo = MockUserEmailRepository::new();
    let mut mock_verification_repo = MockEmailVerificationRepository::new();
    let mock_password_service = MockPasswordService::new();
    let mock_token_service = MockTokenService::new();
    let mock_event_publisher = MockEventPublisher::new();

    // Setup expectations
    mock_email_repo
        .expect_find_by_email()
        .returning(|_| Ok(None));
    
    mock_password_service
        .expect_hash_password()
        .returning(|_| Ok("hashed_password".to_string()));

    let auth_use_case = AuthUseCaseImpl::new(
        Arc::new(mock_user_repo),
        Arc::new(mock_email_repo),
        Arc::new(mock_verification_repo),
        Arc::new(mock_password_service),
        Arc::new(mock_token_service),
        Arc::new(mock_event_publisher),
    );

    let request = SignupRequest {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };

    let result = auth_use_case.signup(request).await;
    assert!(result.is_ok());
}
```

### Integration Testing

Test the complete flow with a test database:

```rust
#[tokio::test]
async fn test_signup_login_flow() {
    let test_db = setup_test_database().await;
    let config = load_test_config();
    
    // Initialize services
    let password_service = Arc::new(PasswordService::new());
    let token_service = Arc::new(JwtTokenService::new(
        config.jwt.secret,
        config.jwt.expiration_seconds,
    ));
    
    // Initialize repositories
    let user_read_repo = Arc::new(SeaOrmUserReadRepository::new(test_db.clone()));
    let user_write_repo = Arc::new(SeaOrmUserWriteRepository::new(test_db.clone()));
    // ... other repositories
    
    // Test signup
    let signup_request = SignupRequest {
        username: "integration_test".to_string(),
        email: "integration@test.com".to_string(),
        password: "testpassword123".to_string(),
    };
    
    let signup_result = auth_use_case.signup(signup_request).await;
    assert!(signup_result.is_ok());
    
    // Manually verify email for test
    verify_email_for_test(&test_db, "integration@test.com").await;
    
    // Test login
    let login_request = LoginRequest {
        email: "integration@test.com".to_string(),
        password: "testpassword123".to_string(),
    };
    
    let login_result = auth_use_case.login(login_request).await;
    assert!(login_result.is_ok());
    
    let response = login_result.unwrap();
    assert_eq!(response.user.username, "integration_test");
    assert!(!response.token.is_empty());
}
```

## Configuration

### Environment Variables

```bash
# JWT Configuration
IAM_JWT__SECRET=your-very-secure-256-bit-secret-key-here
IAM_JWT__EXPIRATION_SECONDS=3600

# Database Configuration
IAM_DATABASE__HOST=localhost
IAM_DATABASE__PORT=5432
IAM_DATABASE__DB=iam_development
IAM_DATABASE__CREDS__USERNAME=postgres
IAM_DATABASE__CREDS__PASSWORD=postgres

# Server Configuration
IAM_SERVER__HOST=0.0.0.0
IAM_SERVER__PORT=8080
```

### Configuration File (config.toml)

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
host = "localhost"
port = 5432
db = "iam_development"

[database.creds]
username = "postgres"
password = "postgres"

[jwt]
secret = "your-very-secure-256-bit-secret-key-here"
expiration_seconds = 3600

[logging]
level = "info"
```

## Examples

### Complete Signup and Login Example

```bash
# 1. Sign up a new user
curl -X POST http://localhost:8080/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "securePassword123"
  }'

# Response: 201 Created
# {
#   "message": "User created successfully. Please check your email for verification instructions."
# }

# 2. Verify email (token would normally be sent via email)
curl -X POST http://localhost:8080/auth/verify \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "verification_token": "abc123def456ghi789"
  }'

# Response: 200 OK
# {
#   "message": "Email verified successfully"
# }

# 3. Login
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "securePassword123"
  }'

# Response: 200 OK
# {
#   "user": {
#     "id": "123e4567-e89b-12d3-a456-426614174000",
#     "username": "alice",
#     "email": "alice@example.com",
#     "avatar": null
#   },
#   "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
# }

# 4. Use the JWT token for authenticated requests
curl -X GET http://localhost:8080/me \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### JavaScript/TypeScript Client Example

```typescript
class AuthClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async signup(username: string, email: string, password: string): Promise<{ message: string }> {
    const response = await fetch(`${this.baseUrl}/auth/signup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, email, password }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message);
    }

    return response.json();
  }

  async verifyEmail(email: string, verificationToken: string): Promise<{ message: string }> {
    const response = await fetch(`${this.baseUrl}/auth/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, verification_token: verificationToken }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message);
    }

    return response.json();
  }

  async login(email: string, password: string): Promise<{
    user: { id: string; username: string; email: string; avatar?: string };
    token: string;
  }> {
    const response = await fetch(`${this.baseUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message);
    }

    return response.json();
  }
}

// Usage example
const authClient = new AuthClient('http://localhost:8080');

try {
  // Sign up
  await authClient.signup('alice', 'alice@example.com', 'securePassword123');
  console.log('Signup successful! Check your email for verification.');

  // Verify email (token would come from email)
  await authClient.verifyEmail('alice@example.com', 'verification-token-from-email');
  console.log('Email verified successfully!');

  // Login
  const loginResponse = await authClient.login('alice@example.com', 'securePassword123');
  console.log('Login successful!', loginResponse.user);
  
  // Store the JWT token for future requests
  localStorage.setItem('authToken', loginResponse.token);
} catch (error) {
  console.error('Authentication error:', error.message);
}
```

## Best Practices

### Security Best Practices

1. **Password Requirements**: Enforce strong password policies
2. **Rate Limiting**: Implement rate limiting on authentication endpoints
3. **Account Lockout**: Lock accounts after multiple failed login attempts
4. **Email Verification**: Always require email verification before allowing login
5. **JWT Security**: Use strong secrets and appropriate token lifetimes
6. **HTTPS Only**: Always use HTTPS in production
7. **Input Validation**: Validate and sanitize all user inputs

### Error Handling Best Practices

1. **Don't Leak Information**: Never reveal whether a user exists in error messages
2. **Consistent Timing**: Use constant-time operations to prevent timing attacks
3. **Graceful Degradation**: Handle service failures gracefully
4. **Comprehensive Logging**: Log security-relevant events for monitoring

### Testing Best Practices

1. **Test All Flows**: Cover success paths, error paths, and edge cases
2. **Mock External Services**: Use mocks for email services and databases in unit tests
3. **Integration Tests**: Test complete flows with real database interactions
4. **Security Tests**: Test for common security vulnerabilities
5. **Performance Tests**: Ensure authentication flows perform well under load

## Troubleshooting

### Common Issues

#### "User already exists" error during signup
- Check if email is already registered
- Verify email uniqueness constraints in database

#### "Email not verified" error during login
- Ensure email verification flow is completed
- Check verification token expiration
- Verify email verification repository implementation

#### JWT token issues
- Verify JWT secret configuration
- Check token expiration settings
- Ensure token service is properly configured

#### Password hashing errors
- Verify Argon2 dependencies are correctly installed
- Check system memory and CPU constraints
- Ensure proper error handling in password service

### Debugging Steps

1. **Check Logs**: Review application logs for detailed error information
2. **Database State**: Verify user and email records in database
3. **Configuration**: Ensure all required configuration values are set
4. **Dependencies**: Verify all required dependencies are installed and compatible
5. **Integration Tests**: Run integration tests to verify component interactions

## Related Documentation

- [Architecture Guide](ARCHITECTURE.md) - Overall system architecture
- [API Reference](API_REFERENCE.md) - Complete API documentation
- [Testing Guide](TESTING_GUIDE.md) - Comprehensive testing strategies
- [Error Handling Guide](ERROR_HANDLING_GUIDE.md) - Error handling patterns
- [Command Pattern Guide](COMMAND_PATTERN.md) - Command pattern implementation
- [Database Configuration](DATABASE_CONFIGURATION.md) - Database setup and configuration 