use domain::error::DomainError;
use thiserror::Error;

/// Application-level errors
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Repository error
    #[error("Repository error: {0}")]
    Repository(String),

    /// Service error
    #[error("Service error: {0}")]
    Service(String),

    /// OAuth2 error
    #[error("OAuth2 error: {0}")]
    OAuth2(String),

    /// Token error
    #[error("Token error: {0}")]
    Token(String),

    /// User profile error
    #[error("User profile error: {0}")]
    UserProfile(String),
}

/// Convert any error implementing std::error::Error to ApplicationError::Repository
pub fn map_repo_err<E: std::error::Error>(err: E) -> ApplicationError {
    ApplicationError::Repository(err.to_string())
} 