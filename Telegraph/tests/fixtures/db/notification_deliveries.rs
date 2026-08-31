use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::*;
use std::sync::Arc;
use uuid::Uuid;

use rustycog::testing::db::{CommittedFixture, DbFixture, TestData};
use telegraph_infra::repository::entity::notification_deliveries;

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

impl Default for NotificationDeliveryFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationDeliveryFixtureBuilder {
    #[must_use]
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

    /// Persist this builder as a committed delivery fixture.
    ///
    /// # Errors
    ///
    /// Returns a [`DbErr`] if the insert fails.
    pub async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<NotificationDeliveryFixture, DbErr> {
        let fixture = DbFixture::commit(self, db).await?;
        Ok(NotificationDeliveryFixture { inner: fixture })
    }

    /// Set the notification ID
    #[must_use]
    pub const fn notification_id(mut self, notification_id: Uuid) -> Self {
        self.notification_id = notification_id;
        self
    }

    /// Set the delivery method
    #[must_use]
    pub fn delivery_method(mut self, delivery_method: String) -> Self {
        self.delivery_method = delivery_method;
        self
    }

    /// Set the status
    #[must_use]
    pub fn status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Set the attempt count
    #[must_use]
    pub const fn attempt_count(mut self, attempt_count: i16) -> Self {
        self.attempt_count = attempt_count;
        self
    }

    /// Set the last attempt timestamp
    #[must_use]
    pub const fn last_attempt_at(mut self, last_attempt_at: Option<DateTime<Utc>>) -> Self {
        self.last_attempt_at = last_attempt_at;
        self
    }

    /// Set the delivered timestamp
    #[must_use]
    pub const fn delivered_at(mut self, delivered_at: Option<DateTime<Utc>>) -> Self {
        self.delivered_at = delivered_at;
        self
    }

    /// Set the error message
    #[must_use]
    pub fn error_message(mut self, error_message: Option<String>) -> Self {
        self.error_message = error_message;
        self
    }

    // Factory methods for common scenarios

    /// Create an email delivery
    #[must_use]
    pub fn email(mut self) -> Self {
        self.delivery_method = "email".to_string();
        self
    }

    /// Create an SMS delivery
    #[must_use]
    pub fn sms(mut self) -> Self {
        self.delivery_method = "sms".to_string();
        self
    }

    /// Create a push notification delivery
    #[must_use]
    pub fn push(mut self) -> Self {
        self.delivery_method = "push".to_string();
        self
    }

    /// Create an in-app notification delivery
    #[must_use]
    pub fn in_app(mut self) -> Self {
        self.delivery_method = "in_app".to_string();
        self
    }

    /// Create a pending delivery
    #[must_use]
    pub fn pending(mut self) -> Self {
        self.status = "pending".to_string();
        self.attempt_count = 0;
        self
    }

    /// Create a sent delivery
    #[must_use]
    pub fn sent(mut self) -> Self {
        self.status = "sent".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(5));
        self
    }

    /// Create a delivered delivery
    #[must_use]
    pub fn delivered(mut self) -> Self {
        self.status = "delivered".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(5));
        self.delivered_at = Some(Utc::now() - chrono::Duration::minutes(3));
        self
    }

    /// Create a failed delivery
    #[must_use]
    pub fn failed(mut self) -> Self {
        self.status = "failed".to_string();
        self.attempt_count = 3;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(1));
        self.error_message = Some("Delivery failed after 3 attempts".to_string());
        self
    }

    /// Create a bounced delivery
    #[must_use]
    pub fn bounced(mut self) -> Self {
        self.status = "bounced".to_string();
        self.attempt_count = 1;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(2));
        self.error_message = Some("Email bounced - invalid address".to_string());
        self
    }

    /// Create a delivery with retry attempts
    #[must_use]
    pub fn retried(mut self, attempts: i16) -> Self {
        self.status = if attempts >= 3 {
            "failed".to_string()
        } else {
            "pending".to_string()
        };
        self.attempt_count = attempts;
        self.last_attempt_at = Some(Utc::now() - chrono::Duration::minutes(1));
        if attempts >= 3 {
            self.error_message = Some(format!("Failed after {attempts} attempts"));
        }
        self
    }
}

#[async_trait]
impl
    DbFixture<
        notification_deliveries::Entity,
        notification_deliveries::Model,
        notification_deliveries::ActiveModel,
    > for NotificationDeliveryFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<notification_deliveries::Model>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> notification_deliveries::ActiveModel {
        let now = TestData::now();

        notification_deliveries::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            notification_id: ActiveValue::Set(self.notification_id),
            delivery_method: ActiveValue::Set(self.delivery_method.clone()),
            status: ActiveValue::Set(self.status.clone()),
            attempt_count: ActiveValue::Set(self.attempt_count),
            last_attempt_at: ActiveValue::Set(self.last_attempt_at),
            delivered_at: ActiveValue::Set(self.delivered_at),
            error_message: ActiveValue::Set(self.error_message.clone()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationDeliveryFixture {
    inner: CommittedFixture<notification_deliveries::Model>,
}

impl NotificationDeliveryFixture {
    /// Returns whether this fixture still exists in the database.
    ///
    /// # Errors
    ///
    /// Returns a [`DbErr`] if the lookup query fails.
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;
        let found = notification_deliveries::Entity::find_by_id(self.id())
            .one(&*db)
            .await?;

        Ok(found.is_some())
    }

    #[must_use]
    pub const fn model(&self) -> &notification_deliveries::Model {
        self.inner.model()
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.model().id
    }

    #[must_use]
    pub const fn notification_id(&self) -> Uuid {
        self.model().notification_id
    }

    #[must_use]
    pub const fn delivery_method(&self) -> &String {
        &self.model().delivery_method
    }

    #[must_use]
    pub const fn status(&self) -> &String {
        &self.model().status
    }

    #[must_use]
    pub const fn attempt_count(&self) -> i16 {
        self.model().attempt_count
    }

    #[must_use]
    pub const fn last_attempt_at(&self) -> Option<DateTime<Utc>> {
        self.model().last_attempt_at
    }

    #[must_use]
    pub const fn delivered_at(&self) -> Option<DateTime<Utc>> {
        self.model().delivered_at
    }

    #[must_use]
    pub const fn error_message(&self) -> Option<&String> {
        self.model().error_message.as_ref()
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.model().created_at
    }

    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.model().updated_at
    }
}
