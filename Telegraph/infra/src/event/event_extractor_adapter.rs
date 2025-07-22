//! Event extractor adapter implementation using JSON utilities

use async_trait::async_trait;
use std::collections::HashMap;
use rustycog_events::DomainEvent;
use telegraph_domain::{DomainError, EventExtractor};
use crate::event::json_utils::json_to_string_map;
use serde_json;

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
    async fn extract_variables(&self, event: &dyn DomainEvent) -> Result<HashMap<String, String>, DomainError> {
        // Serialize the domain event to JSON
        let event_json = serde_json::from_str(&event.to_json()
            .map_err(|e| DomainError::EventProcessingError(format!("Failed to serialize event to JSON: {}", e)))?)
            .map_err(|e| DomainError::EventProcessingError(format!("Failed to serialize event to JSON: {}", e)))?;

        // Convert JSON to HashMap<String, String> for template variables
        json_to_string_map(&event_json)
    }
} 