use async_trait::async_trait;
use std::sync::Arc;

use crate::entity::{
    events::{DomainEvent, UserSignedUpEvent},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::{
    event_publisher::EventPublisher,
    repository::{EmailVerificationRepository, UserEmailRepository, UserReadRepository, UserWriteRepository},
    service::{AuthTokenService, RegistrationTokenService},
};


/// Registration completion result
#[derive(Debug)]
pub struct RegistrationCompletionResult {
    pub user: User,
    pub user_email: UserEmail,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Username check result
#[derive(Debug)]
pub struct UsernameCheckResult {
    pub available: bool,
    pub suggestions: Vec<String>,
}

/// Username validation service
pub struct UsernameValidator;

impl UsernameValidator {
    /// Validate username format
    pub fn validate(username: &str) -> Result<(), DomainError> {
        if username.len() < 3 {
            return Err(DomainError::InvalidUsername);
        }
        if username.len() > 50 {
            return Err(DomainError::InvalidUsername);
        }
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(DomainError::InvalidUsername);
        }
        // Require at least one letter (cannot be only numbers/symbols)
        if !username.chars().any(|c| c.is_alphabetic()) {
            return Err(DomainError::InvalidUsername);
        }
        Ok(())
    }

    /// Generate username suggestions when username is taken
    pub fn generate_suggestions(base_username: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Add numeric suffixes
        for _i in 1..=4 {
            // Add timestamp-based suffix (simpler than rand)
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                % 1000; // Get last 3 digits of milliseconds
            suggestions.push(format!("{}_{}", base_username, timestamp));
        }
        suggestions
    }
}

/// Registration service trait
#[async_trait]
pub trait RegistrationService: Send + Sync {
    /// Complete user registration with username
    async fn complete_registration(
        &self,
        registration_token: &str,
        username: String,
    ) -> Result<RegistrationCompletionResult, DomainError>;

    /// Check username availability
    async fn check_username(&self, username: &str) -> Result<UsernameCheckResult, DomainError>;
}

/// Registration service implementation
pub struct RegistrationServiceImpl<UR, UW, UER, EVR, RTS, TS, EP>
where
    UR: UserReadRepository,
    UW: UserWriteRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    RTS: RegistrationTokenService,
    TS: AuthTokenService,
    EP: EventPublisher,
{
    user_read_repo: Arc<UR>,
    user_write_repo: Arc<UW>,
    user_email_repo: Arc<UER>,
    email_verification_repo: Arc<EVR>,
    registration_token_service: Arc<RTS>,
    token_service: Arc<TS>,
    event_publisher: Arc<EP>,
}

impl<UR, UW, UER, EVR, RTS, TS, EP> RegistrationServiceImpl<UR, UW, UER, EVR, RTS, TS, EP>
where
    UR: UserReadRepository + Send + Sync,
    UW: UserWriteRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    EVR: EmailVerificationRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
{
    pub fn new(
        user_read_repo: Arc<UR>,
        user_write_repo: Arc<UW>,
        user_email_repo: Arc<UER>,
        email_verification_repo: Arc<EVR>,
        registration_token_service: Arc<RTS>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
    ) -> Self {
        Self {
            user_read_repo,
            user_write_repo,
            user_email_repo,
            email_verification_repo,
            registration_token_service,
            token_service,
            event_publisher,
        }
    }


}

#[async_trait]
impl<UR, UW, UER, EVR, RTS, TS, EP> RegistrationService
    for RegistrationServiceImpl<UR, UW, UER, EVR, RTS, TS, EP>
where
    UR: UserReadRepository + Send + Sync,
    UW: UserWriteRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    EVR: EmailVerificationRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
    <UR as UserReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UW as UserWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    <EVR as EmailVerificationRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TS as AuthTokenService>::Error: std::error::Error + Send + Sync + 'static,
{
    async fn complete_registration(
        &self,
        registration_token: &str,
        username: String,
    ) -> Result<RegistrationCompletionResult, DomainError> {
        // Validate username format
        UsernameValidator::validate(&username)?;

        // Validate and decode registration token
        let token_claims = self
            .registration_token_service
            .validate_registration_token(registration_token)?;

        // Get user by ID from token
        let user_id = token_claims
            .get_user_id()
            .map_err(|_| DomainError::InvalidToken)?;

        let mut user = self
            .user_read_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)?;

        // Check if registration is already complete
        if user.is_registration_complete() {
            return Err(DomainError::InvalidToken);
        }

        // Check if username is already taken
        if let Ok(Some(_)) = self.user_read_repo.find_by_username(&username).await {
            return Err(DomainError::UsernameTaken);
        }

        // Update user with username
        user.complete_registration(username.clone());
        let updated_user = self
            .user_write_repo
            .update(user)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        // Get user's primary email
        let user_email = self
            .user_email_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .into_iter()
            .find(|email| email.is_primary)
            .ok_or_else(|| DomainError::RepositoryError("Primary email not found".to_string()))?;

        // Generate access and refresh tokens
        let access_token = self
            .token_service
            .generate_access_token(user_id)
            .await
            .map_err(|e| DomainError::TokenServiceError(e.to_string()))?;

        let refresh_token = self
            .token_service
            .generate_refresh_token(user_id)
            .await
            .map_err(|e| DomainError::TokenServiceError(e.to_string()))?;

        // Publish UserSignedUp event only for email/password flows
        // OAuth flows don't need this event since the email is already verified by the provider
        if token_claims.is_email_password_flow() {
            // Get verification token if email is not verified
            // Telegraph will build the verification URL from environment variables
            let verification_token = if !user_email.is_verified {
                match self.email_verification_repo.find_by_email(&user_email.email).await {
                    Ok(Some(verification)) => Some(verification.verification_token),
                    _ => {
                        tracing::warn!("No verification token found for unverified email: {}", user_email.email);
                        None
                    }
                }
            } else {
                None
            };

            let event = DomainEvent::UserSignedUp(UserSignedUpEvent::new(
                user_id,
                user_email.email.clone(),
                username.clone(),
                user_email.is_verified,
                verification_token,
                None, // Telegraph will build the URL
            ));

            if let Err(e) = self.event_publisher.publish(event).await {
                tracing::warn!("Failed to publish UserSignedUp event: {}", e);
                // Don't fail the registration for event publishing errors
            }
        }

        // Calculate expires_in from the actual token expiration
        let now = chrono::Utc::now();
        let expires_in = (access_token.expires_at - now).num_seconds().max(0) as u64;

        Ok(RegistrationCompletionResult {
            user: updated_user,
            user_email,
            access_token: access_token.token,
            refresh_token: refresh_token.token,
            expires_in, // Now calculated from actual token expiration instead of hardcoded
        })
    }

    async fn check_username(&self, username: &str) -> Result<UsernameCheckResult, DomainError> {
        // Validate username format first
        if let Err(_) = UsernameValidator::validate(username) {
            return Ok(UsernameCheckResult {
                available: false,
                suggestions: UsernameValidator::generate_suggestions(username),
            });
        }

        // Check if username exists
        let exists = self
            .user_read_repo
            .find_by_username(username)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .is_some();

        let result = if exists {
            UsernameCheckResult {
                available: false,
                suggestions: UsernameValidator::generate_suggestions(username),
            }
        } else {
            UsernameCheckResult {
                available: true,
                suggestions: Vec::new(),
            }
        };

        Ok(result)
    }
}
