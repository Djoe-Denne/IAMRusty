pub mod users;
pub mod user_emails;
pub mod provider_tokens;
pub mod refresh_tokens;
pub mod email_verification;
pub mod common;

// Re-export all fixtures for easy access

/// Main entry point for DB fixtures
pub struct DbFixtures;

impl DbFixtures {
    /// Create a new user fixture builder
    pub fn user() -> users::UserFixtureBuilder {
        users::UserFixtureBuilder::new()
    }
    
    /// Create a new user email fixture builder
    pub fn user_email() -> user_emails::UserEmailFixtureBuilder {
        user_emails::UserEmailFixtureBuilder::new()
    }
    
    /// Create a new provider token fixture builder
    pub fn provider_token() -> provider_tokens::ProviderTokenFixtureBuilder {
        provider_tokens::ProviderTokenFixtureBuilder::new()
    }
    
    /// Create a new refresh token fixture builder
    pub fn refresh_token() -> refresh_tokens::RefreshTokenFixtureBuilder {
        refresh_tokens::RefreshTokenFixtureBuilder::new()
    }
    
    /// Create a new email verification fixture builder
    pub fn email_verification() -> email_verification::EmailVerificationFixtureBuilder {
        email_verification::EmailVerificationFixtureBuilder::new()
    }
} 