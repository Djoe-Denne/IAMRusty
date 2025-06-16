use super::common::{CommittedFixture, DbFixture, TestData};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types
use infra::repository::entity::provider_tokens::{
    ActiveModel as ProviderTokenActiveModel, Entity as ProviderTokensEntity,
    Model as ProviderTokenModel,
};

/// Provider token fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct ProviderTokenFixtureBuilder {
    id: Option<i32>,
    user_id: Option<Uuid>,
    provider: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<Option<String>>,
    expires_in: Option<Option<i32>>,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
    provider_user_id: Option<String>,
}

impl ProviderTokenFixtureBuilder {
    /// Create a new provider token fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            user_id: None,
            provider: None,
            access_token: None,
            refresh_token: None,
            expires_in: None,
            created_at: None,
            updated_at: None,
            provider_user_id: None,
        }
    }

    /// Set the token ID (auto-increment, usually not set manually)
    pub fn id(mut self, id: i32) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the user ID
    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the provider name
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the access token
    pub fn access_token(mut self, access_token: impl Into<String>) -> Self {
        self.access_token = Some(access_token.into());
        self
    }

    /// Set the refresh token
    pub fn refresh_token(mut self, refresh_token: Option<String>) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    /// Set the expires_in value
    pub fn expires_in(mut self, expires_in: Option<i32>) -> Self {
        self.expires_in = Some(expires_in);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: NaiveDateTime) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    pub fn updated_at(mut self, updated_at: NaiveDateTime) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Set the provider user ID
    pub fn provider_user_id(mut self, provider_user_id: impl Into<String>) -> Self {
        self.provider_user_id = Some(provider_user_id.into());
        self
    }

    /// Commit the provider token to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<ProviderTokenFixture, DbErr> {
        let fixture = DbFixture::commit(self, &*db).await?;
        Ok(ProviderTokenFixture { inner: fixture })
    }

    // Factory methods for common provider token types

    /// Create a GitHub token
    pub fn github(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .provider("github")
            .access_token(TestData::access_token())
            .refresh_token(Some(TestData::refresh_token()))
            .expires_in(Some(3600))
            .provider_user_id(TestData::provider_user_id())
    }

    /// Create a GitLab token
    pub fn gitlab(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .provider("gitlab")
            .access_token(TestData::access_token())
            .refresh_token(Some(TestData::refresh_token()))
            .expires_in(Some(7200))
            .provider_user_id(TestData::provider_user_id())
    }

    /// Create an expired GitHub token
    pub fn github_expired(self, user_id: Uuid) -> Self {
        self.github(user_id).expires_in(Some(0))
    }

    /// Create a GitHub token without refresh token
    pub fn github_no_refresh(self, user_id: Uuid) -> Self {
        self.github(user_id).refresh_token(None)
    }

    /// Create Arthur's GitHub token
    pub fn arthur_github(self, user_id: Uuid) -> Self {
        self.github(user_id).provider_user_id("123456")
    }

    /// Create Bob's GitHub token
    pub fn bob_github(self, user_id: Uuid) -> Self {
        self.github(user_id).provider_user_id("789012")
    }

    /// Create Alice's GitLab token
    pub fn alice_gitlab(self, user_id: Uuid) -> Self {
        self.gitlab(user_id).provider_user_id("alice123")
    }

    /// Create Charlie's GitLab token
    pub fn charlie_gitlab(self, user_id: Uuid) -> Self {
        self.gitlab(user_id).provider_user_id("charlie456")
    }

    /// Create Charlie's GitHub token
    pub fn charlie_github(self, user_id: Uuid) -> Self {
        self.github(user_id).provider_user_id("charlie789")
    }

    /// Create Diana's GitHub token
    pub fn diana_github(self, user_id: Uuid) -> Self {
        self.github(user_id).provider_user_id("diana321")
    }
}

impl Default for ProviderTokenFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<ProviderTokensEntity, ProviderTokenModel, ProviderTokenActiveModel>
    for ProviderTokenFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<ProviderTokenModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> ProviderTokenActiveModel {
        let now = TestData::now_naive();

        ProviderTokenActiveModel {
            id: if let Some(id) = self.id {
                ActiveValue::Set(id)
            } else {
                ActiveValue::NotSet
            },
            user_id: ActiveValue::Set(self.user_id.expect("user_id is required")),
            provider: ActiveValue::Set(
                self.provider
                    .clone()
                    .unwrap_or_else(|| "github".to_string()),
            ),
            access_token: ActiveValue::Set(
                self.access_token
                    .clone()
                    .unwrap_or_else(TestData::access_token),
            ),
            refresh_token: ActiveValue::Set(
                self.refresh_token
                    .clone()
                    .unwrap_or(Some(TestData::refresh_token())),
            ),
            expires_in: ActiveValue::Set(self.expires_in.unwrap_or(Some(3600))),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
            updated_at: ActiveValue::Set(self.updated_at.unwrap_or(now)),
            provider_user_id: ActiveValue::Set(
                self.provider_user_id
                    .clone()
                    .unwrap_or_else(TestData::provider_user_id),
            ),
        }
    }
}

/// Committed provider token fixture
#[derive(Debug, Clone)]
pub struct ProviderTokenFixture {
    inner: CommittedFixture<ProviderTokenModel>,
}

impl ProviderTokenFixture {
    /// Check if the provider token exists in the database and matches the fixture
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;

        // Find the provider token in the database by ID
        let db_token = ProviderTokensEntity::find_by_id(self.model().id)
            .one(&*db)
            .await?;

        match db_token {
            Some(token) => {
                // Compare all fields
                Ok(token.id == self.model().id
                    && token.user_id == self.model().user_id
                    && token.provider == self.model().provider
                    && token.access_token == self.model().access_token
                    && token.refresh_token == self.model().refresh_token
                    && token.expires_in == self.model().expires_in
                    && token.created_at == self.model().created_at
                    && token.updated_at == self.model().updated_at
                    && token.provider_user_id == self.model().provider_user_id)
            }
            None => Ok(false), // Token doesn't exist in database
        }
    }

    /// Get the provider token model
    pub fn model(&self) -> &ProviderTokenModel {
        self.inner.model()
    }

    /// Get the token ID
    pub fn id(&self) -> i32 {
        self.model().id
    }

    /// Get the user ID
    pub fn user_id(&self) -> Uuid {
        self.model().user_id
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.model().provider
    }

    /// Get the access token
    pub fn access_token(&self) -> &str {
        &self.model().access_token
    }

    /// Get the refresh token
    pub fn refresh_token(&self) -> Option<&String> {
        self.model().refresh_token.as_ref()
    }

    /// Get the expires_in value
    pub fn expires_in(&self) -> Option<i32> {
        self.model().expires_in
    }

    /// Get the created_at timestamp
    pub fn created_at(&self) -> NaiveDateTime {
        self.model().created_at
    }

    /// Get the updated_at timestamp
    pub fn updated_at(&self) -> NaiveDateTime {
        self.model().updated_at
    }

    /// Get the provider user ID
    pub fn provider_user_id(&self) -> &str {
        &self.model().provider_user_id
    }
}
