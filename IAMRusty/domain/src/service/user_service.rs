use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::user::User;
use crate::error::DomainError;
use crate::port::{
    repository::{UserEmailRepository, UserRepository},
    service::AuthTokenService,
};

/// User profile containing user and email information
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// User entity
    pub user: User,
    /// Primary email address
    pub email: Option<String>,
}

/// User domain service trait
#[async_trait]
pub trait UserService: Send + Sync {
    /// Get a user profile by ID (includes primary email)
    async fn get_user_profile(&self, id: Uuid) -> Result<UserProfile, DomainError>;

    /// Validate a user's JWT token and return the user ID
    async fn validate_access_token(&self, token: &str) -> Result<Uuid, DomainError>;
}

/// User domain service implementation
pub struct UserServiceImpl<UR, UER, T>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    T: AuthTokenService,
{
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_service: Arc<T>,
}

impl<UR, UER, T> UserServiceImpl<UR, UER, T>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    T: AuthTokenService,
{
    /// Create a new UserServiceImpl
    pub fn new(user_repo: Arc<UR>, user_email_repo: Arc<UER>, token_service: Arc<T>) -> Self {
        Self {
            user_repo,
            user_email_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<UR, UER, T> UserService for UserServiceImpl<UR, UER, T>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    T: AuthTokenService + Send + Sync,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_user_profile(&self, id: Uuid) -> Result<UserProfile, DomainError> {
        // Get the user
        let user = self
            .user_repo
            .find_by_id(id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)?;

        // Get the primary email
        let primary_email = self
            .user_email_repo
            .find_primary_by_user_id(id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(UserProfile {
            user,
            email: primary_email.map(|email| email.email),
        })
    }

    async fn validate_access_token(&self, token: &str) -> Result<Uuid, DomainError> {
        // Validate the token and get the user ID
        self.token_service
            .validate_access_token(token)
            .await
            .map_err(|e| DomainError::InvalidToken)
    }
}
