use async_trait::async_trait;
use std::sync::Arc;
use domain::service::RegistrationService;
use domain::error::DomainError;

use crate::dto::auth::{
    CompleteRegistrationRequest,
    CompleteRegistrationResponse,
    CheckUsernameRequest,
    CheckUsernameResponse,
    UserDto,
};

/// Registration use case error - thin wrapper over domain errors
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    /// Domain registration error
    #[error("Registration failed: {0}")]
    DomainError(#[from] DomainError),
}

/// Registration use case interface
#[async_trait]
pub trait RegistrationUseCase: Send + Sync {
    /// Complete user registration with username
    async fn complete_registration(&self, request: CompleteRegistrationRequest) -> Result<CompleteRegistrationResponse, RegistrationError>;
    
    /// Check username availability
    async fn check_username(&self, request: CheckUsernameRequest) -> Result<CheckUsernameResponse, RegistrationError>;
}

/// Registration use case implementation - thin orchestration layer
pub struct RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService,
{
    registration_service: Arc<RS>,
}

impl<RS> RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService + Send + Sync,
{
    pub fn new(registration_service: Arc<RS>) -> Self {
        Self {
            registration_service,
        }
    }
}

#[async_trait]
impl<RS> RegistrationUseCase for RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService + Send + Sync,
{
    async fn complete_registration(&self, request: CompleteRegistrationRequest) -> Result<CompleteRegistrationResponse, RegistrationError> {
        // Delegate to domain service
        let result = self.registration_service
            .complete_registration(&request.registration_token, request.username)
            .await?;

        // Convert domain result to DTO
        Ok(CompleteRegistrationResponse {
            user: UserDto {
                id: result.user.id.to_string(),
                username: result.user.username.unwrap_or_default(),
                email: result.user_email.email,
                avatar: result.user.avatar_url,
            },
            access_token: result.access_token,
            expires_in: result.expires_in,
            refresh_token: result.refresh_token,
        })
    }

    async fn check_username(&self, request: CheckUsernameRequest) -> Result<CheckUsernameResponse, RegistrationError> {
        // Delegate to domain service
        let result = self.registration_service
            .check_username(&request.username)
            .await?;

        // Convert domain result to DTO
        Ok(CheckUsernameResponse {
            available: result.available,
            suggestions: Some(result.suggestions),
        })
    }
} 