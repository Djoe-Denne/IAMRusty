use async_trait::async_trait;
use iam_domain::error::DomainError;
use iam_domain::port::repository::IdentityRepository;
use iam_domain::service::RegistrationService;
use std::sync::Arc;

use crate::dto::auth::{
    CheckUsernameRequest, CheckUsernameResponse, CompleteRegistrationRequest,
    CompleteRegistrationResponse, UserDto,
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
    async fn complete_registration(
        &self,
        request: CompleteRegistrationRequest,
    ) -> Result<CompleteRegistrationResponse, RegistrationError>;

    /// Check username availability
    async fn check_username(
        &self,
        request: CheckUsernameRequest,
    ) -> Result<CheckUsernameResponse, RegistrationError>;
}

/// Registration use case implementation - thin orchestration layer
pub struct RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService,
{
    registration_service: Arc<RS>,
    identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
}

impl<RS> RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService + Send + Sync,
{
    pub fn new(
        registration_service: Arc<RS>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        platform_issuer: impl Into<String>,
    ) -> Self {
        Self {
            registration_service,
            identity_repo,
            platform_issuer: platform_issuer.into(),
        }
    }
}

#[async_trait]
impl<RS> RegistrationUseCase for RegistrationUseCaseImpl<RS>
where
    RS: RegistrationService + Send + Sync,
{
    async fn complete_registration(
        &self,
        request: CompleteRegistrationRequest,
    ) -> Result<CompleteRegistrationResponse, RegistrationError> {
        // Delegate to domain service
        let result = self
            .registration_service
            .complete_registration(&request.registration_token, request.username)
            .await?;

        self.identity_repo
            .ensure_platform_identity(result.user.id, &self.platform_issuer)
            .await
            .map_err(RegistrationError::DomainError)?;

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

    async fn check_username(
        &self,
        request: CheckUsernameRequest,
    ) -> Result<CheckUsernameResponse, RegistrationError> {
        // Delegate to domain service
        let result = self
            .registration_service
            .check_username(&request.username)
            .await?;

        // Convert domain result to DTO
        Ok(CheckUsernameResponse {
            available: result.available,
            suggestions: Some(result.suggestions),
        })
    }
}
