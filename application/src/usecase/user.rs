use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;
use std::sync::Arc;
use domain::port::{
    repository::UserRepository,
    service::TokenService,
};
use domain::entity::user::User;

/// User usecase error
#[derive(Debug, Error)]
pub enum UserError {
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(Box<dyn std::error::Error + Send + Sync>),

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,
    
    /// Token expired
    #[error("Token expired")]
    TokenExpired,
}

/// User use case interface
#[async_trait]
pub trait UserUseCase: Send + Sync {
    /// Get a user by ID
    async fn get_user(&self, id: Uuid) -> Result<User, UserError>;
    
    /// Validate a user's JWT token and return the user ID
    async fn validate_token(&self, token: &str) -> Result<Uuid, UserError>;
}

/// User use case implementation
pub struct UserUseCaseImpl<R, T>
where
    R: UserRepository,
    T: TokenService,
{
    user_repo: Arc<R>,
    token_service: Arc<T>,
}

impl<R, T> UserUseCaseImpl<R, T>
where
    R: UserRepository,
    T: TokenService,
{
    /// Create a new UserUseCaseImpl
    pub fn new(user_repo: Arc<R>, token_service: Arc<T>) -> Self {
        Self {
            user_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<R, T> UserUseCase for UserUseCaseImpl<R, T>
where
    R: UserRepository + Send + Sync,
    T: TokenService + Send + Sync,
    <R as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_user(&self, id: Uuid) -> Result<User, UserError> {
        self.user_repo
            .find_by_id(id)
            .await
            .map_err(|e| UserError::RepositoryError(Box::new(e)))?
            .ok_or(UserError::UserNotFound)
    }
    
    async fn validate_token(&self, token: &str) -> Result<Uuid, UserError> {
        // Validate the token and get the user ID
        self.token_service
            .validate_access_token(token)
            .await
            .map_err(|e| {
                // Map token service errors to user errors
                if e.to_string().contains("expired") {
                    UserError::TokenExpired
                } else {
                    UserError::TokenServiceError(Box::new(e))
                }
            })
    }
} 