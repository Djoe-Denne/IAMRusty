use sea_orm::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use serde_json::json;

use telegraph_infra::repository::entity::notifications;
use super::common::{FixtureBuilder, FixtureFactory};

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

    /// Set the user ID
    pub fn user_id(mut self, user_id: Uuid) -> Self {
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
    pub fn is_read(mut self, is_read: bool) -> Self {
        self.is_read = is_read;
        self
    }

    /// Set the priority (1=high, 2=medium, 3=normal, 4=low)
    pub fn priority(mut self, priority: i16) -> Self {
        self.priority = priority;
        self
    }

    /// Set expiration date
    pub fn expires_at(mut self, expires_at: Option<DateTime<Utc>>) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Set read timestamp
    pub fn read_at(mut self, read_at: Option<DateTime<Utc>>) -> Self {
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
impl FixtureBuilder<notifications::Model> for NotificationFixtureBuilder {
    async fn commit(self, db: DatabaseConnection) -> anyhow::Result<notifications::Model> {
        let notification = notifications::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(self.user_id),
            title: Set(self.title),
            content: Set(self.content),
            content_type: Set(self.content_type),
            is_read: Set(self.is_read),
            priority: Set(self.priority),
            expires_at: Set(self.expires_at.map(|dt| dt.naive_utc())),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            read_at: Set(self.read_at.map(|dt| dt.naive_utc())),
        };

        let result = notifications::Entity::insert(notification)
            .exec_with_returning(&db)
            .await?;

        Ok(result)
    }

    async fn check(&self, db: &DatabaseConnection, entity: &notifications::Model) -> anyhow::Result<bool> {
        let found = notifications::Entity::find_by_id(entity.id)
            .one(db)
            .await?;

        if let Some(found) = found {
            Ok(found.user_id == self.user_id
                && found.title == self.title
                && found.content == self.content
                && found.is_read == self.is_read
                && found.priority == self.priority)
        } else {
            Ok(false)
        }
    }
}

impl FixtureFactory<NotificationFixtureBuilder> for NotificationFixtureBuilder {
    fn default() -> NotificationFixtureBuilder {
        NotificationFixtureBuilder::new()
    }
} 