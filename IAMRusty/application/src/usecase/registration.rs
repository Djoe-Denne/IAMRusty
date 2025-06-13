use async_trait::async_trait;
use thiserror::Error;
use std::sync::Arc;
use domain::{
    entity::{
        events::{DomainEvent, UserSignedUpEvent},
    },
    port::{
        repository::{UserEmailRepository, UserReadRepository, UserWriteRepository},
        service::{RegistrationTokenService, AuthTokenService},
        event_publisher::EventPublisher,
    },
    error::DomainError,
};

use crate::dto::auth::{
    CompleteRegistrationRequest,
    CompleteRegistrationResponse,
    CheckUsernameRequest,
    CheckUsernameResponse,
    UserDto,
};

/// Registration use case error
#[derive(Debug, Error)]
pub enum RegistrationError {
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(Box<dyn std::error::Error + Send + Sync>),

    /// Event publishing error
    #[error("Event publishing error: {0}")]
    EventError(Box<dyn std::error::Error + Send + Sync>),

    /// Domain error
    #[error("Domain error: {0}")]
    DomainError(#[from] DomainError),

    /// Invalid or expired registration token
    #[error("Invalid or expired registration token")]
    InvalidToken,

    /// Token expired
    #[error("Registration token has expired")]
    TokenExpired,

    /// Username already taken
    #[error("Username already taken")]
    UsernameTaken,

    /// Invalid username format
    #[error("Invalid username format")]
    InvalidUsername,

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// User already has username (registration already complete)
    #[error("Registration already completed")]
    RegistrationAlreadyComplete,
}

/// Registration use case interface
#[async_trait]
pub trait RegistrationUseCase: Send + Sync {
    /// Complete user registration with username
    async fn complete_registration(&self, request: CompleteRegistrationRequest) -> Result<CompleteRegistrationResponse, RegistrationError>;
    
    /// Check username availability
    async fn check_username(&self, request: CheckUsernameRequest) -> Result<CheckUsernameResponse, RegistrationError>;
}

/// Username validation rules
pub struct UsernameValidator;

impl UsernameValidator {
    /// Validate username format
    pub fn validate(username: &str) -> Result<(), RegistrationError> {
        if username.len() < 3 {
            return Err(RegistrationError::InvalidUsername);
        }
        if username.len() > 50 {
            return Err(RegistrationError::InvalidUsername);
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(RegistrationError::InvalidUsername);
        }
        // Require at least one letter (cannot be only numbers/symbols)
        if !username.chars().any(|c| c.is_alphabetic()) {
            return Err(RegistrationError::InvalidUsername);
        }
        Ok(())
    }

    /// Generate username suggestions when username is taken
    pub fn generate_suggestions(base_username: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Add numeric suffixes
        for i in 1..=10 {
            suggestions.push(format!("{}{}", base_username, i));
        }
        
        // Add underscore variants (only suffix, not prefix)
        suggestions.push(format!("{}_", base_username));
        
        // Add random suffix
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_suffix: u32 = rng.gen_range(100..999);
        suggestions.push(format!("{}_{}", base_username, random_suffix));

        suggestions
    }
}

/// Registration use case implementation
pub struct RegistrationUseCaseImpl<UR, UW, UER, RTS, TS, EP>
where
    UR: UserReadRepository,
    UW: UserWriteRepository,
    UER: UserEmailRepository,
    RTS: RegistrationTokenService,
    TS: AuthTokenService,
    EP: EventPublisher,
{
    user_read_repo: Arc<UR>,
    user_write_repo: Arc<UW>,
    user_email_repo: Arc<UER>,
    registration_token_service: Arc<RTS>,
    token_service: Arc<TS>,
    event_publisher: Arc<EP>,
}

impl<UR, UW, UER, RTS, TS, EP> RegistrationUseCaseImpl<UR, UW, UER, RTS, TS, EP>
where
    UR: UserReadRepository + Send + Sync,
    UW: UserWriteRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
{
    pub fn new(
        user_read_repo: Arc<UR>,
        user_write_repo: Arc<UW>,
        user_email_repo: Arc<UER>,
        registration_token_service: Arc<RTS>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
    ) -> Self {
        Self {
            user_read_repo,
            user_write_repo,
            user_email_repo,
            registration_token_service,
            token_service,
            event_publisher,
        }
    }
}

#[async_trait]
impl<UR, UW, UER, RTS, TS, EP> RegistrationUseCase for RegistrationUseCaseImpl<UR, UW, UER, RTS, TS, EP>
where
    UR: UserReadRepository + Send + Sync,
    UW: UserWriteRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
    <UR as UserReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UW as UserWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TS as AuthTokenService>::Error: std::error::Error + Send + Sync + 'static,

{
    async fn complete_registration(&self, request: CompleteRegistrationRequest) -> Result<CompleteRegistrationResponse, RegistrationError> {
        // Validate username format
        UsernameValidator::validate(&request.username)?;

        // Validate and decode registration token
        let token_claims = self.registration_token_service
            .validate_registration_token(&request.registration_token)
            .map_err(|domain_error| match domain_error {
                DomainError::TokenExpired => RegistrationError::TokenExpired,
                _ => RegistrationError::InvalidToken,
            })?;

        // Get user by ID from token
        let user_id = token_claims.get_user_id()
            .map_err(|_| RegistrationError::InvalidToken)?;

        let mut user = self.user_read_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| RegistrationError::RepositoryError(Box::new(e)))?
            .ok_or(RegistrationError::UserNotFound)?;

        // Check if registration is already complete
        if user.is_registration_complete() {
            return Err(RegistrationError::InvalidToken);
        }

        // Check if username is already taken
        if let Ok(Some(_)) = self.user_read_repo.find_by_username(&request.username).await {
            return Err(RegistrationError::UsernameTaken);
        }

        // Update user with username
        user.complete_registration(request.username.clone());
        let updated_user = self.user_write_repo
            .update(user)
            .await
            .map_err(|e| RegistrationError::RepositoryError(Box::new(e)))?;

        // Get user's primary email (no automatic verification)
        let user_email = self.user_email_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| RegistrationError::RepositoryError(Box::new(e)))?
            .into_iter()
            .find(|email| email.is_primary)
            .ok_or_else(|| RegistrationError::RepositoryError(
                Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Primary email not found"))
            ))?;

        // Generate access and refresh tokens
        let access_token = self.token_service
            .generate_access_token(user_id)
            .await
            .map_err(|e| RegistrationError::TokenServiceError(Box::new(e)))?;

        let refresh_token = self.token_service
            .generate_refresh_token(user_id)
            .await
            .map_err(|e| RegistrationError::TokenServiceError(Box::new(e)))?;

        // Publish UserSignedUp event only for email/password flows
        // OAuth flows don't need this event since the email is already verified by the provider
        if token_claims.is_email_password_flow() {
            let event = DomainEvent::UserSignedUp(UserSignedUpEvent::new(
                user_id,
                user_email.email.clone(),
                request.username.clone(),
                user_email.is_verified,
            ));

            if let Err(e) = self.event_publisher.publish(event).await {
                tracing::warn!("Failed to publish UserSignedUp event: {}", e);
                // Don't fail the registration for event publishing errors
            }
        }

        Ok(CompleteRegistrationResponse {
            user: UserDto {
                id: updated_user.id.to_string(),
                username: updated_user.username.unwrap_or_default(),
                email: user_email.email,
                avatar: updated_user.avatar_url,
            },
            access_token: access_token.token,
            expires_in: 3600, // TODO: Get from config
            refresh_token: refresh_token.token,
        })
    }

    async fn check_username(&self, request: CheckUsernameRequest) -> Result<CheckUsernameResponse, RegistrationError> {
        // Validate username format first
        if let Err(_) = UsernameValidator::validate(&request.username) {
            return Ok(CheckUsernameResponse {
                available: false,
                suggestions: Some(UsernameValidator::generate_suggestions(&request.username)),
            });
        }

        // Check if username exists
        let exists = self.user_read_repo
            .find_by_username(&request.username)
            .await
            .map_err(|e| RegistrationError::RepositoryError(Box::new(e)))?
            .is_some();

        let response = if exists {
            CheckUsernameResponse {
                available: false,
                suggestions: Some(UsernameValidator::generate_suggestions(&request.username)),
            }
        } else {
            CheckUsernameResponse {
                available: true,
                suggestions: Some(Vec::new()),
            }
        };

        Ok(response)
    }
} 