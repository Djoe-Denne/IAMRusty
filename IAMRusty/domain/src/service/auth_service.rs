use chrono::Duration;
use crate::entity::token::{JwkSet, TokenClaims};
use crate::error::DomainError;
use crate::port::service::JwtTokenEncoder;

/// Service for JWT token operations
pub struct TokenService {
    user_repository: Box<dyn UserRepository>,
    user_email_repository: Box<dyn UserEmailRepository>,
    email_verification_repository: Box<dyn EmailVerificationRepository>,
    password_service: Box<dyn PasswordService>,
    token_service: Box<dyn AuthTokenService>,
    event_publisher: Box<dyn EventPublisher>,
}

impl TokenService {
    /// Create a new token service
    pub fn new(
        user_repository: Box<dyn UserRepository>,
        user_email_repository: Box<dyn UserEmailRepository>,
        email_verification_repository: Box<dyn EmailVerificationRepository>,
        password_service: Box<dyn PasswordService>,
        token_service: Box<dyn AuthTokenService>,
        event_publisher: Box<dyn EventPublisher>,
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
            .generate_access_token(user.id)
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

        Ok(LoginResponse {
            user: UserProfile {
                id: user.id,
                username: user.username,
                email: request.email,
                avatar: user.avatar_url,
            },
            token: token.token,
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
