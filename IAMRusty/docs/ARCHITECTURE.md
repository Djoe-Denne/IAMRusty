# IAM Service Architecture

## Overview

The IAM (Identity and Access Management) service is built using **Hexagonal Architecture** (also known as Ports and Adapters) combined with **Domain-Driven Design (DDD)** principles. This architecture ensures clean separation of concerns, testability, and maintainability while keeping the business logic independent of external frameworks and infrastructure.

## Table of Contents

- [Architectural Principles](#architectural-principles)
- [Layer Structure](#layer-structure)
- [Domain Layer](#domain-layer)
- [Application Layer](#application-layer)
- [Infrastructure Layer](#infrastructure-layer)
- [HTTP Layer](#http-layer)
- [Configuration Layer](#configuration-layer)
- [Setup Layer](#setup-layer)
- [Dependency Flow](#dependency-flow)
- [Key Design Patterns](#key-design-patterns)
- [Benefits](#benefits)
- [Migration Journey](#migration-journey)

## Architectural Principles

### Hexagonal Architecture (Ports and Adapters)

The hexagonal architecture isolates the core business logic from external concerns by defining clear boundaries:

- **Inside the Hexagon**: Pure business logic (Domain + Application layers)
- **Outside the Hexagon**: External adapters (Infrastructure, HTTP, Database, etc.)
- **Ports**: Interfaces that define contracts between inside and outside
- **Adapters**: Implementations that connect external systems to ports

### Domain-Driven Design (DDD)

DDD principles guide our domain modeling:

- **Ubiquitous Language**: Consistent terminology across code and business
- **Bounded Contexts**: Clear boundaries around related concepts
- **Entities**: Objects with identity and lifecycle
- **Value Objects**: Immutable objects defined by their attributes
- **Domain Services**: Business logic that doesn't belong to entities
- **Repository Pattern**: Abstraction for data persistence

## Layer Structure

```
┌─────────────────────────────────────────────────────────────┐
│                        HTTP Layer                           │
│                    (Axum Web Server)                        │
├─────────────────────────────────────────────────────────────┤
│                    Application Layer                        │
│                   (Use Cases/Orchestration)                 │
├─────────────────────────────────────────────────────────────┤
│                      Domain Layer                           │
│                  (Business Logic/Entities)                  │
├─────────────────────────────────────────────────────────────┤
│                   Infrastructure Layer                      │
│              (Database/External Services)                   │
└─────────────────────────────────────────────────────────────┘
```

### Crate Organization

```
iam-service/
├── domain/           # Core business logic
├── application/      # Use cases and orchestration
├── infra/           # Infrastructure adapters
├── http/            # HTTP/REST API layer
├── configuration/   # Configuration management
├── setup/           # Application bootstrap
└── migration/       # Database migrations
```

## Domain Layer

**Location**: `domain/` crate  
**Purpose**: Contains pure business logic, free from external dependencies

### Structure

```
domain/
├── src/
│   ├── entity/      # Domain entities and value objects
│   ├── service/     # Domain services
│   ├── port/        # Interfaces (ports)
│   └── error.rs     # Domain-specific errors
```

### Key Components

#### Entities
- **User**: Core user entity with identity and business rules
- **Provider**: OAuth provider enumeration (GitHub, GitLab)
- **ProviderTokens**: OAuth tokens from external providers
- **JwtToken**: JWT token representation
- **RefreshToken**: Refresh token for token renewal

#### Domain Services
- **AuthService**: Handles OAuth authentication flow
- **TokenService**: Manages JWT token operations

#### Ports (Interfaces)
- **UserRepository**: User data persistence contract
- **TokenRepository**: Token storage contract
- **ProviderOAuth2Client**: OAuth provider integration contract
- **JwtTokenEncoder**: JWT encoding/decoding contract

#### Domain Errors
All domain operations return `DomainError` types:
- `UserNotFound`
- `ProviderNotSupported`
- `TokenExpired`
- `InvalidToken`
- `RepositoryError`

### Example: Domain Service

```rust
impl<U, T> AuthService<U, T>
where
    U: UserRepository,
    T: TokenRepository,
{
    pub async fn process_callback(
        &self,
        provider_name: &str,
        code: &str,
    ) -> Result<(User, String), DomainError> {
        // Pure business logic - no infrastructure concerns
        let provider = Provider::from_str(provider_name)?;
        let client = self.get_provider_client(provider)?;
        let tokens = client.exchange_code(code).await?;
        let profile = client.get_user_profile(&tokens).await?;
        let user = self.find_or_create_user(provider, profile).await?;
        let jwt_token = self.token_service.generate_token(&user.id.to_string(), &user.username)?;
        
        Ok((user, jwt_token))
    }
}
```

## Application Layer

**Location**: `application/` crate  
**Purpose**: Orchestrates domain services and handles use cases

### Structure

```
application/
├── src/
│   ├── usecase/     # Use case implementations
│   ├── dto/         # Data Transfer Objects
│   ├── auth.rs      # Authentication traits
│   └── error.rs     # Application-specific errors
```

### Key Components

#### Use Cases
- **LoginUseCase**: Handles user login flow
- **UserUseCase**: User management operations
- **TokenUseCase**: Token refresh and revocation
- **LinkProviderUseCase**: Link OAuth providers to users

#### DTOs (Data Transfer Objects)
- **AuthResponseDto**: Authentication response
- **UserProfileDto**: User profile information
- **ProviderTokenResponseDto**: Provider token response

#### Error Mapping
Application layer maps domain errors to application errors:

```rust
pub enum AuthUseCaseError {
    Domain(#[from] DomainError),
    Application(#[from] ApplicationError),
}
```

### Example: Use Case Implementation

```rust
impl<U, T> AuthUseCase for AuthUseCaseImpl<U, T> {
    async fn process_callback(
        &self,
        provider: &str,
        code: &str,
    ) -> Result<AuthResponseDto, AuthUseCaseError> {
        // Orchestrate domain services
        let (user, token) = self.auth_service
            .process_callback(provider, code)
            .await
            .map_err(AuthUseCaseError::Domain)?;

        // Convert to DTO
        let response = AuthResponseDto {
            token,
            user: UserProfileDto::from(user),
        };

        Ok(response)
    }
}
```

## Infrastructure Layer

**Location**: `infra/` crate  
**Purpose**: Implements domain ports with external systems

### Structure

```
infra/
├── src/
│   ├── auth/        # OAuth provider implementations
│   ├── repository/  # Database repository implementations
│   ├── token/       # JWT token service implementation
│   ├── db/          # Database connection management
│   └── config/      # Configuration loading utilities
```

### Key Adapters

#### Repository Implementations
- **UserRepositoryImpl**: PostgreSQL user storage
- **TokenRepositoryImpl**: PostgreSQL token storage
- **RefreshTokenRepositoryImpl**: Refresh token storage

#### OAuth Client Implementations
- **GitHubOAuth2Client**: GitHub OAuth integration
- **GitLabOAuth2Client**: GitLab OAuth integration

#### Token Service Implementation
- **JwtTokenService**: JWT encoding/decoding with jsonwebtoken

### Example: Repository Implementation

```rust
#[async_trait]
impl UserRepository for CombinedUserRepository {
    type Error = RepositoryError;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
        // Database-specific implementation
        let user_model = self.read_repo.find_by_id(id).await?;
        Ok(user_model.map(|model| model.into()))
    }

    async fn create(&self, user: User) -> Result<User, Self::Error> {
        // Database-specific implementation
        let user_model = UserModel::from(user);
        let created = self.write_repo.create(user_model).await?;
        Ok(created.into())
    }
}
```

## HTTP Layer

**Location**: `http/` crate  
**Purpose**: Provides REST API endpoints

### Structure

```
http/
├── src/
│   ├── handlers/    # HTTP request handlers
│   ├── middleware_auth.rs  # Authentication middleware
│   └── oauth_state.rs      # OAuth state management
```

### Key Components

#### Handlers
- **auth**: OAuth authentication endpoints
- **user**: User profile endpoints
- **token**: Token refresh endpoints

#### Middleware
- **auth**: JWT token validation middleware

### Example: HTTP Handler

```rust
pub async fn oauth_callback(
    Path(provider): Path<String>,
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Result<Json<AuthResponseDto>, ApiError> {
    // Delegate to use case
    let response = state
        .login_usecase
        .login(
            Provider::from_str(&provider)?,
            params.code,
            get_redirect_uri(&state.oauth_config, &provider),
        )
        .await?;

    Ok(Json(AuthResponseDto {
        token: response.access_token,
        user: UserProfileDto {
            id: response.user.id,
            username: response.user.username,
            avatar_url: response.user.avatar_url,
        },
    }))
}
```

## Configuration Layer

**Location**: `configuration/` crate  
**Purpose**: Centralized configuration management with environment-specific settings

### Structure

```
configuration/
├── src/
│   └── lib.rs       # All configuration types and utilities
```

### Key Components

#### Configuration Types
- **AppConfig**: Root configuration containing all subsystem configs
- **ServerConfig**: HTTP server configuration (host, port, TLS settings)
- **DatabaseConfig**: Database connection configuration with read replicas
- **OAuthConfig**: OAuth provider configurations (GitHub, GitLab)
- **JwtConfig**: JWT token configuration with extensible secret storage
- **CommandConfig**: Command retry configuration system
- **CommandRetryConfig**: Retry policy configuration for commands

#### JWT Secret Storage Architecture

The JWT configuration system uses a layered approach to support multiple secret storage backends while keeping the JWT encoder agnostic to the secret source:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JwtConfig {
    /// Token expiration time in seconds
    pub expiration_seconds: u64,
    /// Secret storage configuration
    pub secret_storage: SecretStorage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SecretStorage {
    /// Plain text HMAC secret (development/legacy)
    PlainText { secret: String },
    /// RSA key pair from PEM files (recommended for production)
    PemFile {
        private_key_path: String,
        public_key_path: String,
    },
    /// HashiCorp Vault integration (future)
    Vault {
        vault_url: String,
        secret_path: String,
        role: String,
    },
    /// GCP Secret Manager integration (future)
    GcpSecretManager {
        project_id: String,
        secret_name: String,
        version: String,
    },
}

/// Resolved JWT secret for token operations
#[derive(Debug, Clone)]
pub enum JwtSecret {
    /// HMAC symmetric key
    Hmac(String),
    /// RSA asymmetric key pair
    Rsa {
        private_key: String,
        public_key: String,
        kid: String,
    },
}
```

**Secret Resolution Flow**:
1. **Configuration Loading**: SecretStorage enum loaded from config files
2. **Secret Resolution**: Converts storage config to JwtSecret at startup
3. **JWT Service Creation**: JwtTokenService created with resolved secret
4. **Token Operations**: JWT encoding/decoding using appropriate algorithm

**Benefits**:
- **Extensibility**: Easy to add new secret storage backends
- **Security**: Separation of storage mechanism from cryptographic operations
- **Flexibility**: Support for both symmetric and asymmetric algorithms
- **Future-Proof**: Ready for enterprise secret management integration

### Example: Complete Configuration Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub oauth: OAuthConfig,
    pub jwt: JwtConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub command: CommandConfig,  // Command retry configuration
}
```

### Configuration File Examples

#### Development Environment (`config/development.toml`)
```toml
[command.retry]
# Lenient settings for development
max_attempts = 5
base_delay_ms = 200
max_delay_ms = 10000
backoff_multiplier = 2.0
use_jitter = true

[command.overrides.test_command]
max_attempts = 2
base_delay_ms = 100
max_delay_ms = 5000
backoff_multiplier = 1.5
use_jitter = false
```

#### Production Environment (`config/production.toml`)
```toml
[command.retry]
# Conservative settings for production
max_attempts = 3
base_delay_ms = 500
max_delay_ms = 60000  # 1 minute max delay
backoff_multiplier = 2.0
use_jitter = true

[command.overrides.critical_command]
max_attempts = 5
base_delay_ms = 1000
max_delay_ms = 30000
backoff_multiplier = 1.8
use_jitter = true
```

#### Testing Environment (`config/test.toml`)
```toml
[command.retry]
# Fast, predictable settings for tests
max_attempts = 2
base_delay_ms = 50
max_delay_ms = 5000
backoff_multiplier = 2.0
use_jitter = false  # Disable jitter for test determinism
```

### Configuration Integration

The configuration system integrates with the Command Bus for runtime policy resolution:

```rust
impl CommandConfig {
    /// Get retry configuration for a specific command
    /// Returns command-specific config if available, otherwise returns default
    pub fn get_retry_config(&self, command_type: &str) -> &CommandRetryConfig {
        self.overrides.get(command_type).unwrap_or(&self.retry)
    }
}
```

### Environment Variable Support

All configuration values can be overridden using environment variables:

```bash
# Override default retry attempts
IAM_COMMAND__RETRY__MAX_ATTEMPTS=5

# Override specific command configuration
IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__MAX_ATTEMPTS=3
IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__BASE_DELAY_MS=100
```

## Setup Layer

**Location**: `setup/` crate  
**Purpose**: Application bootstrap and dependency injection

### Responsibilities
- **Dependency Wiring**: Connect all layers together
- **Configuration Loading**: Load and validate configuration
- **Service Construction**: Build all services and repositories
- **Server Startup**: Initialize and start the HTTP server

### Example: Dependency Injection

```rust
pub async fn build_app_state(config: AppConfig) -> Result<AppState> {
    // Infrastructure layer
    let db_pool = DbConnectionPool::new(&config.database).await?;
    let user_repo = CombinedUserRepository::new(/* ... */);
    let token_repo = CombinedTokenRepository::new(/* ... */);
    
    // Domain services
    let auth_service = AuthService::new(user_repo, token_repo, token_service);
    
    // Application use cases
    let login_usecase = LoginUseCaseImpl::new(/* ... */);
    
    // HTTP layer
    let app_state = AppState::new(
        Arc::new(login_usecase),
        /* ... */
    );
    
    Ok(app_state)
}
```

## Dependency Flow

The dependency flow follows the Dependency Inversion Principle:

```
HTTP Layer
    ↓ (depends on)
Application Layer (Use Cases)
    ↓ (depends on)
Domain Layer (Entities, Services, Ports)
    ↑ (implemented by)
Infrastructure Layer (Adapters)
```

### Key Rules

1. **Domain Layer**: No dependencies on other layers
2. **Application Layer**: Only depends on Domain
3. **Infrastructure Layer**: Implements Domain ports
4. **HTTP Layer**: Only depends on Application use cases
5. **Setup Layer**: Wires everything together

## Key Design Patterns

### Repository Pattern
Abstracts data persistence behind domain interfaces:

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
    async fn create(&self, user: User) -> Result<User, Self::Error>;
}
```

### Adapter Pattern
External services implement domain ports:

```rust
#[async_trait]
impl ProviderOAuth2Client for GitHubOAuth2Client {
    async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError> {
        // GitHub-specific implementation
    }
}
```

### Use Case Pattern
Application orchestrates domain services:

```rust
pub trait LoginUseCase: Send + Sync {
    async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LoginResponse, LoginError>;
}
```

### Error Mapping
Each layer has its own error types with proper mapping:

```rust
// Domain errors
pub enum DomainError {
    UserNotFound,
    TokenExpired,
    // ...
}

// Application errors map domain errors
pub enum LoginError {
    AuthError(String),
    DbError(Box<dyn std::error::Error + Send + Sync>),
    TokenError(Box<dyn std::error::Error + Send + Sync>),
}
```

## Benefits

### 1. **Testability**
- Domain logic can be tested in isolation
- Easy to mock external dependencies
- Clear boundaries for unit vs integration tests

### 2. **Maintainability**
- Changes to external systems don't affect business logic
- Clear separation of concerns
- Easy to understand and modify

### 3. **Flexibility**
- Can swap implementations (e.g., database, OAuth providers)
- Easy to add new features
- Framework-agnostic core

### 4. **Domain Focus**
- Business logic is explicit and central
- Domain experts can understand the code
- Ubiquitous language throughout

### 5. **Error Handling**
- Domain errors are explicit and typed
- Proper error propagation through layers
- No infrastructure errors leak to domain

## Migration Journey

### Before: Monolithic Structure
- Services mixed application and domain logic
- Direct dependencies on infrastructure
- Difficult to test and maintain

### After: Hexagonal + DDD
- **Domain Services**: Pure business logic in `domain/src/service/`
- **Use Cases**: Orchestration in `application/src/usecase/`
- **Ports**: Clear interfaces in `domain/src/port/`
- **Adapters**: Infrastructure implementations in `infra/`
- **Error Mapping**: Domain errors mapped to application errors

### Key Migration Steps
1. **Moved Services**: From `application` to `domain` layer
2. **Created Ports**: Defined clear interfaces for external dependencies
3. **Error Refactoring**: Domain services return `DomainError`
4. **Use Case Adaptation**: Application layer maps domain errors
5. **Configuration Extraction**: Centralized in dedicated crate

### Example: Service Migration

**Before** (Application Service):
```rust
// Mixed concerns - application and domain logic together
impl AuthService {
    pub async fn process_callback(&self, provider: &str, code: &str) 
        -> Result<AuthResponseDto, ApplicationError> {
        // Business logic mixed with DTO creation
    }
}
```

**After** (Domain Service + Use Case):
```rust
// Domain Service - Pure business logic
impl AuthService {
    pub async fn process_callback(&self, provider: &str, code: &str) 
        -> Result<(User, String), DomainError> {
        // Pure business logic, returns domain entities
    }
}

// Use Case - Orchestration and DTO mapping
impl AuthUseCase {
    pub async fn process_callback(&self, provider: &str, code: &str) 
        -> Result<AuthResponseDto, AuthUseCaseError> {
        let (user, token) = self.auth_service
            .process_callback(provider, code)
            .await
            .map_err(AuthUseCaseError::Domain)?;
            
        Ok(AuthResponseDto {
            token,
            user: UserProfileDto::from(user),
        })
    }
}
```

## Conclusion

This architecture provides a robust foundation for the IAM service that:

- **Protects** business logic from external changes
- **Enables** comprehensive testing at all levels
- **Supports** rapid feature development
- **Maintains** code quality and readability
- **Follows** industry best practices

The combination of Hexagonal Architecture and DDD ensures that the codebase remains maintainable and extensible as the service grows and evolves. 