//! Authentication service for email/password operations

use crate::entity::{
    email_verification::EmailVerification,
    events::{DomainEvent, UserEmailVerifiedEvent, UserLoggedInEvent, UserSignedUpEvent},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::{
    repository::{EmailVerificationRepository, UserEmailRepository, UserRepository},
    service::{AuthTokenService, RegistrationTokenService},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rustycog::events::event::EventPublisher;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;
use uuid::Uuid;

use super::IamOutboxUnitOfWork;
use crate::utils;

/// Authentication service errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Weak password")]
    WeakPassword,

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Email not found")]
    EmailNotFound,

    #[error("Email already verified")]
    EmailAlreadyVerified,

    #[error("Invalid verification token")]
    InvalidVerificationToken,

    #[error("Verification token expired")]
    VerificationTokenExpired,

    #[error("Repository error: {0}")]
    RepositoryError(#[from] DomainError),

    #[error("Event publishing error: {0}")]
    EventPublishingError(String),

    #[error("Token service error: {0}")]
    TokenServiceError(Box<dyn std::error::Error + Send + Sync>),

    #[error("Password hashing error: {0}")]
    PasswordHashingError(String),

    #[error("Verification token generation error: {0}")]
    VerificationTokenGenerationError(String),
}

/// Signup request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

/// Signup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignupResponse {
    /// Existing user - password auth added
    ExistingUser {
        user: UserProfile,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
        message: String,
    },
    /// New user created - username required  
    RegistrationRequired {
        user: IncompleteUserProfile,
        registration_token: String,
        requires_username: bool,
        message: String,
    },
}

/// User profile for incomplete registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteUserProfile {
    pub id: Uuid,
    pub email: String,
}

/// Login request for email/password authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response for email/password authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoginResponse {
    /// Successful login
    Success {
        user: UserProfile,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
    },
    /// Registration incomplete - needs username
    RegistrationIncomplete {
        registration_token: String,
        message: String,
    },
}

/// Verify email request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub verification_token: String,
}

/// Verify email response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEmailResponse {
    pub message: String,
}

/// Resend verification email request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendVerificationEmailRequest {
    pub email: String,
}

/// Resend verification email response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendVerificationEmailResponse {
    pub message: String,
}

/// User profile for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: String,
    pub avatar: Option<String>,
}

/// Password service trait for dependency injection
#[async_trait]
pub trait PasswordService: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, AuthError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
}

#[async_trait]
pub trait SignupTransaction: Send + Sync {
    async fn create_incomplete_user_with_verification(
        &self,
        user: User,
        user_email: UserEmail,
        email_verification: EmailVerification,
    ) -> Result<User, DomainError>;
}

/// Authentication service for email/password operations
pub struct AuthService<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher<DomainError>,
{
    user_repository: Arc<UR>,
    user_email_repository: Arc<UER>,
    email_verification_repository: Arc<EVR>,
    password_service: Arc<PS>,
    token_service: Arc<TS>,
    registration_token_service: Arc<RTS>,
    event_publisher: Arc<EP>,
    signup_transaction: Option<Arc<dyn SignupTransaction>>,
    outbox_unit_of_work: Option<Arc<dyn IamOutboxUnitOfWork>>,
}

pub struct AuthServiceDependencies<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher<DomainError>,
{
    pub user_repository: Arc<UR>,
    pub user_email_repository: Arc<UER>,
    pub email_verification_repository: Arc<EVR>,
    pub password_service: Arc<PS>,
    pub token_service: Arc<TS>,
    pub registration_token_service: Arc<RTS>,
    pub event_publisher: Arc<EP>,
}

impl<UR, UER, EVR, PS, TS, RTS, EP> AuthService<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher<DomainError>,
{
    #[must_use]
    pub fn new(dependencies: AuthServiceDependencies<UR, UER, EVR, PS, TS, RTS, EP>) -> Self {
        Self::from_parts(dependencies, None, None)
    }

    pub fn new_with_signup_transaction(
        dependencies: AuthServiceDependencies<UR, UER, EVR, PS, TS, RTS, EP>,
        signup_transaction: Arc<dyn SignupTransaction>,
    ) -> Self {
        Self::from_parts(dependencies, Some(signup_transaction), None)
    }

    pub fn new_with_signup_transaction_and_outbox(
        dependencies: AuthServiceDependencies<UR, UER, EVR, PS, TS, RTS, EP>,
        signup_transaction: Arc<dyn SignupTransaction>,
        outbox_unit_of_work: Arc<dyn IamOutboxUnitOfWork>,
    ) -> Self {
        Self::from_parts(
            dependencies,
            Some(signup_transaction),
            Some(outbox_unit_of_work),
        )
    }

    fn from_parts(
        dependencies: AuthServiceDependencies<UR, UER, EVR, PS, TS, RTS, EP>,
        signup_transaction: Option<Arc<dyn SignupTransaction>>,
        outbox_unit_of_work: Option<Arc<dyn IamOutboxUnitOfWork>>,
    ) -> Self {
        Self {
            user_repository: dependencies.user_repository,
            user_email_repository: dependencies.user_email_repository,
            email_verification_repository: dependencies.email_verification_repository,
            password_service: dependencies.password_service,
            token_service: dependencies.token_service,
            registration_token_service: dependencies.registration_token_service,
            event_publisher: dependencies.event_publisher,
            signup_transaction,
            outbox_unit_of_work,
        }
    }

    /// Generate a verification token using UUID v4
    /// Simple, secure, and doesn't require crypto dependencies
    /// In test/QA mode, returns a static token for predictable testing
    fn generate_verification_token() -> String {
        #[cfg(any(test, feature = "test-mode"))]
        {
            debug!("Generating test/QA verification token");
            "VALIDATION_TOKEN".to_string()
        }
        #[cfg(not(any(test, feature = "test-mode")))]
        Uuid::new_v4().to_string()
    }

    fn expires_in_secs(expires_at: DateTime<Utc>) -> u64 {
        u64::try_from((expires_at - Utc::now()).num_seconds()).unwrap_or(0)
    }
}

impl<UR, UER, EVR, PS, TS, RTS, EP> AuthService<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    EVR: EmailVerificationRepository + Send + Sync,
    PS: PasswordService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    EP: EventPublisher<DomainError> + Send + Sync,
{
    async fn record_or_publish_event(
        &self,
        event: Box<dyn rustycog::events::event::DomainEvent + 'static>,
    ) -> Result<(), String> {
        if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            outbox_unit_of_work
                .record_event(event)
                .await
                .map_err(|e| e.to_string())
        } else {
            self.event_publisher
                .publish(event.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
    }

    /// Register a new email/password account or attach a password to an existing one.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError`] if the email is already fully registered, the user cannot be
    /// loaded or persisted, password hashing fails, or token generation fails.
    pub async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, AuthError> {
        if let Ok(Some(existing_email)) = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
        {
            return self.signup_existing_email(request, existing_email).await;
        }
        self.signup_new_user(request).await
    }

    async fn signup_existing_email(
        &self,
        request: SignupRequest,
        existing_email: UserEmail,
    ) -> Result<SignupResponse, AuthError> {
        let existing_user = self
            .user_repository
            .find_by_id(existing_email.user_id)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::UserNotFound)?;

        if existing_user.is_registration_complete() {
            return Err(AuthError::UserAlreadyExists);
        }

        // Incomplete OAuth account (no password): do not attach a password
        // from the public signup. Linking a password requires an authenticated
        // session.
        if !existing_user.has_password() {
            return Err(AuthError::UserAlreadyExists);
        }

        let stored_hash = existing_user
            .password_hash
            .as_deref()
            .expect("incomplete password signup has a hash");
        let password_matches = self
            .password_service
            .verify_password(&request.password, stored_hash)
            .await
            .unwrap_or(false);
        if !password_matches {
            return Err(AuthError::UserAlreadyExists);
        }

        let registration_token = self
            .registration_token_service
            .generate_registration_token(existing_user.id, request.email.clone())
            .map_err(AuthError::RepositoryError)?;
        Ok(SignupResponse::RegistrationRequired {
            user: IncompleteUserProfile {
                id: existing_user.id,
                email: request.email,
            },
            registration_token,
            requires_username: true,
            message: "Account created. Please choose a username to complete registration"
                .to_string(),
        })
    }

    async fn signup_new_user(&self, request: SignupRequest) -> Result<SignupResponse, AuthError> {
        let password_hash = self
            .password_service
            .hash_password(&request.password)
            .await?;
        let user = User::new_incomplete_with_password(password_hash, None);
        let user_email = UserEmail::new(user.id, request.email.clone(), true, false);
        let email_verification = EmailVerification::new(
            request.email.clone(),
            Self::generate_verification_token(),
            24,
        );

        let created_user = if let Some(signup_transaction) = &self.signup_transaction {
            signup_transaction
                .create_incomplete_user_with_verification(user, user_email, email_verification)
                .await
                .map_err(AuthError::RepositoryError)?
        } else {
            let created_user = self.user_repository.create(user).await.map_err(|e| {
                AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
            })?;
            self.user_email_repository
                .create(user_email)
                .await
                .map_err(|e| {
                    AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
                })?;
            self.email_verification_repository
                .create(&email_verification)
                .await
                .map_err(|e| {
                    AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
                })?;
            created_user
        };

        let registration_token = self
            .registration_token_service
            .generate_registration_token(created_user.id, request.email.clone())
            .map_err(AuthError::RepositoryError)?;
        Ok(SignupResponse::RegistrationRequired {
            user: IncompleteUserProfile {
                id: created_user.id,
                email: request.email,
            },
            registration_token,
            requires_username: true,
            message: "Account created. Please choose a username to complete registration"
                .to_string(),
        })
    }

    /// Authenticate an email/password account.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError`] if credentials are invalid, the email is not verified,
    /// the user cannot be loaded, password verification fails, or token generation fails.
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AuthError> {
        // Find user by email
        let user_email = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::InvalidCredentials)?; // Don't leak user existence

        // Get the user
        let user = self
            .user_repository
            .find_by_id(user_email.user_id)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check if user has a password (should not happen if signup worked correctly)
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AuthError::InvalidCredentials)?;

        // Verify password first
        let is_valid = self
            .password_service
            .verify_password(&request.password, password_hash)
            .await?;

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if user has completed registration (has username) BEFORE checking email verification
        // This is because incomplete users cannot have verified emails
        if user.username.is_none() {
            // Generate registration token (RSA-signed JWT)
            let registration_token = self
                .registration_token_service
                .generate_registration_token(user.id, request.email.clone())
                .map_err(AuthError::RepositoryError)?;

            return Ok(LoginResponse::RegistrationIncomplete {
                registration_token,
                message: "Account exists but registration is incomplete. Please complete registration with a username.".to_string(),
            });
        }

        // Only check email verification for complete users
        if !user_email.is_verified {
            return Err(AuthError::EmailNotVerified);
        }

        self.complete_login(user, request.email).await
    }

    async fn complete_login(&self, user: User, email: String) -> Result<LoginResponse, AuthError> {
        let access_token = self
            .token_service
            .generate_access_token(user.id)
            .await
            .map_err(|e| AuthError::TokenServiceError(Box::new(e)))?;
        let refresh_token = self
            .token_service
            .generate_refresh_token(user.id)
            .await
            .map_err(|e| AuthError::TokenServiceError(Box::new(e)))?;

        let event: Box<dyn rustycog::events::event::DomainEvent + 'static> =
            DomainEvent::UserLoggedIn(UserLoggedInEvent::new(
                user.id,
                email.clone(),
                "email_password".to_string(),
            ))
            .into();
        if let Err(e) = self.event_publisher.publish(event.as_ref()).await {
            tracing::warn!("Failed to publish UserLoggedIn event: {}", e);
        }

        Ok(LoginResponse::Success {
            user: UserProfile {
                id: user.id,
                username: user.username.clone(),
                email,
                avatar: user.avatar_url,
            },
            access_token: access_token.token,
            expires_in: Self::expires_in_secs(access_token.expires_at),
            refresh_token: refresh_token.token,
        })
    }

    /// Confirm an email address with a verification token.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError`] if the email or token is missing or invalid, the token
    /// is expired, the email is already verified, or persistence fails.
    pub async fn verify_email(
        &self,
        request: VerifyEmailRequest,
    ) -> Result<VerifyEmailResponse, AuthError> {
        debug!("Verifying email: {}", request.email);
        // Find verification token
        let verification = self
            .email_verification_repository
            .find_by_email_and_token(&request.email, &request.verification_token)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        let Some(verification) = verification else {
            return Err(self.verification_lookup_error(&request.email).await?);
        };

        // Check if token is expired
        if verification.is_expired() {
            return Err(AuthError::VerificationTokenExpired);
        }

        // Find user email
        let mut user_email = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::EmailNotFound)?;

        // Check if already verified
        if user_email.is_verified {
            return Err(AuthError::EmailAlreadyVerified);
        }

        // Mark email as verified
        user_email.verify();
        let updated_email = self
            .user_email_repository
            .update(user_email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        self.cleanup_used_verification_token(verification.id, &request.email)
            .await;

        // Publish UserEmailVerified event
        let event = DomainEvent::UserEmailVerified(UserEmailVerifiedEvent::new(
            updated_email.user_id,
            request.email,
        ));

        if let Err(e) = self.record_or_publish_event(event.into()).await {
            tracing::warn!("Failed to record/publish UserEmailVerified event: {}", e);
            // Don't fail the verification for event publishing errors
        }

        Ok(VerifyEmailResponse {
            message: "Email verified successfully".to_string(),
        })
    }

    async fn verification_lookup_error(&self, email: &str) -> Result<AuthError, AuthError> {
        let user_email_exists = self
            .user_email_repository
            .find_by_email(email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .is_some();
        if user_email_exists {
            Ok(AuthError::InvalidVerificationToken)
        } else {
            Ok(AuthError::EmailNotFound)
        }
    }

    async fn cleanup_used_verification_token(&self, verification_id: Uuid, email: &str) {
        if let Err(e) = self
            .email_verification_repository
            .delete_by_id(verification_id)
            .await
        {
            tracing::warn!(
                "Failed to delete verification token after successful verification: {e}"
            );
            if let Err(e2) = self
                .email_verification_repository
                .delete_by_email(email)
                .await
            {
                tracing::error!("Failed to delete verification token by email as fallback: {e2}");
            }
        }
    }

    /// Request another verification email for an unverified address.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError`] if the email lookup fails at the repository.
    pub async fn resend_verification_email(
        &self,
        request: ResendVerificationEmailRequest,
    ) -> Result<ResendVerificationEmailResponse, AuthError> {
        // Find user by email
        let user_email_result = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // For security reasons (prevent user enumeration), always return success response
        // but only perform actions if the email exists and is unverified
        match user_email_result {
            Some(user_email) if !user_email.is_verified => {
                self.resend_unverified_email(&request.email, user_email.user_id)
                    .await;
            }
            Some(_) => {
                // Email is already verified - log but don't reveal this information
                tracing::debug!(
                    "Resend verification requested for already verified email: {}",
                    request.email
                );
            }
            None => {
                // Email not found - log but don't reveal this information
                tracing::debug!(
                    "Resend verification requested for non-existent email: {}",
                    request.email
                );
            }
        }

        // Always return success response to prevent user enumeration attacks
        Ok(ResendVerificationEmailResponse {
            message:
                "If your email is registered and unverified, a verification email has been sent."
                    .to_string(),
        })
    }

    async fn resend_unverified_email(&self, email: &str, user_id: Uuid) {
        if let Err(e) = self
            .email_verification_repository
            .delete_by_email(email)
            .await
        {
            tracing::warn!(
                "Failed to delete existing verification tokens for {}: {}",
                email,
                e
            );
        }

        let email_verification =
            EmailVerification::new(email.to_string(), Self::generate_verification_token(), 24);

        if let Err(e) = self
            .email_verification_repository
            .create(&email_verification)
            .await
        {
            tracing::error!("Failed to create verification token: {}", e);
            return;
        }

        self.publish_resend_verification_event(email, user_id, &email_verification)
            .await;
    }

    async fn publish_resend_verification_event(
        &self,
        email: &str,
        user_id: Uuid,
        email_verification: &EmailVerification,
    ) {
        let Ok(Some(user)) = self.user_repository.find_by_id(user_id).await else {
            return;
        };

        let Some(username) = user.username else {
            return;
        };

        let event: Box<dyn rustycog::events::event::DomainEvent + 'static> =
            DomainEvent::UserSignedUp(UserSignedUpEvent::new(
                user_id,
                email.to_string(),
                username,
                false,
                Some(email_verification.verification_token.clone()),
                Some(utils::UrlUtils::build_verification_url()),
            ))
            .into();

        if let Err(e) = self.record_or_publish_event(event).await {
            tracing::warn!("Failed to record/publish UserSignedUp event: {}", e);
        }
    }
}
