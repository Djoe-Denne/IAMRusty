//! Hive Domain Events
//! 
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::{DomainEvent, BaseEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub permission: String,
    pub resource: String,
}

impl Role {
    pub fn new(permission: String, resource: String) -> Self {
        Self { permission, resource }
    }
}
