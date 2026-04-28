// Common DB fixtures are now in rustycog-testing
// pub mod common;
pub mod email_verification;
pub mod password_reset_tokens;
pub mod provider_tokens;
pub mod refresh_tokens;
pub mod user_emails;
pub mod users;

// Re-export all fixtures for easy access
use iam_infra::auth::PasswordService;
use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;

/// Main entry point for DB fixtures
pub struct DbFixtures;

impl DbFixtures {
    /// Create a new user fixture builder
    pub const fn user() -> users::UserFixtureBuilder {
        users::UserFixtureBuilder::new()
    }

    /// Create a new user email fixture builder
    pub const fn user_email() -> user_emails::UserEmailFixtureBuilder {
        user_emails::UserEmailFixtureBuilder::new()
    }

    /// Create a new provider token fixture builder
    pub const fn provider_token() -> provider_tokens::ProviderTokenFixtureBuilder {
        provider_tokens::ProviderTokenFixtureBuilder::new()
    }

    /// Create a new refresh token fixture builder
    pub const fn refresh_token() -> refresh_tokens::RefreshTokenFixtureBuilder {
        refresh_tokens::RefreshTokenFixtureBuilder::new()
    }

    /// Create a new email verification fixture builder
    pub const fn email_verification() -> email_verification::EmailVerificationFixtureBuilder {
        email_verification::EmailVerificationFixtureBuilder::new()
    }

    /// Create a new password reset token fixture builder
    pub const fn password_reset_token() -> password_reset_tokens::PasswordResetTokenFixtureBuilder {
        password_reset_tokens::PasswordResetTokenFixtureBuilder::new()
    }

    // Helper methods for common test scenarios

    /// Create a user with email and password authentication
    pub async fn create_user_with_email_password(
        db: &DatabaseConnection,
        email: &str,
        password: &str,
        username: Option<&str>,
    ) -> Result<users::UserFixture, DbErr> {
        // Hash the password
        let password_service = PasswordService::new();
        let password_hash = password_service
            .hash_password(password)
            .map_err(|e| DbErr::Custom(format!("Failed to hash password: {e}")))?;

        // Create user
        let user = Self::user()
            .username(username.unwrap_or("testuser").to_string())
            .password_hash(password_hash)
            .commit(Arc::new(db.clone()))
            .await?;

        // Create primary email
        Self::user_email()
            .user_id(user.id())
            .email(email)
            .is_primary(true)
            .is_verified(true)
            .commit(Arc::new(db.clone()))
            .await?;

        Ok(user)
    }

    /// Create a user without username (for registration flow testing)
    pub async fn create_user_without_username(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<users::UserFixture, DbErr> {
        // Create user without username
        let user = Self::user().commit(Arc::new(db.clone())).await?;

        // Create primary email
        Self::user_email()
            .user_id(user.id())
            .email(email)
            .is_primary(true)
            .is_verified(false)
            .commit(Arc::new(db.clone()))
            .await?;

        Ok(user)
    }

    /// Create a complete test user with email and OAuth provider
    pub async fn create_user_with_oauth_provider(
        db: &DatabaseConnection,
        email: &str,
        username: &str,
        provider: &str,
    ) -> Result<(users::UserFixture, provider_tokens::ProviderTokenFixture), DbErr> {
        // Create user
        let user = Self::user()
            .username(username)
            .commit(Arc::new(db.clone()))
            .await?;

        // Create primary email
        Self::user_email()
            .user_id(user.id())
            .email(email)
            .is_primary(true)
            .is_verified(true)
            .commit(Arc::new(db.clone()))
            .await?;

        // Create provider token
        let provider_token = match provider {
            "github" => {
                Self::provider_token()
                    .github(user.id())
                    .commit(Arc::new(db.clone()))
                    .await?
            }
            "gitlab" => {
                Self::provider_token()
                    .gitlab(user.id())
                    .commit(Arc::new(db.clone()))
                    .await?
            }
            _ => return Err(DbErr::Custom(format!("Unsupported provider: {provider}"))),
        };

        Ok((user, provider_token))
    }
}
