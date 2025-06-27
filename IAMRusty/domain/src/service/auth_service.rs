//! Authentication service for email/password operations

use crate::entity::{
    email_verification::EmailVerification,
    events::{DomainEvent, UserEmailVerifiedEvent, UserLoggedInEvent, UserSignedUpEvent},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::{
    event_publisher::EventPublisher,
    repository::{EmailVerificationRepository, UserEmailRepository, UserRepository},
    service::{AuthTokenService, RegistrationTokenService},
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;
use uuid::Uuid;

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

/// Authentication service for email/password operations
pub struct AuthService<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    user_repository: Arc<UR>,
    user_email_repository: Arc<UER>,
    email_verification_repository: Arc<EVR>,
    password_service: Arc<PS>,
    token_service: Arc<TS>,
    registration_token_service: Arc<RTS>,
    event_publisher: Arc<EP>,
}

impl<UR, UER, EVR, PS, TS, RTS, EP> AuthService<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    pub fn new(
        user_repository: Arc<UR>,
        user_email_repository: Arc<UER>,
        email_verification_repository: Arc<EVR>,
        password_service: Arc<PS>,
        token_service: Arc<TS>,
        registration_token_service: Arc<RTS>,
        event_publisher: Arc<EP>,
    ) -> Self {
        Self {
            user_repository,
            user_email_repository,
            email_verification_repository,
            password_service,
            token_service,
            registration_token_service,
            event_publisher,
        }
    }

    /// Generate a verification token using UUID v4
    /// Simple, secure, and doesn't require crypto dependencies
    /// In test/QA mode, returns a static token for predictable testing
    fn generate_verification_token(&self) -> String {
        #[cfg(any(test, feature = "test-mode"))]
        {
            debug!("Generating test/QA verification token");
            "VALIDATION_TOKEN".to_string()
        }
        #[cfg(not(any(test, feature = "test-mode")))] 
        Uuid::new_v4().to_string()
    }

    pub async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, AuthError> {
        // Check if user already exists by email
        if let Ok(Some(existing_email)) = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
        {
            // User exists - check if they already have password auth
            let existing_user = self
                .user_repository
                .find_by_id(existing_email.user_id)
                .await
                .map_err(|e| {
                    AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
                })?
                .ok_or(AuthError::UserNotFound)?;

            if existing_user.password_hash.is_some() && existing_user.username.is_some() {
                return Err(AuthError::UserAlreadyExists);
            }

            // Add password to existing user (OAuth user adding password auth)
            let password_hash = self
                .password_service
                .hash_password(&request.password)
                .await?;
            let mut updated_user = existing_user.clone();
            updated_user.password_hash = Some(password_hash);

            let updated_user = self
                .user_repository
                .update(updated_user)
                .await
                .map_err(|e| {
                    AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
                })?;

            // Generate tokens if user has completed registration (has username)
            if let Some(username) = &updated_user.username {
                let access_token = self
                    .token_service
                    .generate_access_token(updated_user.id)
                    .await
                    .map_err(|e| AuthError::TokenServiceError(Box::new(e)))?;

                let refresh_token = self
                    .token_service
                    .generate_refresh_token(updated_user.id)
                    .await
                    .map_err(|e| AuthError::TokenServiceError(Box::new(e)))?;

                return Ok(SignupResponse::ExistingUser {
                    user: UserProfile {
                        id: updated_user.id,
                        username: Some(username.clone()),
                        email: request.email,
                        avatar: updated_user.avatar_url,
                    },
                    access_token: access_token.token,
                    expires_in: (access_token.expires_at.timestamp() - Utc::now().timestamp())
                        as u64,
                    refresh_token: refresh_token.token,
                    message: "Password authentication added to existing account".to_string(),
                });
            }

            // Generate registration token (RSA-signed JWT)
            let registration_token = self
                .registration_token_service
                .generate_registration_token(updated_user.id, request.email.clone())
                .map_err(|e| AuthError::RepositoryError(e))?;

            return Ok(SignupResponse::RegistrationRequired {
                user: IncompleteUserProfile {
                    id: updated_user.id,
                    email: request.email,
                },
                registration_token,
                requires_username: true,
                message: "Account created. Please choose a username to complete registration"
                    .to_string(),
            });
        }

        // Create new user without username (incomplete registration)
        let password_hash = self
            .password_service
            .hash_password(&request.password)
            .await?;
        let user = User::new_incomplete_with_password(password_hash, None);

        // Save the user
        let created_user =
            self.user_repository.create(user).await.map_err(|e| {
                AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
            })?;

        // Create the user email (unverified initially)
        let user_email = UserEmail::new(
            created_user.id,
            request.email.clone(),
            true,  // Primary email
            false, // Not verified yet
        );

        // Save the user email
        let _created_email = self
            .user_email_repository
            .create(user_email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Generate verification token
        let verification_token = self.generate_verification_token();
        let email_verification = EmailVerification::new(
            request.email.clone(),
            verification_token,
            24, // Expires in 24 hours
        );

        // Save the verification token
        self.email_verification_repository
            .create(&email_verification)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Generate registration token (RSA-signed JWT)
        let registration_token = self
            .registration_token_service
            .generate_registration_token(created_user.id, request.email.clone())
            .map_err(|e| AuthError::RepositoryError(e))?;

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
        let password_hash = user.password_hash.ok_or(AuthError::InvalidCredentials)?;

        // Verify password first
        let is_valid = self
            .password_service
            .verify_password(&request.password, &password_hash)
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
                .map_err(|e| AuthError::RepositoryError(e))?;

            return Ok(LoginResponse::RegistrationIncomplete {
                registration_token,
                message: "Account exists but registration is incomplete. Please complete registration with a username.".to_string(),
            });
        }

        // Only check email verification for complete users
        if !user_email.is_verified {
            return Err(AuthError::EmailNotVerified);
        }

        // Generate JWT access token and refresh token
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

        // Publish UserLoggedIn event
        let event = DomainEvent::UserLoggedIn(UserLoggedInEvent::new(
            user.id,
            request.email.clone(),
            "email_password".to_string(),
        ));

        if let Err(e) = self.event_publisher.publish(event).await {
            tracing::warn!("Failed to publish UserLoggedIn event: {}", e);
            // Don't fail the login for event publishing errors
        }

        Ok(LoginResponse::Success {
            user: UserProfile {
                id: user.id,
                username: user.username.clone(),
                email: request.email,
                avatar: user.avatar_url,
            },
            access_token: access_token.token,
            expires_in: (access_token.expires_at.timestamp() - Utc::now().timestamp()) as u64,
            refresh_token: refresh_token.token,
        })
    }

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

        let verification = match verification {
            Some(v) => v,
            None => {
                // Check if the email exists in user_emails to distinguish between
                // nonexistent email (404) vs invalid token (400)
                let user_email_exists = self
                    .user_email_repository
                    .find_by_email(&request.email)
                    .await
                    .map_err(|e| {
                        AuthError::RepositoryError(DomainError::RepositoryError(e.to_string()))
                    })?
                    .is_some();

                if !user_email_exists {
                    return Err(AuthError::EmailNotFound);
                } else {
                    return Err(AuthError::InvalidVerificationToken);
                }
            }
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

        // Clean up verification token after successful verification
        if let Err(e) = self
            .email_verification_repository
            .delete_by_id(verification.id)
            .await
        {
            tracing::warn!(
                "Failed to delete verification token after successful verification: {}",
                e
            );
            // Try to delete by email as fallback
            if let Err(e2) = self
                .email_verification_repository
                .delete_by_email(&request.email)
                .await
            {
                tracing::error!(
                    "Failed to delete verification token by email as fallback: {}",
                    e2
                );
                // Don't fail the verification process if cleanup fails
            }
        }

        // Publish UserEmailVerified event
        let event = DomainEvent::UserEmailVerified(UserEmailVerifiedEvent::new(
            updated_email.user_id,
            request.email,
        ));

        if let Err(e) = self.event_publisher.publish(event).await {
            tracing::warn!("Failed to publish UserEmailVerified event: {}", e);
            // Don't fail the verification for event publishing errors
        }

        Ok(VerifyEmailResponse {
            message: "Email verified successfully".to_string(),
        })
    }

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
            Some(user_email) => {
                // Only proceed if email is not verified
                if !user_email.is_verified {
                    // Delete any existing verification tokens for this email first
                    if let Err(e) = self
                        .email_verification_repository
                        .delete_by_email(&request.email)
                        .await
                    {
                        tracing::warn!(
                            "Failed to delete existing verification tokens for {}: {}",
                            request.email,
                            e
                        );
                        // Continue anyway - the create might still work if there were no existing tokens
                    }

                    // Generate verification token
                    let verification_token = self.generate_verification_token();
                    let email_verification = EmailVerification::new(
                        request.email.clone(),
                        verification_token,
                        24, // Expires in 24 hours
                    );

                    // Save the verification token
                    if let Err(e) = self
                        .email_verification_repository
                        .create(&email_verification)
                        .await
                    {
                        tracing::error!("Failed to create verification token: {}", e);
                        // Continue and return success to prevent information leakage
                    } else {
                        // Fetch user to include username for event
                        if let Ok(Some(user)) =
                            self.user_repository.find_by_id(user_email.user_id).await
                        {
                            // Only publish event if user has a username (complete registration)
                            if let Some(username) = user.username {
                                // Publish event to trigger email service
                                let event = DomainEvent::UserSignedUp(UserSignedUpEvent::new(
                                    user_email.user_id,
                                    request.email.clone(),
                                    username,
                                    false,
                                ));

                                if let Err(e) = self.event_publisher.publish(event).await {
                                    tracing::warn!("Failed to publish UserSignedUp event: {}", e);
                                    // Don't fail the resend for event publishing errors
                                }
                            }
                        }
                    }
                } else {
                    // Email is already verified - log but don't reveal this information
                    tracing::debug!(
                        "Resend verification requested for already verified email: {}",
                        request.email
                    );
                }
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
}
