use async_trait::async_trait;
use domain::entity::events::{DomainEvent, UserSignedUpEvent, UserEmailVerifiedEvent, UserLoggedInEvent};
use domain::entity::user::User;
use domain::entity::user_email::UserEmail;
use domain::entity::email_verification::EmailVerification;
use domain::port::repository::{UserRepository, UserEmailRepository, EmailVerificationRepository};
use domain::port::event_publisher::EventPublisher;
use domain::error::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use rand::{distributions::Alphanumeric, Rng};

/// Auth use case errors
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
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Signup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupResponse {
    pub message: String,
}

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub token: String,
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

/// User profile for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
}

/// Password service trait for dependency injection
#[async_trait]
pub trait PasswordService: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, AuthError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
}

/// Token service trait for dependency injection
#[async_trait]
pub trait TokenService: Send + Sync {
    async fn generate_jwt_for_user(&self, user_id: Uuid) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

/// Auth use case trait
#[async_trait]
pub trait AuthUseCase: Send + Sync {
    async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, AuthError>;
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AuthError>;
    async fn verify_email(&self, request: VerifyEmailRequest) -> Result<VerifyEmailResponse, AuthError>;
}

/// Implementation of the auth use case
pub struct AuthUseCaseImpl<UR, UER, EVR, PS, TS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: TokenService,
    EP: EventPublisher,
{
    user_repository: Arc<UR>,
    user_email_repository: Arc<UER>,
    email_verification_repository: Arc<EVR>,
    password_service: Arc<PS>,
    token_service: Arc<TS>,
    event_publisher: Arc<EP>,
}

impl<UR, UER, EVR, PS, TS, EP> AuthUseCaseImpl<UR, UER, EVR, PS, TS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: TokenService,
    EP: EventPublisher,
{
    pub fn new(
        user_repository: Arc<UR>,
        user_email_repository: Arc<UER>,
        email_verification_repository: Arc<EVR>,
        password_service: Arc<PS>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
    ) -> Self {
        Self {
            user_repository,
            user_email_repository,
            email_verification_repository,
            password_service,
            token_service,
            event_publisher,
        }
    }

    /// Generate a random verification token
    fn generate_verification_token() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }
}

#[async_trait]
impl<UR, UER, EVR, PS, TS, EP> AuthUseCase for AuthUseCaseImpl<UR, UER, EVR, PS, TS, EP>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    EVR: EmailVerificationRepository + Send + Sync,
    PS: PasswordService + Send + Sync,
    TS: TokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
{
    async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, AuthError> {
        // Check if user already exists by email
        if let Ok(Some(_)) = self.user_email_repository.find_by_email(&request.email).await {
            return Err(AuthError::UserAlreadyExists);
        }

        // Hash the password
        let password_hash = self.password_service.hash_password(&request.password).await?;

        // Create the user with password
        let user = User::new_with_password(
            request.username.clone(),
            password_hash,
            None, // No avatar initially
        );

        // Save the user
        let _created_user = self.user_repository.create(user).await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Create the user email (unverified initially)
        let user_email = UserEmail::new(
            _created_user.id,
            request.email.clone(),
            true, // Primary email
            false, // Not verified yet
        );

        // Save the user email
        let _created_email = self.user_email_repository.create(user_email).await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Generate verification token
        let verification_token = Self::generate_verification_token();
        let email_verification = EmailVerification::new(
            request.email.clone(),
            verification_token,
            24, // Expires in 24 hours
        );

        // Save the verification token
        self.email_verification_repository.create(&email_verification).await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Publish UserSignedUp event
        let event = DomainEvent::UserSignedUp(UserSignedUpEvent::new(
            _created_user.id,
            request.email,
            request.username,
            false, // Email not verified yet
        ));

        if let Err(e) = self.event_publisher.publish(event).await {
            tracing::warn!("Failed to publish UserSignedUp event: {}", e);
            // Don't fail the signup for event publishing errors
        }

        Ok(SignupResponse {
            message: "User created successfully. Please check your email for verification instructions.".to_string(),
        })
    }

    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AuthError> {
        // Find user by email
        let user_email = self.user_email_repository
            .find_by_email(&request.email)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::InvalidCredentials)?; // Don't leak user existence

        // Check if email is verified
        if !user_email.is_verified {
            return Err(AuthError::EmailNotVerified);
        }

        // Get the user
        let user = self.user_repository
            .find_by_id(user_email.user_id)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check if user has a password (should not happen if signup worked correctly)
        let password_hash = user.password_hash
            .ok_or(AuthError::InvalidCredentials)?;

        // Verify password
        let is_valid = self.password_service
            .verify_password(&request.password, &password_hash)
            .await?;

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        // Generate JWT token
        let token = self.token_service
            .generate_jwt_for_user(user.id)
            .await
            .map_err(AuthError::TokenServiceError)?;

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

        Ok(LoginResponse {
            user: UserProfile {
                id: user.id,
                username: user.username,
                email: request.email,
                avatar: user.avatar_url,
            },
            token,
        })
    }

    async fn verify_email(&self, request: VerifyEmailRequest) -> Result<VerifyEmailResponse, AuthError> {
        // Find verification token
        let verification = self.email_verification_repository
            .find_by_email_and_token(&request.email, &request.verification_token)
            .await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        let verification = match verification {
            Some(v) => v,
            None => {
                // Check if the email exists in user_emails to distinguish between 
                // nonexistent email (404) vs invalid token (400)
                let user_email_exists = self.user_email_repository
                    .find_by_email(&request.email)
                    .await
                    .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?
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
        let mut user_email = self.user_email_repository
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
        let _updated_email = self.user_email_repository.update(user_email).await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Clean up verification token
        self.email_verification_repository.delete_by_email(&request.email).await
            .map_err(|e| AuthError::RepositoryError(DomainError::RepositoryError(e.to_string())))?;

        // Publish UserEmailVerified event
        let event = DomainEvent::UserEmailVerified(UserEmailVerifiedEvent::new(
            _updated_email.user_id,
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
} 