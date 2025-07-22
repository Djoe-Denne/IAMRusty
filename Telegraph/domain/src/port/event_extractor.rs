//! Event extractor port for extracting template variables from events

use async_trait::async_trait;
use std::collections::HashMap;
use rustycog_events::DomainEvent;
use crate::error::DomainError;

/// Port for extracting template variables from domain events
#[async_trait]
pub trait EventExtractor: Send + Sync {
    /// Extract template variables from a domain event
    /// Converts the event into a flat HashMap<String, String> suitable for template rendering
    async fn extract_variables(&self, event: &dyn DomainEvent) -> Result<HashMap<String, String>, DomainError>;
} 