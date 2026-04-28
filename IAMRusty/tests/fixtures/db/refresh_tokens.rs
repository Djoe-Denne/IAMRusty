use chrono::Utc;
use rustycog_testing::db::{CommittedFixture, DbFixture, TestData};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types
use iam_infra::repository::entity::refresh_tokens::{
    ActiveModel as RefreshTokenActiveModel, Entity as RefreshTokensEntity,
    Model as RefreshTokenModel,
};

/// Refresh token fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct RefreshTokenFixtureBuilder {
    id: Option<Uuid>,
    user_id: Option<Uuid>,
    token: Option<String>,
    is_valid: Option<bool>,
    created_at: Option<DateTimeWithTimeZone>,
    expires_at: Option<DateTimeWithTimeZone>,
}

impl RefreshTokenFixtureBuilder {
    /// Create a new refresh token fixture builder
    pub const fn new() -> Self {
        Self {
            id: None,
            user_id: None,
            token: None,
            is_valid: None,
            created_at: None,
            expires_at: None,
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

    /// Set the token string
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set whether the token is valid
    pub const fn is_valid(mut self, is_valid: bool) -> Self {
        self.is_valid = Some(is_valid);
        self
    }

    /// Set the `created_at` timestamp
    pub const fn created_at(mut self, created_at: DateTimeWithTimeZone) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the `expires_at` timestamp
    pub const fn expires_at(mut self, expires_at: DateTimeWithTimeZone) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Commit the refresh token to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<RefreshTokenFixture, DbErr> {
        let fixture = DbFixture::commit(self, &db).await?;
        Ok(RefreshTokenFixture { inner: fixture })
    }

    // Factory methods for common refresh token types

    /// Create a valid refresh token
    pub fn valid(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .token(TestData::jwt_token())
            .is_valid(true)
            .expires_at(TestData::future())
    }

    /// Create an expired refresh token
    pub fn expired(self, user_id: Uuid) -> Self {
        let past = (Utc::now() - chrono::Duration::hours(1)).into();
        self.user_id(user_id)
            .token(TestData::jwt_token())
            .is_valid(true)
            .expires_at(past)
    }

    /// Create an invalid refresh token
    pub fn invalid(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .token(TestData::jwt_token())
            .is_valid(false)
            .expires_at(TestData::future())
    }

    /// Create Arthur's refresh token
    pub fn arthur_valid(self, user_id: Uuid) -> Self {
        self.valid(user_id).token("arthur_refresh_token_123")
    }

    /// Create Bob's refresh token
    pub fn bob_valid(self, user_id: Uuid) -> Self {
        self.valid(user_id).token("bob_refresh_token_456")
    }

    /// Create Alice's refresh token
    pub fn alice_valid(self, user_id: Uuid) -> Self {
        self.valid(user_id).token("alice_refresh_token_789")
    }

    /// Create Charlie's refresh token
    pub fn charlie_valid(self, user_id: Uuid) -> Self {
        self.valid(user_id).token("charlie_refresh_token_012")
    }

    /// Create a refresh token that expires soon (in 5 minutes)
    pub fn expires_soon(self, user_id: Uuid) -> Self {
        let soon = (Utc::now() + chrono::Duration::minutes(5)).into();
        self.user_id(user_id)
            .token(TestData::jwt_token())
            .is_valid(true)
            .expires_at(soon)
    }
}

impl Default for RefreshTokenFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<RefreshTokensEntity, RefreshTokenModel, RefreshTokenActiveModel>
    for RefreshTokenFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<RefreshTokenModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> RefreshTokenActiveModel {
        let now = TestData::now_with_tz();

        RefreshTokenActiveModel {
            id: ActiveValue::Set(self.id.unwrap_or_else(TestData::uuid)),
            user_id: ActiveValue::Set(self.user_id.expect("user_id is required")),
            token: ActiveValue::Set(self.token.clone().unwrap_or_else(TestData::jwt_token)),
            is_valid: ActiveValue::Set(self.is_valid.unwrap_or(true)),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
            expires_at: ActiveValue::Set(self.expires_at.unwrap_or_else(TestData::future)),
        }
    }
}

/// Committed refresh token fixture
#[derive(Debug, Clone)]
pub struct RefreshTokenFixture {
    inner: CommittedFixture<RefreshTokenModel>,
}

impl RefreshTokenFixture {
    /// Check if the refresh token exists in the database and matches the fixture
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;

        // Find the refresh token in the database by ID
        let db_token = RefreshTokensEntity::find_by_id(self.model().id)
            .one(&*db)
            .await?;

        match db_token {
            Some(token) => {
                // Compare all fields
                Ok(token.id == self.model().id
                    && token.user_id == self.model().user_id
                    && token.token == self.model().token
                    && token.is_valid == self.model().is_valid
                    && token.created_at == self.model().created_at
                    && token.expires_at == self.model().expires_at)
            }
            None => Ok(false), // Token doesn't exist in database
        }
    }

    /// Get the refresh token model
    pub const fn model(&self) -> &RefreshTokenModel {
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

    /// Get the token string
    pub fn token(&self) -> &str {
        &self.model().token
    }

    /// Check if the token is valid
    pub const fn is_valid(&self) -> bool {
        self.model().is_valid
    }

    /// Get the `created_at` timestamp
    pub const fn created_at(&self) -> DateTimeWithTimeZone {
        self.model().created_at
    }

    /// Get the `expires_at` timestamp
    pub const fn expires_at(&self) -> DateTimeWithTimeZone {
        self.model().expires_at
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let now: DateTimeWithTimeZone = Utc::now().into();
        self.expires_at() < now
    }

    /// Check if the token is currently usable (valid and not expired)
    pub fn is_usable(&self) -> bool {
        self.is_valid() && !self.is_expired()
    }
}
