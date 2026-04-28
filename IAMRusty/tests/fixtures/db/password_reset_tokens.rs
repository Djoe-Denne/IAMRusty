use chrono::{DateTime, Utc};
use iam_domain::entity::password_reset_token::PasswordResetToken;
use rustycog_testing::db::{CommittedFixture, DbFixture, TestData};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types
use iam_infra::repository::entity::password_reset_tokens::{
    ActiveModel as PasswordResetTokenActiveModel, Entity as PasswordResetTokensEntity,
    Model as PasswordResetTokenModel,
};

/// Password reset token fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct PasswordResetTokenFixtureBuilder {
    id: Option<Uuid>,
    user_id: Option<Uuid>,
    raw_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    used_at: Option<Option<DateTime<Utc>>>,
}

impl PasswordResetTokenFixtureBuilder {
    /// Create a new password reset token fixture builder
    pub const fn new() -> Self {
        Self {
            id: None,
            user_id: None,
            raw_token: None,
            expires_at: None,
            created_at: None,
            used_at: None,
        }
    }

    /// Set the token ID
    pub const fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the user ID
    pub const fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the raw token (will be hashed when stored)
    pub fn raw_token(mut self, raw_token: impl Into<String>) -> Self {
        self.raw_token = Some(raw_token.into());
        self
    }

    /// Set the `expires_at` timestamp
    pub const fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set the `created_at` timestamp
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the `used_at` timestamp (None means not used)
    pub const fn used_at(mut self, used_at: Option<DateTime<Utc>>) -> Self {
        self.used_at = Some(used_at);
        self
    }

    /// Mark as used now
    pub fn mark_as_used(mut self) -> Self {
        self.used_at = Some(Some(Utc::now()));
        self
    }

    /// Mark as not used
    pub const fn mark_as_not_used(mut self) -> Self {
        self.used_at = Some(None);
        self
    }

    /// Commit the password reset token to the database
    pub async fn commit(
        self,
        db: Arc<DatabaseConnection>,
    ) -> Result<PasswordResetTokenFixture, DbErr> {
        let raw_token = self
            .raw_token
            .clone()
            .unwrap_or_else(Self::generate_raw_token);
        let fixture = DbFixture::commit(self, &db).await?;
        Ok(PasswordResetTokenFixture {
            inner: fixture,
            raw_token,
        })
    }

    // Factory methods for common password reset token scenarios

    /// Create a valid password reset token
    pub fn valid(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .raw_token(Self::generate_raw_token())
            .expires_at(Self::default_expiry())
            .mark_as_not_used()
    }

    /// Create an expired password reset token
    pub fn expired(self, user_id: Uuid) -> Self {
        let past = Utc::now() - chrono::Duration::hours(1);
        self.user_id(user_id)
            .raw_token(Self::generate_raw_token())
            .expires_at(past)
            .mark_as_not_used()
    }

    /// Create a used password reset token
    pub fn used(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .raw_token(Self::generate_raw_token())
            .expires_at(Self::default_expiry())
            .mark_as_used()
    }

    /// Create a token that expires soon (in 5 minutes)
    pub fn expires_soon(self, user_id: Uuid) -> Self {
        let soon = Utc::now() + chrono::Duration::minutes(5);
        self.user_id(user_id)
            .raw_token(Self::generate_raw_token())
            .expires_at(soon)
            .mark_as_not_used()
    }

    /// Create Arthur's password reset token
    pub fn arthur_reset(self, user_id: Uuid) -> Self {
        self.valid(user_id).raw_token("arthur_reset_token_123")
    }

    /// Create Bob's password reset token
    pub fn bob_reset(self, user_id: Uuid) -> Self {
        self.valid(user_id).raw_token("bob_reset_token_456")
    }

    /// Create Alice's password reset token
    pub fn alice_reset(self, user_id: Uuid) -> Self {
        self.valid(user_id).raw_token("alice_reset_token_789")
    }

    /// Create a token for test user
    pub fn test_reset(self, user_id: Uuid) -> Self {
        self.valid(user_id).raw_token(Self::generate_raw_token())
    }

    // Helper methods

    /// Generate a password reset token
    fn generate_raw_token() -> String {
        PasswordResetToken::generate_raw_token()
    }

    /// Default expiry time (1 hour from now)
    fn default_expiry() -> DateTime<Utc> {
        Utc::now() + chrono::Duration::hours(1)
    }
}

impl Default for PasswordResetTokenFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<PasswordResetTokensEntity, PasswordResetTokenModel, PasswordResetTokenActiveModel>
    for PasswordResetTokenFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<PasswordResetTokenModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> PasswordResetTokenActiveModel {
        let now = Utc::now();
        let raw_token = self
            .raw_token
            .clone()
            .unwrap_or_else(Self::generate_raw_token);

        PasswordResetTokenActiveModel {
            id: ActiveValue::Set(self.id.unwrap_or_else(TestData::uuid)),
            user_id: ActiveValue::Set(self.user_id.expect("user_id is required")),
            token_hash: ActiveValue::Set(PasswordResetToken::hash_token(&raw_token)),
            expires_at: ActiveValue::Set(self.expires_at.unwrap_or_else(Self::default_expiry)),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
            used_at: ActiveValue::Set(self.used_at.unwrap_or(None)),
        }
    }
}

/// Committed password reset token fixture
pub struct PasswordResetTokenFixture {
    inner: CommittedFixture<PasswordResetTokenModel>,
    raw_token: String,
}

impl PasswordResetTokenFixture {
    /// Check if this password reset token exists in the database
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;

        if let Some(token) = PasswordResetTokensEntity::find_by_id(self.id())
            .one(&*db)
            .await?
        {
            Ok(token.user_id == self.user_id()
                && token.token_hash == PasswordResetToken::hash_token(&self.raw_token)
                && token.used_at == self.used_at())
        } else {
            Ok(false)
        }
    }

    /// Get the underlying model
    pub const fn model(&self) -> &PasswordResetTokenModel {
        self.inner.model()
    }

    /// Get the token ID
    pub const fn id(&self) -> Uuid {
        self.model().id
    }

    /// Get the user ID
    pub const fn user_id(&self) -> Uuid {
        self.model().user_id
    }

    /// Get the raw token (for API testing)
    pub fn token(&self) -> &str {
        &self.raw_token
    }

    /// Get the token hash
    pub fn token_hash(&self) -> &str {
        &self.model().token_hash
    }

    /// Get the `expires_at` timestamp
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.model().expires_at
    }

    /// Get the `created_at` timestamp
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.model().created_at
    }

    /// Get the `used_at` timestamp
    pub const fn used_at(&self) -> Option<DateTime<Utc>> {
        self.model().used_at
    }

    /// Check if the token is used
    pub const fn is_used(&self) -> bool {
        self.model().is_used()
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        self.model().is_expired()
    }

    /// Check if the token is valid (not expired and not used)
    pub fn is_valid(&self) -> bool {
        self.model().is_valid()
    }

    /// Check if the token expires soon (in less than 10 minutes)
    pub fn expires_soon(&self) -> bool {
        let soon = Utc::now() + chrono::Duration::minutes(10);
        self.expires_at() < soon
    }
}

// TestData methods are now available through rustycog_testing
