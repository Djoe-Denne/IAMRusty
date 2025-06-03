use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;
use std::sync::Arc;
use domain::port::{
    repository::{UserRepository, UserEmailRepository},
    service::AuthTokenService,
};
use domain::entity::user::User;

/// User profile with email information
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// User entity
    pub user: User,
    /// Primary email address
    pub email: Option<String>,
}

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
    /// Get a user profile by ID (includes primary email)
    async fn get_user(&self, id: Uuid) -> Result<UserProfile, UserError>;
    
    /// Validate a user's JWT token and return the user ID
    async fn validate_token(&self, token: &str) -> Result<Uuid, UserError>;
}

/// User use case implementation
pub struct UserUseCaseImpl<UR, UER, T>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    T: AuthTokenService,
{
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_service: Arc<T>,
}

impl<UR, UER, T> UserUseCaseImpl<UR, UER, T>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    T: AuthTokenService,
{
    /// Create a new UserUseCaseImpl
    pub fn new(user_repo: Arc<UR>, user_email_repo: Arc<UER>, token_service: Arc<T>) -> Self {
        Self {
            user_repo,
            user_email_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<UR, UER, T> UserUseCase for UserUseCaseImpl<UR, UER, T>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    T: AuthTokenService + Send + Sync,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_user(&self, id: Uuid) -> Result<UserProfile, UserError> {
        // Get the user
        let user = self.user_repo
            .find_by_id(id)
            .await
            .map_err(|e| UserError::RepositoryError(Box::new(e)))?
            .ok_or(UserError::UserNotFound)?;
        
        // Get the primary email
        let primary_email = self.user_email_repo
            .find_primary_by_user_id(id)
            .await
            .map_err(|e| UserError::RepositoryError(Box::new(e)))?;
        
        Ok(UserProfile {
            user,
            email: primary_email.map(|email| email.email),
        })
    }
    
    async fn validate_token(&self, token: &str) -> Result<Uuid, UserError> {
        // Validate the token and get the user ID
        self.token_service
            .validate_access_token(token)
            .await
            .map_err(|e| UserError::TokenServiceError(Box::new(e)))
    }
} 