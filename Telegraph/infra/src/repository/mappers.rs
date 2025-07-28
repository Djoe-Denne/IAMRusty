//! Mappers for converting between domain and infrastructure entities

use sea_orm::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::repository::entity::{notifications, notification_deliveries};
use telegraph_domain::error::DomainError;
use telegraph_domain::entity::{
    communication::{NotificationCommunication, CommunicationRecipient}, delivery::MessageDelivery,
    communication::CommunicationMode, DeliveryStatus
};

/// Convert a database notification model to a domain model
pub fn to_domain_notification(model: notifications::Model) -> Result<NotificationCommunication, DomainError> {
    // Deserialize content from JSON bytes
    let content_str = String::from_utf8(model.content)
        .map_err(|e| DomainError::infrastructure_error(format!("Invalid UTF-8 content: {}", e)))?;
    
    // Create recipient - for notifications we primarily use user_id
    let recipient = CommunicationRecipient {
        user_id: Some(model.user_id),
        email: None,
        };

    Ok(NotificationCommunication {
        recipient,
        id: Some(model.id),
        title: model.title,
        body: content_str,
        data: HashMap::new(),
        is_read: Some(model.is_read),
        created_at: Some(model.created_at),
        updated_at: Some(model.updated_at),
        read_at: model.read_at,
    })
}

/// Convert a database delivery model to a domain model
pub fn to_domain_delivery(model: notification_deliveries::Model) -> Result<MessageDelivery, DomainError> {
    // Map delivery_method string to CommunicationMode
    let mode = CommunicationMode::Notification;

    // Map status string to DeliveryStatus
    let status = match model.status.as_str() {
        "pending" => DeliveryStatus::Pending,
        "processing" => DeliveryStatus::Processing,
        "sent" => DeliveryStatus::Sent,
        "delivered" => DeliveryStatus::Delivered,
        "failed" => DeliveryStatus::Failed,
        "rejected" => DeliveryStatus::Rejected,
        "bounced" => DeliveryStatus::Bounced,
        "read" => DeliveryStatus::Read,
        _ => return Err(DomainError::infrastructure_error(format!("Unknown delivery status: {}", model.status))),
    };

    Ok(MessageDelivery {
        id: model.id,
        message_id: model.notification_id,
        mode,
        status,
        attempts: model.attempt_count as u32,
        provider_message_id: None, // Could be stored in metadata if needed
        metadata: HashMap::new(),
        created_at: model.created_at,
        updated_at: model.updated_at,
        delivered_at: model.delivered_at,
        error_details: model.error_message,
    })
}

/// Convert a domain notification to an infrastructure active model
pub fn to_infra_notification(notification: NotificationCommunication) -> Result<notifications::ActiveModel, DomainError> {
    // Map priority enum to i16
    let priority = 1;

    // Extract user_id from recipient
    let user_id = notification.recipient.user_id
        .ok_or_else(|| DomainError::invalid_recipient("User ID is required for notifications".to_string()))?;

    // Extract title from content for database storage
    let title = notification.title;

    Ok(notifications::ActiveModel {
        id: ActiveValue::Set(notification.id.unwrap_or_else(|| Uuid::new_v4())),
        user_id: ActiveValue::Set(user_id),
        title: ActiveValue::Set(title),
        content: ActiveValue::Set(notification.body.as_bytes().to_vec()),
        content_type: ActiveValue::Set("application/text".to_string()),
        is_read: ActiveValue::Set(notification.is_read.unwrap_or(false)),
        priority: ActiveValue::Set(priority),
        expires_at: ActiveValue::NotSet, // Could be calculated based on priority or content
        created_at: ActiveValue::Set(notification.created_at.unwrap_or_else(|| Utc::now())),
        updated_at: ActiveValue::Set(notification.updated_at.unwrap_or_else(|| Utc::now())),
        read_at: ActiveValue::Set(notification.read_at),
    })
}

/// Convert a domain delivery to an infrastructure active model
pub fn to_infra_delivery(delivery: MessageDelivery) -> notification_deliveries::ActiveModel {
    // Map DeliveryStatus to status string
    let status = delivery.status.to_string();

    notification_deliveries::ActiveModel {
        id: ActiveValue::Set(delivery.id),
        notification_id: ActiveValue::Set(delivery.message_id),
        delivery_method: ActiveValue::Set("notification".to_string()),
        status: ActiveValue::Set(status),
        attempt_count: ActiveValue::Set(delivery.attempts as i16),
        last_attempt_at: ActiveValue::NotSet,
        delivered_at: ActiveValue::Set(delivery.delivered_at),
        error_message: ActiveValue::Set(delivery.error_details),
        created_at: ActiveValue::Set(delivery.created_at),
        updated_at: ActiveValue::Set(delivery.updated_at),
    }
} 