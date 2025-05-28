use sea_orm::{DatabaseConnection, ActiveValue, DbErr, ActiveModelTrait};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDateTime};
use super::common::{DbFixture, CommittedFixture, TestData};

// Import the entity types
use infra::repository::entity::user_emails::{Entity as UserEmailsEntity, Model as UserEmailModel, ActiveModel as UserEmailActiveModel};

/// User email fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct UserEmailFixtureBuilder {
    id: Option<Uuid>,
    user_id: Option<Uuid>,
    email: Option<String>,
    is_primary: Option<bool>,
    is_verified: Option<bool>,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
}

impl UserEmailFixtureBuilder {
    /// Create a new user email fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            user_id: None,
            email: None,
            is_primary: None,
            is_verified: None,
            created_at: None,
            updated_at: None,
        }
    }
    
    /// Set the email ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }
    
    /// Set the user ID
    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Set the email address
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
    
    /// Set whether this is the primary email
    pub fn is_primary(mut self, is_primary: bool) -> Self {
        self.is_primary = Some(is_primary);
        self
    }
    
    /// Set whether this email is verified
    pub fn is_verified(mut self, is_verified: bool) -> Self {
        self.is_verified = Some(is_verified);
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
    
    /// Commit the user email to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<UserEmailFixture, DbErr> {
        let fixture = DbFixture::commit(self, &*db).await?;
        Ok(UserEmailFixture { inner: fixture })
    }
    
    // Factory methods for common email types
    
    /// Create a primary verified email
    pub fn primary_verified(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email(TestData::email())
            .is_primary(true)
            .is_verified(true)
    }
    
    /// Create a secondary unverified email
    pub fn secondary_unverified(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email(TestData::email())
            .is_primary(false)
            .is_verified(false)
    }
    
    /// Create Arthur's primary email
    pub fn arthur_primary(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("arthur@example.com")
            .is_primary(true)
            .is_verified(true)
    }
    
    /// Create Arthur's secondary email
    pub fn arthur_secondary(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("arthur.secondary@example.com")
            .is_primary(false)
            .is_verified(false)
    }
    
    /// Create Arthur's GitHub email
    pub fn arthur_github(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("arthur@github.example.com")
            .is_primary(false)
            .is_verified(false)
    }
    
    /// Create Bob's primary email
    pub fn bob_primary(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("bob@example.com")
            .is_primary(true)
            .is_verified(true)
    }
    
    /// Create Alice's primary email
    pub fn alice_primary(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("alice@example.com")
            .is_primary(true)
            .is_verified(true)
    }
    
    /// Create Alice's GitLab email
    pub fn alice_gitlab(self, user_id: Uuid) -> Self {
        self.user_id(user_id)
            .email("alice@gitlab.example.com")
            .is_primary(false)
            .is_verified(false)
    }
}

impl Default for UserEmailFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<UserEmailsEntity, UserEmailModel, UserEmailActiveModel> for UserEmailFixtureBuilder {
    async fn commit(self, db: &DatabaseConnection) -> Result<CommittedFixture<UserEmailModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }
    
    fn model(&self) -> UserEmailActiveModel {
        let now = TestData::now_naive();
        
        UserEmailActiveModel {
            id: ActiveValue::Set(self.id.unwrap_or_else(TestData::uuid)),
            user_id: ActiveValue::Set(self.user_id.expect("user_id is required")),
            email: ActiveValue::Set(self.email.clone().unwrap_or_else(TestData::email)),
            is_primary: ActiveValue::Set(self.is_primary.unwrap_or(false)),
            is_verified: ActiveValue::Set(self.is_verified.unwrap_or(false)),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
            updated_at: ActiveValue::Set(self.updated_at.unwrap_or(now)),
        }
    }
}

/// Committed user email fixture
#[derive(Debug, Clone)]
pub struct UserEmailFixture {
    inner: CommittedFixture<UserEmailModel>,
}

impl UserEmailFixture {
    /// Check if the user email exists in the database and matches the fixture
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;
        
        // Find the user email in the database by ID
        let db_email = UserEmailsEntity::find_by_id(self.model().id)
            .one(&*db)
            .await?;
        
        match db_email {
            Some(email) => {
                // Compare all fields
                Ok(email.id == self.model().id
                    && email.user_id == self.model().user_id
                    && email.email == self.model().email
                    && email.is_primary == self.model().is_primary
                    && email.is_verified == self.model().is_verified
                    && email.created_at == self.model().created_at
                    && email.updated_at == self.model().updated_at)
            }
            None => Ok(false), // Email doesn't exist in database
        }
    }
    
    /// Get the user email model
    pub fn model(&self) -> &UserEmailModel {
        self.inner.model()
    }
    
    /// Get the email ID
    pub fn id(&self) -> Uuid {
        self.model().id
    }
    
    /// Get the user ID
    pub fn user_id(&self) -> Uuid {
        self.model().user_id
    }
    
    /// Get the email address
    pub fn email(&self) -> &str {
        &self.model().email
    }
    
    /// Check if this is the primary email
    pub fn is_primary(&self) -> bool {
        self.model().is_primary
    }
    
    /// Check if this email is verified
    pub fn is_verified(&self) -> bool {
        self.model().is_verified
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