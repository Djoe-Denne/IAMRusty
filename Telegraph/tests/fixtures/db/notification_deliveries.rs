use sea_orm::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;

use telegraph_infra::repository::entity::notification_deliveries;
use super::common::{FixtureBuilder, FixtureFactory};

/// Builder for notification delivery fixtures
#[derive(Clone, Debug)]
pub struct NotificationDeliveryFixtureBuilder {
    notification_id: Uuid,
    delivery_method: String,
    status: String,
    attempt_count: i16,
    last_attempt_at: Option<DateTime<Utc>>,
    delivered_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

impl NotificationDeliveryFixtureBuilder {
    pub fn new() -> Self {
        Self {
            notification_id: Uuid::new_v4(),
            delivery_method: "email".to_string(),
            status: "pending".to_string(),
            attempt_count: 0,
            last_attempt_at: None,
            delivered_at: None,
            error_message: None,
        }
    }

    /// Set the notification ID
    pub fn notification_id(mut self, notification_id: Uuid) -> Self {
        self.notification_id = notification_id;
        self
    }

    /// Set the delivery method
    pub fn delivery_method(mut self, delivery_method: String) -> Self {
        self.delivery_method = delivery_method;
        self
    }

    /// Set the status
    pub fn status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Set the attempt count
    pub fn attempt_count(mut self, attempt_count: i16) -> Self {
        self.attempt_count = attempt_count;
        self
    }

    /// Set the last attempt timestamp
    pub fn last_attempt_at(mut self, last_attempt_at: Option<DateTime<Utc>>) -> Self {
        self.last_attempt_at = last_attempt_at;
        self
    }

    /// Set the delivered timestamp
    pub fn delivered_at(mut self, delivered_at: Option<DateTime<Utc>>) -> Self {
        self.delivered_at = delivered_at;
        self
    }

    /// Set the error message
    pub fn error_message(mut self, error_message: Option<String>) -> Self {
        self.error_message = error_message;
        self
    }

    // Factory methods for common scenarios

    /// Create an email delivery
    pub fn email(mut self) -> Self {
        self.delivery_method = "email".to_string();
        self
    }

    /// Create an SMS delivery
    pub fn sms(mut self) -> Self {
        self.delivery_method = "sms".to_string();
        self
    }

    /// Create a push notification delivery
    pub fn push(mut self) -> Self {
        self.delivery_method = "push".to_string();
        self
    }

    /// Create an in-app notification delivery
    pub fn in_app(mut self) -> Self {
        self.delivery_method = "in_app".to_string();
        self
    }

    /// Create a pending delivery
    pub fn pending(mut self) -> Self {
        self.status = "pending".to_string();
        self.attempt_count = 0;
        self
    }

    /// Create a sent delivery
    pub fn sent(mut self) -> Self {
        self.status = "sent".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(5));
        self
    }

    /// Create a delivered delivery
    pub fn delivered(mut self) -> Self {
        self.status = "delivered".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(5));
        self.delivered_at = Some(Utc::now() - chrono::Duration::minutes(3));
        self
    }

    /// Create a failed delivery
    pub fn failed(mut self) -> Self {
        self.status = "failed".to_string();
        self.attempt_count = 3;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(1));
        self.error_message = Some("Delivery failed after 3 attempts".to_string());
        self
    }

    /// Create a bounced delivery
    pub fn bounced(mut self) -> Self {
        self.status = "bounced".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(2));
        self.error_message = Some("Email bounced - invalid address".to_string());
        self
    }

    /// Create a delivery with retry attempts
    pub fn retried(mut self, attempts: i16) -> Self {
        self.status = if attempts >= 3 { "failed".to_string() } else { "pending".to_string() };
        self.attempt_count = attempts;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(1));
        if attempts >= 3 {
            self.error_message = Some(format!("Failed after {} attempts", attempts));
        }
        self
    }
}

#[async_trait]
impl FixtureBuilder<notification_deliveries::Model> for NotificationDeliveryFixtureBuilder {
    async fn commit(self, db: DatabaseConnection) -> anyhow::Result<notification_deliveries::Model> {
        let delivery = notification_deliveries::ActiveModel {
            id: Set(Uuid::new_v4()),
            notification_id: Set(self.notification_id),
            delivery_method: Set(self.delivery_method),
            status: Set(self.status),
            attempt_count: Set(self.attempt_count),
            last_attempt_at: Set(self.last_attempt_at),
            delivered_at: Set(self.delivered_at),
            error_message: Set(self.error_message),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = notification_deliveries::Entity::insert(delivery)
            .exec_with_returning(&db)
            .await?;

        Ok(result)
    }

    async fn check(&self, db: &DatabaseConnection, entity: &notification_deliveries::Model) -> anyhow::Result<bool> {
        let found = notification_deliveries::Entity::find_by_id(entity.id)
            .one(db)
            .await?;

        if let Some(found) = found {
            Ok(found.notification_id == self.notification_id
                && found.delivery_method == self.delivery_method
                && found.status == self.status
                && found.attempt_count == self.attempt_count)
        } else {
            Ok(false)
        }
    }
}

impl FixtureFactory<NotificationDeliveryFixtureBuilder> for NotificationDeliveryFixtureBuilder {
    fn default() -> NotificationDeliveryFixtureBuilder {
        NotificationDeliveryFixtureBuilder::new()
    }
} 