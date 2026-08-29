//! Telegraph domain events.
//!
//! `NotificationCreated` is the AuthZ-relevant event consumed by
//! `sentinel-sync` to write `notification:{id}#recipient@user:{user_id}`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog::core::error::ServiceError;
use rustycog::events::{BaseEvent, DomainEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum TelegraphDomainEvent {
    #[serde(rename = "notification_created")]
    NotificationCreated(NotificationCreatedEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl NotificationCreatedEvent {
    #[must_use]
    pub fn new(notification_id: Uuid, user_id: Uuid, created_at: DateTime<Utc>) -> Self {
        Self {
            base: BaseEvent::new("notification_created".to_string(), notification_id),
            notification_id,
            user_id,
            created_at,
        }
    }
}

impl DomainEvent for TelegraphDomainEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::NotificationCreated(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            Self::NotificationCreated(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::NotificationCreated(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::NotificationCreated(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::NotificationCreated(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            Self::NotificationCreated(event) => event.base.metadata.clone(),
        }
    }
}

impl From<TelegraphDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: TelegraphDomainEvent) -> Self {
        Box::new(event)
    }
}
