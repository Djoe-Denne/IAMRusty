pub mod common;
pub mod notification_deliveries;
pub mod notifications;

use rustycog_testing::db::TestData;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use telegraphmigration::{Migrator, MigratorTrait};

pub use common::*;
pub use notification_deliveries::*;
pub use notifications::*;

/// Database fixtures for Telegraph
pub struct DbFixtures;

impl DbFixtures {
    /// Create a notification fixture builder
    pub fn notification() -> NotificationFixtureBuilder {
        NotificationFixtureBuilder::new()
    }

    /// Create a notification delivery fixture builder
    pub fn notification_delivery() -> NotificationDeliveryFixtureBuilder {
        NotificationDeliveryFixtureBuilder::new()
    }

    /// Helper method to create a notification with delivery tracking
    pub async fn create_notification_with_delivery(
        db: DatabaseConnection,
        user_id: uuid::Uuid,
        delivery_method: &str,
    ) -> anyhow::Result<(
        telegraph_infra::repository::entity::notifications::Model,
        telegraph_infra::repository::entity::notification_deliveries::Model,
    )> {
        let notification = Self::notification()
            .user_id(user_id)
            .title("Test Notification".to_string())
            .commit(&db)
            .await?;

        let delivery = Self::notification_delivery()
            .notification_id(notification.id())
            .delivery_method(delivery_method.to_string())
            .commit(&db)
            .await?;

        Ok((notification.model().clone(), delivery.model().clone()))
    }

    /// Helper method to create a read notification
    pub async fn create_read_notification(
        db: DatabaseConnection,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<telegraph_infra::repository::entity::notifications::Model> {
        let notification = Self::notification()
            .user_id(user_id)
            .title("Read Notification".to_string())
            .is_read(true)
            .read_at(Some(chrono::Utc::now()))
            .commit(&db)
            .await?;

        Ok(notification.model().clone())
    }

    /// Helper method to create an expired notification
    pub async fn create_expired_notification(
        db: DatabaseConnection,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<telegraph_infra::repository::entity::notifications::Model> {
        let expired_date = TestData::now() - chrono::Duration::hours(1);
        let notification = Self::notification()
            .user_id(user_id)
            .title("Expired Notification".to_string())
            .expires_at(Some(expired_date))
            .commit(&db)
            .await?;

        Ok(notification.model().clone())
    }

    /// Clean up function to truncate all tables between tests
    pub async fn cleanup(db: &DatabaseConnection) -> anyhow::Result<()> {
        use sea_orm::*;

        // Truncate in reverse order due to foreign key constraints
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "TRUNCATE TABLE notification_deliveries CASCADE".to_owned(),
        ))
        .await?;

        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "TRUNCATE TABLE notifications CASCADE".to_owned(),
        ))
        .await?;

        Ok(())
    }
}
