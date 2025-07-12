use rustycog_testing::db::{CommittedFixture, DbFixture, TestData};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types
use iam_infra::repository::entity::users::{
    ActiveModel as UserActiveModel, Entity as UsersEntity, Model as UserModel,
};

/// User fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct UserFixtureBuilder {
    id: Option<Uuid>,
    username: Option<String>,
    password_hash: Option<Option<String>>,
    avatar_url: Option<Option<String>>,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
}

impl UserFixtureBuilder {
    /// Create a new user fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            username: None,
            password_hash: None,
            avatar_url: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the user ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the username
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password hash
    pub fn password_hash(mut self, password_hash: impl Into<String>) -> Self {
        self.password_hash = Some(Some(password_hash.into()));
        self
    }

    /// Set the avatar URL
    pub fn avatar_url(mut self, avatar_url: Option<String>) -> Self {
        self.avatar_url = Some(avatar_url);
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

    /// Commit the user to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<UserFixture, DbErr> {
        let fixture = DbFixture::commit(self, &*db).await?;
        Ok(UserFixture { inner: fixture })
    }

    // Factory methods for common user types

    /// Create Arthur user (GitHub user)
    pub fn arthur(self) -> Self {
        self.username("arthur").avatar_url(Some(
            "https://avatars.githubusercontent.com/u/123456".to_string(),
        ))
    }

    /// Create Bob user (GitHub user)
    pub fn bob(self) -> Self {
        self.username("bob").avatar_url(Some(
            "https://avatars.githubusercontent.com/u/789012".to_string(),
        ))
    }

    /// Create Alice user (GitLab user)
    pub fn alice(self) -> Self {
        self.username("alice").avatar_url(Some(
            "https://gitlab.com/uploads/-/system/user/avatar/123/avatar.png".to_string(),
        ))
    }

    /// Create Charlie user (GitLab user)
    pub fn charlie(self) -> Self {
        self.username("charlie").avatar_url(Some(
            "https://gitlab.com/uploads/-/system/user/avatar/456/avatar.png".to_string(),
        ))
    }

    /// Create Diana user (GitHub user)
    pub fn diana(self) -> Self {
        self.username("diana").avatar_url(Some(
            "https://avatars.githubusercontent.com/u/321654".to_string(),
        ))
    }

    /// Create a user without avatar
    pub fn no_avatar(self) -> Self {
        self.username("no_avatar_user").avatar_url(None)
    }

    /// Create a user with email/password authentication
    pub fn with_password(
        self,
        username: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        self.username(username)
            .password_hash(password_hash)
            .avatar_url(None)
    }
}

impl Default for UserFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<UsersEntity, UserModel, UserActiveModel> for UserFixtureBuilder {
    async fn commit(self, db: &DatabaseConnection) -> Result<CommittedFixture<UserModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> UserActiveModel {
        let now = TestData::now_naive();

        UserActiveModel {
            id: ActiveValue::Set(self.id.unwrap_or_else(TestData::uuid)),
            username: ActiveValue::Set(self.username.clone()),
            password_hash: ActiveValue::Set(self.password_hash.clone().unwrap_or(None)),
            avatar_url: ActiveValue::Set(self.avatar_url.clone().unwrap_or(None)),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
            updated_at: ActiveValue::Set(self.updated_at.unwrap_or(now)),
        }
    }
}

/// Committed user fixture
#[derive(Debug, Clone)]
pub struct UserFixture {
    inner: CommittedFixture<UserModel>,
}

impl UserFixture {
    /// Check if the user exists in the database and matches the fixture
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;

        // Find the user in the database by ID
        let db_user = UsersEntity::find_by_id(self.model().id).one(&*db).await?;

        match db_user {
            Some(user) => {
                // Compare all fields
                Ok(user.id == self.model().id
                    && user.username == self.model().username
                    && user.avatar_url == self.model().avatar_url
                    && user.created_at == self.model().created_at
                    && user.updated_at == self.model().updated_at)
            }
            None => Ok(false), // User doesn't exist in database
        }
    }

    /// Get the user model
    pub fn model(&self) -> &UserModel {
        self.inner.model()
    }

    /// Get the user ID
    pub fn id(&self) -> Uuid {
        self.model().id
    }

    /// Get the username
    pub fn username(&self) -> Option<&str> {
        self.model().username.as_deref()
    }

    /// Get the avatar URL
    pub fn avatar_url(&self) -> Option<&String> {
        self.model().avatar_url.as_ref()
    }

    /// Get the created_at timestamp
    pub fn created_at(&self) -> NaiveDateTime {
        self.model().created_at
    }

    /// Get the updated_at timestamp
    pub fn updated_at(&self) -> NaiveDateTime {
        self.model().updated_at
    }
}
