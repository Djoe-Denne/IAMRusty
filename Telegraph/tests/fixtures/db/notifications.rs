use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::*;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use super::common::FixtureFactory;
use rustycog_testing::db::{CommittedFixture, DbFixture, TestData};
use telegraph_infra::repository::entity::notifications;

/// Builder for notification fixtures
#[derive(Clone, Debug)]
pub struct NotificationFixtureBuilder {
    user_id: Uuid,
    title: String,
    content: Vec<u8>,
    content_type: String,
    is_read: bool,
    priority: i16,
    expires_at: Option<DateTime<Utc>>,
    read_at: Option<DateTime<Utc>>,
}

impl NotificationFixtureBuilder {
    pub fn new() -> Self {
        let default_content = json!({
            "message": "Default notification content",
            "action": "none"
        });

        Self {
            user_id: Uuid::new_v4(),
            title: "Test Notification".to_string(),
            content: default_content.to_string().into_bytes(),
            content_type: "application/json".to_string(),
            is_read: false,
            priority: 3,
            expires_at: None,
            read_at: None,
        }
    }

    pub async fn commit(self, db: &DatabaseConnection) -> Result<NotificationFixture, DbErr> {
        let fixture = DbFixture::commit(self, db).await?;
        Ok(NotificationFixture { inner: fixture })
    }

    /// Set the user ID
    pub const fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = user_id;
        self
    }

    /// Set the title
    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    /// Set the content as JSON
    pub fn content_json(mut self, content: serde_json::Value) -> Self {
        self.content = content.to_string().into_bytes();
        self.content_type = "application/json".to_string();
        self
    }

    /// Set the content as raw bytes
    pub fn content_bytes(mut self, content: Vec<u8>, content_type: String) -> Self {
        self.content = content;
        self.content_type = content_type;
        self
    }

    /// Set whether the notification is read
    pub const fn is_read(mut self, is_read: bool) -> Self {
        self.is_read = is_read;
        self
    }

    /// Set the priority (1=high, 2=medium, 3=normal, 4=low)
    pub const fn priority(mut self, priority: i16) -> Self {
        self.priority = priority;
        self
    }

    /// Set expiration date
    pub const fn expires_at(mut self, expires_at: Option<DateTime<Utc>>) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Set read timestamp
    pub const fn read_at(mut self, read_at: Option<DateTime<Utc>>) -> Self {
        self.read_at = read_at;
        self
    }

    // Factory methods for common scenarios

    /// Create a high priority notification
    pub fn high_priority(mut self) -> Self {
        self.priority = 1;
        self.title = "High Priority Notification".to_string();
        self
    }

    /// Create a low priority notification
    pub fn low_priority(mut self) -> Self {
        self.priority = 4;
        self.title = "Low Priority Notification".to_string();
        self
    }

    /// Create an urgent notification that expires soon
    pub fn urgent(mut self) -> Self {
        self.priority = 1;
        self.title = "Urgent Notification".to_string();
        self.expires_at = Some(Utc::now() + chrono::Duration::hours(1));
        self
    }

    /// Create an already read notification
    pub fn read(mut self) -> Self {
        self.is_read = true;
        self.read_at = Some(Utc::now() - chrono::Duration::minutes(30));
        self
    }

    /// Create an expired notification
    pub fn expired(mut self) -> Self {
        self.title = "Expired Notification".to_string();
        self.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        self
    }

    /// Create a notification for user Alice
    pub fn alice(mut self) -> Self {
        self.user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        self.title = "Notification for Alice".to_string();
        self
    }

    /// Create a notification for user Bob
    pub fn bob(mut self) -> Self {
        self.user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        self.title = "Notification for Bob".to_string();
        self
    }

    /// Create a notification for user Charlie
    pub fn charlie(mut self) -> Self {
        self.user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap();
        self.title = "Notification for Charlie".to_string();
        self
    }
}

#[async_trait]
impl DbFixture<notifications::Entity, notifications::Model, notifications::ActiveModel>
    for NotificationFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<notifications::Model>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> notifications::ActiveModel {
        let now = TestData::now();

        notifications::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(self.user_id),
            title: ActiveValue::Set(self.title.clone()),
            content: ActiveValue::Set(self.content.clone()),
            content_type: ActiveValue::Set(self.content_type.clone()),
            is_read: ActiveValue::Set(self.is_read),
            priority: ActiveValue::Set(self.priority),
            expires_at: ActiveValue::Set(self.expires_at),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            read_at: ActiveValue::Set(self.read_at),
        }
    }
}

impl FixtureFactory<Self> for NotificationFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NotificationFixture {
    inner: CommittedFixture<notifications::Model>,
}

impl NotificationFixture {
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;
        let found = notifications::Entity::find_by_id(self.id())
            .one(&*db)
            .await?;

        Ok(found.is_some())
    }

    pub const fn model(&self) -> &notifications::Model {
        self.inner.model()
    }

    pub const fn id(&self) -> Uuid {
        self.model().id
    }

    pub const fn user_id(&self) -> Uuid {
        self.model().user_id
    }

    pub const fn title(&self) -> &String {
        &self.model().title
    }

    pub const fn content(&self) -> &Vec<u8> {
        &self.model().content
    }

    pub const fn content_type(&self) -> &String {
        &self.model().content_type
    }

    pub const fn is_read(&self) -> bool {
        self.model().is_read
    }

    pub const fn priority(&self) -> i16 {
        self.model().priority
    }

    pub const fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.model().expires_at
    }

    pub const fn read_at(&self) -> Option<DateTime<Utc>> {
        self.model().read_at
    }

    pub const fn created_at(&self) -> DateTime<Utc> {
        self.model().created_at
    }

    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.model().updated_at
    }
}
