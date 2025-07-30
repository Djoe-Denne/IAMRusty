//! Event extractor adapter implementation using JSON utilities

use crate::event::json_utils::json_to_string_map;
use async_trait::async_trait;
use rustycog_events::DomainEvent;
use serde_json;
use std::collections::HashMap;
use telegraph_domain::{DomainError, EventExtractor};
use tracing::debug;

/// Event extractor adapter that uses JSON utilities to extract template variables
pub struct JsonEventExtractor;

impl JsonEventExtractor {
    /// Create a new JSON event extractor
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventExtractor for JsonEventExtractor {
    /// Extract template variables from a domain event
    async fn extract_variables(
        &self,
        event: &dyn DomainEvent,
    ) -> Result<HashMap<String, String>, DomainError> {
        // Serialize the domain event to JSON
        let event_json = event.to_json().map_err(|e| {
            DomainError::EventProcessingError(format!("Failed to serialize event to JSON: {}", e))
        })?;
        let event_json: serde_json::Value = serde_json::from_str(&event_json).map_err(|e| {
            DomainError::EventProcessingError(format!("Failed to parse event to JSON: {}", e))
        })?;

        debug!("Event JSON: {}", event_json);
        let data = event_json
            .get("data")
            .ok_or(DomainError::EventProcessingError(
                "Event data not found".to_string(),
            ))?;
        // Convert JSON to HashMap<String, String> for template variables
        json_to_string_map(data)
    }
}
