use thiserror::Error;

/// Domain-level errors
#[derive(Debug, Error)]
pub enum DomainError {
    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Provider not supported
    #[error("Provider not supported: {0}")]
    ProviderNotSupported(String),

    /// Business rule violation
    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Authorization error
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// OAuth2 error
    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),

    /// User profile error
    #[error("Failed to get user profile: {0}")]
    UserProfileError(String),

    /// No token found for provider and user
    #[error("No token found for provider {0} and user {1}")]
    NoTokenForProvider(String, String),

    /// Token generation failed
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    /// Token validation failed
    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),

    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(String),

    // Registration-specific errors
    /// Username already taken
    #[error("Username already taken")]
    UsernameTaken,

    /// Invalid username format
    #[error("Invalid username format")]
    InvalidUsername,

    /// User already has username (registration already complete)
    #[error("Registration already completed")]
    RegistrationAlreadyComplete,

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(String),

    /// Event publishing error
    #[error("Event publishing error: {0}")]
    EventError(String),
} 