use async_trait::async_trait;
use iam_domain::error::DomainError;
use iam_domain::service::{UserProfile as DomainUserProfile, UserService};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// User profile with email information
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// User entity
    pub user: iam_domain::entity::user::User,
    /// Primary email address
    pub email: Option<String>,
}

impl From<DomainUserProfile> for UserProfile {
    fn from(domain_profile: DomainUserProfile) -> Self {
        Self {
            user: domain_profile.user,
            email: domain_profile.email,
        }
    }
}

/// User usecase error
#[derive(Debug, Error)]
pub enum UserError {
    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),

    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(String),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(String),

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
    async fn validate_access_token(&self, token: &str) -> Result<Uuid, UserError>;
}

/// User use case implementation - thin orchestration layer
pub struct UserUseCaseImpl<US>
where
    US: UserService,
{
    user_service: Arc<US>,
}

impl<US> UserUseCaseImpl<US>
where
    US: UserService + Send + Sync,
{
    /// Create a new UserUseCaseImpl
    pub fn new(user_service: Arc<US>) -> Self {
        Self { user_service }
    }
}

#[async_trait]
impl<US> UserUseCase for UserUseCaseImpl<US>
where
    US: UserService + Send + Sync,
{
    async fn get_user(&self, id: Uuid) -> Result<UserProfile, UserError> {
        // Delegate to domain service
        let domain_profile = self.user_service.get_user_profile(id).await?;

        // Convert domain result to use case DTO
        Ok(UserProfile::from(domain_profile))
    }

    async fn validate_access_token(&self, token: &str) -> Result<Uuid, UserError> {
        // Delegate to domain service
        self.user_service
            .validate_access_token(token)
            .await
            .map_err(Into::into)
    }
}
