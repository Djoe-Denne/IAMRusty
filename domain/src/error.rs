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
} 