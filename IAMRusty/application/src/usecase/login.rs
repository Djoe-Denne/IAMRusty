//! Login use case module for email/password authentication

use domain::service::auth_service::{
    AuthService, AuthError,
};
use domain::port::{
    repository::{UserRepository, UserEmailRepository, EmailVerificationRepository},
    service::{AuthTokenService, RegistrationTokenService},
    event_publisher::EventPublisher,
};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

// Re-export types for command handlers
pub use domain::service::auth_service::{
    SignupRequest, SignupResponse, LoginRequest, LoginResponse,
    VerifyEmailRequest, VerifyEmailResponse,
    ResendVerificationEmailRequest, ResendVerificationEmailResponse,
    PasswordService,
};

/// Login use case errors for email/password authentication
#[derive(Debug, Error)]
pub enum LoginError {
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
    
    #[error("Authentication service error: {0}")]
    AuthServiceError(String),
}

impl From<AuthError> for LoginError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::UserAlreadyExists => LoginError::UserAlreadyExists,
            AuthError::UserNotFound => LoginError::UserNotFound,
            AuthError::InvalidCredentials => LoginError::InvalidCredentials,
            AuthError::WeakPassword => LoginError::WeakPassword,
            AuthError::InvalidEmail => LoginError::InvalidEmail,
            AuthError::EmailNotVerified => LoginError::EmailNotVerified,
            AuthError::EmailNotFound => LoginError::EmailNotFound,
            AuthError::EmailAlreadyVerified => LoginError::EmailAlreadyVerified,
            AuthError::InvalidVerificationToken => LoginError::InvalidVerificationToken,
            AuthError::VerificationTokenExpired => LoginError::VerificationTokenExpired,
            _ => LoginError::AuthServiceError(error.to_string()),
        }
    }
}

/// Login use case trait for email/password authentication
#[async_trait]
pub trait LoginUseCase: Send + Sync {
    async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, LoginError>;
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, LoginError>;
    async fn verify_email(&self, request: VerifyEmailRequest) -> Result<VerifyEmailResponse, LoginError>;
    async fn resend_verification_email(&self, request: ResendVerificationEmailRequest) -> Result<ResendVerificationEmailResponse, LoginError>;
}

/// Implementation of the login use case for email/password authentication
pub struct LoginUseCaseImpl<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    auth_service: Arc<AuthService<UR, UER, EVR, PS, TS, RTS, EP>>,
}

impl<UR, UER, EVR, PS, TS, RTS, EP> LoginUseCaseImpl<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    EVR: EmailVerificationRepository,
    PS: PasswordService,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    pub fn new(auth_service: Arc<AuthService<UR, UER, EVR, PS, TS, RTS, EP>>) -> Self {
        Self {
            auth_service,
        }
    }
}

#[async_trait]
impl<UR, UER, EVR, PS, TS, RTS, EP> LoginUseCase for LoginUseCaseImpl<UR, UER, EVR, PS, TS, RTS, EP>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    EVR: EmailVerificationRepository + Send + Sync,
    PS: PasswordService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
{
    async fn signup(&self, request: SignupRequest) -> Result<SignupResponse, LoginError> {
        self.auth_service.signup(request).await.map_err(Into::into)
    }

    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, LoginError> {
        self.auth_service.login(request).await.map_err(Into::into)
    }

    async fn verify_email(&self, request: VerifyEmailRequest) -> Result<VerifyEmailResponse, LoginError> {
        self.auth_service.verify_email(request).await.map_err(Into::into)
    }

    async fn resend_verification_email(&self, request: ResendVerificationEmailRequest) -> Result<ResendVerificationEmailResponse, LoginError> {
        self.auth_service.resend_verification_email(request).await.map_err(Into::into)
    }
} 