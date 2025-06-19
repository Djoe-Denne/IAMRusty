//! Mock event publisher for testing event publishing behavior
//! 
//! This allows tests to verify that events are correctly sent or not sent
//! based on the business logic, without requiring a real message queue.

use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use domain::{port::event_publisher::EventPublisher, entity::events::DomainEvent, error::DomainError};

/// Mock event publisher that captures published events for test verification
#[derive(Debug)]
pub struct MockEventPublisher {
    published_events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl MockEventPublisher {
    /// Create a new mock event publisher
    pub fn new() -> Self {
        Self {
            published_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all published events (for test assertions)
    pub fn get_published_events(&self) -> Vec<DomainEvent> {
        let events = self.published_events.lock().unwrap();
        events.clone()
    }

    /// Get count of published events
    pub fn get_event_count(&self) -> usize {
        let events = self.published_events.lock().unwrap();
        events.len()
    }

    /// Check if any PasswordResetRequested events were published
    pub fn has_password_reset_requested_event(&self) -> bool {
        let events = self.published_events.lock().unwrap();
        events.iter().any(|event| matches!(event, DomainEvent::PasswordResetRequested(_)))
    }

    /// Get all PasswordResetRequested events
    pub fn get_password_reset_requested_events(&self) -> Vec<DomainEvent> {
        let events = self.published_events.lock().unwrap();
        events.iter()
            .filter(|event| matches!(event, DomainEvent::PasswordResetRequested(_)))
            .cloned()
            .collect()
    }

    /// Clear all captured events (useful for test setup)
    pub fn clear_events(&self) {
        let mut events = self.published_events.lock().unwrap();
        events.clear();
    }

    /// Get a shared reference to this publisher (for dependency injection)
    pub fn as_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for MockEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventPublisher for MockEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError> {
        // Capture the event for test verification
        let mut events = self.published_events.lock().unwrap();
        events.push(event);
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        // Capture all events for test verification
        let mut stored_events = self.published_events.lock().unwrap();
        stored_events.extend(events);
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        // Mock is always healthy
        Ok(())
    }
} 