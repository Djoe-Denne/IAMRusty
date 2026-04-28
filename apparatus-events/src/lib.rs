//! # Apparatus Events
//!
//! Shared domain events for apparatus component services.
//! This crate provides common event types that can be published by the Manifesto service
//! and consumed by component services.

pub mod component;

// Re-export for convenience
pub use component::*;

// Re-export rustycog-events for consumers
pub use rustycog_events::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog_core::error::ServiceError;
use rustycog_events::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum ApparatusDomainEvent {
    #[serde(rename = "component_status_changed")]
    ComponentStatusChanged(ComponentStatusChangedEvent),
}

impl DomainEvent for ApparatusDomainEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::ComponentStatusChanged(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            Self::ComponentStatusChanged(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::ComponentStatusChanged(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::ComponentStatusChanged(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::ComponentStatusChanged(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            Self::ComponentStatusChanged(event) => event.base.metadata.clone(),
        }
    }
}

impl From<ApparatusDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: ApparatusDomainEvent) -> Self {
        Box::new(event)
    }
}
