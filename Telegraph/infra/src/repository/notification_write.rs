use anyhow::Result;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use std::sync::Arc;
use uuid::Uuid;

use tracing::debug;

use crate::repository::entity::{notification_deliveries, notifications};
use crate::repository::mappers;
use telegraph_domain::entity::{
    communication::{CommunicationMode, NotificationCommunication},
    delivery::MessageDelivery,
};
use telegraph_domain::error::DomainError;
use telegraph_domain::port::repository::NotificationWriteRepository;

/// Repository for writing notifications
#[derive(Clone)]
pub struct NotificationWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl NotificationWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl NotificationWriteRepository for NotificationWriteRepositoryImpl {
    /// Create a new notification
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError> {
        debug!(
            "Creating notification for user: {:?}",
            notification.recipient.user_id
        );

        let active_model = mappers::to_infra_notification(notification)?;

        let result = notifications::Entity::insert(active_model)
            .exec_with_returning(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!("Failed to create notification: {}", e))
            })?;

        mappers::to_domain_notification(result)
    }

    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError> {
        debug!(
            "Creating notification and delivery for user: {:?}",
            notification.recipient.user_id
        );

        let txn = self.db.begin().await.map_err(|e| {
            DomainError::infrastructure_error(format!(
                "Failed to begin notification transaction: {}",
                e
            ))
        })?;

        let result = async {
            let notification_model = mappers::to_infra_notification(notification)?;
            let notification_row = notifications::Entity::insert(notification_model)
                .exec_with_returning(&txn)
                .await
                .map_err(|e| {
                    DomainError::infrastructure_error(format!(
                        "Failed to create notification: {}",
                        e
                    ))
                })?;

            let notification = mappers::to_domain_notification(notification_row)?;
            let delivery = MessageDelivery::new(notification.id.unwrap(), delivery_mode);
            let delivery_model = mappers::to_infra_delivery(delivery);
            let delivery_row = notification_deliveries::Entity::insert(delivery_model)
                .exec_with_returning(&txn)
                .await
                .map_err(|e| {
                    DomainError::infrastructure_error(format!(
                        "Failed to create delivery record: {}",
                        e
                    ))
                })?;
            let delivery = mappers::to_domain_delivery(delivery_row)?;

            Ok::<_, DomainError>((notification, delivery))
        }
        .await;

        match result {
            Ok(created) => {
                txn.commit().await.map_err(|e| {
                    DomainError::infrastructure_error(format!(
                        "Failed to commit notification transaction: {}",
                        e
                    ))
                })?;
                Ok(created)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback notification transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }

    /// Mark notification as read
    async fn mark_as_read(
        &self,
        notification_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError> {
        debug!("Marking notification as read: {}", notification_id);

        let now = Utc::now();

        let result = notifications::Entity::update_many()
            .col_expr(notifications::Column::IsRead, Expr::value(true))
            .col_expr(notifications::Column::ReadAt, Expr::value(now))
            .col_expr(notifications::Column::UpdatedAt, Expr::value(now))
            .filter(notifications::Column::Id.eq(notification_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to mark notification as read: {}",
                    e
                ))
            })?;

        if result.rows_affected == 0 {
            return Err(DomainError::notification_not_found(format!(
                "Notification not found: {}",
                notification_id
            )));
        }

        // Fetch the updated notification
        let updated_notification = notifications::Entity::find_by_id(notification_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to fetch updated notification: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                DomainError::notification_not_found(format!(
                    "Notification not found after update: {}",
                    notification_id
                ))
            })?;

        mappers::to_domain_notification(updated_notification)
    }

    /// Delete expired notifications
    async fn delete_expired_notifications(&self) -> Result<u64, DomainError> {
        debug!("Deleting expired notifications");

        let result = notifications::Entity::delete_many()
            .filter(notifications::Column::ExpiresAt.lt(Utc::now()))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to delete expired notifications: {}",
                    e
                ))
            })?;

        Ok(result.rows_affected)
    }

    /// Create a delivery record for a notification
    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError> {
        debug!(
            "Creating delivery record for message: {}",
            delivery.message_id
        );

        let active_model = mappers::to_infra_delivery(delivery);

        let result = notification_deliveries::Entity::insert(active_model)
            .exec_with_returning(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to create delivery record: {}",
                    e
                ))
            })?;

        mappers::to_domain_delivery(result)
    }

    /// Update delivery status
    async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<MessageDelivery, DomainError> {
        debug!(
            "Updating delivery status for delivery: {} to status: {}",
            delivery_id, status
        );

        let now = Utc::now();

        let mut update = notification_deliveries::Entity::update_many()
            .col_expr(
                notification_deliveries::Column::Status,
                Expr::value(status.clone()),
            )
            .col_expr(notification_deliveries::Column::UpdatedAt, Expr::value(now))
            .col_expr(
                notification_deliveries::Column::ErrorMessage,
                Expr::value(error_message),
            )
            .filter(notification_deliveries::Column::Id.eq(delivery_id));

        // If status is delivered, set delivered_at timestamp
        if status == "delivered" {
            update = update.col_expr(
                notification_deliveries::Column::DeliveredAt,
                Expr::value(now),
            );
        }

        let result = update.exec(self.db.as_ref()).await.map_err(|e| {
            DomainError::infrastructure_error(format!("Failed to update delivery status: {}", e))
        })?;

        if result.rows_affected == 0 {
            return Err(DomainError::notification_not_found(format!(
                "Delivery record not found: {}",
                delivery_id
            )));
        }

        // Fetch the updated delivery
        let updated_delivery = notification_deliveries::Entity::find_by_id(delivery_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to fetch updated delivery: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                DomainError::notification_not_found(format!(
                    "Delivery record not found after update: {}",
                    delivery_id
                ))
            })?;

        mappers::to_domain_delivery(updated_delivery)
    }

    /// Increment delivery attempt count
    async fn increment_delivery_attempt(
        &self,
        delivery_id: Uuid,
    ) -> Result<MessageDelivery, DomainError> {
        debug!(
            "Incrementing delivery attempt for delivery: {}",
            delivery_id
        );

        let now = Utc::now();

        let result = notification_deliveries::Entity::update_many()
            .col_expr(
                notification_deliveries::Column::AttemptCount,
                Expr::col(notification_deliveries::Column::AttemptCount).add(1),
            )
            .col_expr(
                notification_deliveries::Column::LastAttemptAt,
                Expr::value(now),
            )
            .col_expr(notification_deliveries::Column::UpdatedAt, Expr::value(now))
            .filter(notification_deliveries::Column::Id.eq(delivery_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to increment delivery attempt: {}",
                    e
                ))
            })?;

        if result.rows_affected == 0 {
            return Err(DomainError::notification_not_found(format!(
                "Delivery record not found: {}",
                delivery_id
            )));
        }

        // Fetch the updated delivery
        let updated_delivery = notification_deliveries::Entity::find_by_id(delivery_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                DomainError::infrastructure_error(format!(
                    "Failed to fetch updated delivery: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                DomainError::notification_not_found(format!(
                    "Delivery record not found after update: {}",
                    delivery_id
                ))
            })?;

        mappers::to_domain_delivery(updated_delivery)
    }
}
